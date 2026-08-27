use std::fs::File;
use std::io::BufWriter;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::{ffi::c_void, ptr};

use audiowaveform_core::{
    AmplitudeScale, Error as CoreError, GenerateOptions, ScaleSpec, Waveform, WaveformFormat,
    generate_waveform_from_path,
};
use magnus::{Error, ExceptionClass, Module, Object, RString, Ruby, function, method, wrap};

#[wrap(class = "AudioWaveform::Waveform", free_immediately)]
struct RubyWaveform(Waveform);

struct NoGvlTask<F, T> {
    function: Option<F>,
    result: Option<T>,
    panicked: bool,
}

#[derive(Clone, Copy)]
enum NoGvlFailure {
    Panicked,
    DidNotRun,
}

unsafe extern "C" fn call_without_gvl<F, T>(data: *mut c_void) -> *mut c_void
where
    F: FnOnce() -> T,
{
    // SAFETY: `data` points to a live `NoGvlTask` for the duration of the
    // synchronous `rb_thread_call_without_gvl` call, and Ruby cannot access it.
    let task = unsafe { &mut *data.cast::<NoGvlTask<F, T>>() };
    let Some(function) = task.function.take() else {
        return ptr::null_mut();
    };
    match catch_unwind(AssertUnwindSafe(function)) {
        Ok(result) => task.result = Some(result),
        Err(_) => task.panicked = true,
    }
    ptr::null_mut()
}

fn without_gvl<F, T>(function: F) -> Result<T, NoGvlFailure>
where
    F: FnOnce() -> T,
{
    let mut task = NoGvlTask {
        function: Some(function),
        result: None,
        panicked: false,
    };
    // SAFETY: the callback only accesses the stack-allocated task during this
    // synchronous call, does not call the Ruby API, and catches Rust panics
    // before returning across the C ABI boundary.
    unsafe {
        rb_sys::rb_thread_call_without_gvl(
            Some(call_without_gvl::<F, T>),
            (&mut task as *mut NoGvlTask<F, T>).cast(),
            None,
            ptr::null_mut(),
        );
    }

    if task.panicked {
        Err(NoGvlFailure::Panicked)
    } else {
        task.result.ok_or(NoGvlFailure::DidNotRun)
    }
}

impl RubyWaveform {
    fn sample_rate(&self) -> u32 {
        self.0.sample_rate()
    }

    fn samples_per_pixel(&self) -> u32 {
        self.0.samples_per_pixel()
    }

    fn channels(&self) -> u16 {
        self.0.channels()
    }

    fn storage_bits(&self) -> u8 {
        self.0.storage_bits()
    }

    fn length(&self) -> usize {
        self.0.len()
    }

    fn empty(&self) -> bool {
        self.0.is_empty()
    }

    fn duration(&self) -> f64 {
        self.0.duration_seconds()
    }

    fn data(&self) -> Vec<i16> {
        self.0.interleaved_samples().to_vec()
    }

    fn point(&self, channel: u16, index: usize) -> Option<(i16, i16)> {
        self.0
            .point(channel, index)
            .map(|point| (point.min, point.max))
    }

    fn save(
        ruby: &Ruby,
        waveform: &Self,
        path: String,
        format: String,
        bits: u8,
    ) -> Result<(), Error> {
        let format = parse_format(ruby, &format)?;
        let result = without_gvl(|| -> Result<(), CoreError> {
            let file = File::create(path)?;
            waveform
                .0
                .write_to_writer(BufWriter::new(file), format, Some(bits))
        })
        .map_err(|failure| no_gvl_error(ruby, failure))?;
        result.map_err(|error| core_error(ruby, error))
    }

    fn serialize(ruby: &Ruby, waveform: &Self, format: String, bits: u8) -> Result<RString, Error> {
        let format = parse_format(ruby, &format)?;
        let result = without_gvl(|| {
            let mut bytes = Vec::new();
            waveform
                .0
                .write_to_writer(&mut bytes, format, Some(bits))
                .map(|()| bytes)
        })
        .map_err(|failure| no_gvl_error(ruby, failure))?;
        let bytes = result.map_err(|error| core_error(ruby, error))?;
        Ok(ruby.str_from_slice(&bytes))
    }
}

fn generate(
    ruby: &Ruby,
    input: String,
    scale_kind: String,
    scale_value: u32,
    split_channels: bool,
    amplitude_kind: String,
    amplitude_value: f64,
) -> Result<RubyWaveform, Error> {
    let scale = match scale_kind.as_str() {
        "samples_per_pixel" => ScaleSpec::SamplesPerPixel(scale_value),
        "pixels_per_second" => ScaleSpec::PixelsPerSecond(scale_value),
        _ => return Err(argument_error(ruby, "unsupported waveform scale")),
    };
    let amplitude_scale = match amplitude_kind.as_str() {
        "none" => None,
        "auto" => Some(AmplitudeScale::Auto),
        "fixed" => Some(AmplitudeScale::Fixed(amplitude_value)),
        _ => return Err(argument_error(ruby, "unsupported amplitude scale")),
    };
    let options = GenerateOptions {
        scale,
        split_channels,
        amplitude_scale,
    };

    without_gvl(|| generate_waveform_from_path(input, &options))
        .map_err(|failure| no_gvl_error(ruby, failure))?
        .map(RubyWaveform)
        .map_err(|error| core_error(ruby, error))
}

fn parse_format(ruby: &Ruby, format: &str) -> Result<WaveformFormat, Error> {
    format
        .parse()
        .map_err(|error: CoreError| core_error(ruby, error))
}

fn core_error(ruby: &Ruby, error: CoreError) -> Error {
    if matches!(error, CoreError::InvalidArgument { .. }) {
        argument_error(ruby, error.to_string())
    } else {
        ruby_error(ruby, error.to_string())
    }
}

fn argument_error(ruby: &Ruby, message: impl AsRef<str>) -> Error {
    Error::new(ruby.exception_arg_error(), message.as_ref().to_owned())
}

fn ruby_error(ruby: &Ruby, message: impl AsRef<str>) -> Error {
    let error_class = ruby
        .eval::<ExceptionClass>("AudioWaveform::Error")
        .unwrap_or_else(|_| ruby.exception_standard_error());
    Error::new(error_class, message.as_ref().to_owned())
}

fn no_gvl_error(ruby: &Ruby, failure: NoGvlFailure) -> Error {
    let message = match failure {
        NoGvlFailure::Panicked => "native waveform operation failed unexpectedly",
        NoGvlFailure::DidNotRun => "native waveform operation did not complete",
    };
    ruby_error(ruby, message)
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("AudioWaveform")?;
    module.define_error("Error", ruby.exception_standard_error())?;

    let native = module.define_module("Native")?;
    native.define_singleton_method("generate", function!(generate, 6))?;

    let waveform = module.define_class("Waveform", ruby.class_object())?;
    waveform.define_method("sample_rate", method!(RubyWaveform::sample_rate, 0))?;
    waveform.define_method(
        "samples_per_pixel",
        method!(RubyWaveform::samples_per_pixel, 0),
    )?;
    waveform.define_method("channels", method!(RubyWaveform::channels, 0))?;
    waveform.define_method("storage_bits", method!(RubyWaveform::storage_bits, 0))?;
    waveform.define_method("length", method!(RubyWaveform::length, 0))?;
    waveform.define_method("empty?", method!(RubyWaveform::empty, 0))?;
    waveform.define_method("duration", method!(RubyWaveform::duration, 0))?;
    waveform.define_method("data", method!(RubyWaveform::data, 0))?;
    waveform.define_private_method("__point", method!(RubyWaveform::point, 2))?;
    waveform.define_private_method("__save", method!(RubyWaveform::save, 3))?;
    waveform.define_private_method("__serialize", method!(RubyWaveform::serialize, 2))?;
    Ok(())
}
