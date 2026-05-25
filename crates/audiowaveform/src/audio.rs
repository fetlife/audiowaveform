#[cfg(feature = "decode")]
use std::fs::File;
use std::io::Read;
#[cfg(feature = "decode")]
use std::io::Seek;
#[cfg(feature = "decode")]
use std::path::Path;

#[cfg(feature = "decode")]
use symphonia::core::audio::GenericAudioBufferRef;
#[cfg(feature = "decode")]
use symphonia::core::codecs::audio::AudioDecoderOptions;
#[cfg(feature = "decode")]
use symphonia::core::errors::Error as SymphoniaError;
#[cfg(feature = "decode")]
use symphonia::core::formats::FormatOptions;
#[cfg(feature = "decode")]
use symphonia::core::formats::TrackType;
#[cfg(feature = "decode")]
use symphonia::core::formats::probe::Hint;
#[cfg(feature = "decode")]
use symphonia::core::io::{MediaSource, MediaSourceStream};
#[cfg(feature = "decode")]
use symphonia::core::meta::MetadataOptions;
#[cfg(feature = "decode")]
use symphonia::default::{get_codecs, get_probe};

#[cfg(feature = "decode")]
use crate::AudioFormat;
use crate::{AmplitudeScale, Error, Waveform, WaveformPoint};

#[cfg(feature = "decode")]
struct ReadSeekMediaSource<R> {
    inner: R,
    byte_len: Option<u64>,
}

#[cfg(feature = "decode")]
impl<R> ReadSeekMediaSource<R> {
    fn new(inner: R, byte_len: Option<u64>) -> Self {
        Self { inner, byte_len }
    }
}

#[cfg(feature = "decode")]
impl<R: Read> Read for ReadSeekMediaSource<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

#[cfg(feature = "decode")]
impl<R: Seek> Seek for ReadSeekMediaSource<R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

#[cfg(feature = "decode")]
impl<R: Read + Seek + Send + Sync> MediaSource for ReadSeekMediaSource<R> {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        self.byte_len
    }
}

/// A decoded interleaved PCM audio buffer.
#[derive(Clone, Debug, PartialEq)]
pub struct PcmAudio {
    sample_rate: u32,
    channels: u16,
    samples: Vec<i16>,
}

impl PcmAudio {
    /// Creates a PCM buffer from interleaved 16-bit samples.
    pub fn new(sample_rate: u32, channels: u16, samples: Vec<i16>) -> Result<Self, Error> {
        if sample_rate == 0 {
            return Err(Error::invalid_argument(
                "sample rate",
                "Invalid input sample rate: must be greater than zero",
            ));
        }
        if channels == 0 {
            return Err(Error::invalid_argument(
                "channels",
                "Invalid number of input channels: must be greater than zero",
            ));
        }
        if !samples.len().is_multiple_of(usize::from(channels)) {
            return Err(Error::invalid_argument(
                "samples",
                "Interleaved PCM sample count must be divisible by the channel count",
            ));
        }
        Ok(Self {
            sample_rate,
            channels,
            samples,
        })
    }

    /// Returns the source sample rate in Hz.
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Returns the channel count.
    pub const fn channels(&self) -> u16 {
        self.channels
    }

    /// Returns the interleaved PCM samples.
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// Returns the number of audio frames.
    pub fn frame_count(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }

    /// Returns the duration in seconds.
    pub fn duration_seconds(&self) -> f64 {
        self.frame_count() as f64 / self.sample_rate as f64
    }
}

/// Waveform scale selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScaleSpec {
    /// Use a fixed number of source samples per waveform point.
    SamplesPerPixel(u32),
    /// Derive samples per waveform point from a target number of rendered pixels per second.
    PixelsPerSecond(u32),
    /// Fit a duration or full clip into the provided width.
    FitWidth {
        /// Output width in pixels.
        width_pixels: u32,
        /// Optional `(start_time, end_time)` range in seconds.
        time_range: Option<(f64, f64)>,
    },
}

impl ScaleSpec {
    /// Resolves the scale to a concrete number of samples per waveform point.
    pub fn resolve(self, sample_rate: u32, frame_count: usize) -> Result<u32, Error> {
        let resolved = match self {
            Self::SamplesPerPixel(value) => value,
            Self::PixelsPerSecond(value) => {
                if value == 0 {
                    return Err(Error::invalid_argument(
                        "pixels per second",
                        "Invalid pixels per second: must be greater than zero",
                    ));
                }
                sample_rate / value
            }
            Self::FitWidth {
                width_pixels,
                time_range,
            } => {
                if width_pixels == 0 {
                    return Err(Error::invalid_argument(
                        "image width",
                        "Invalid image width: minimum 1",
                    ));
                }
                let frames = if let Some((start, end)) = time_range {
                    if end < start {
                        return Err(Error::invalid_argument(
                            "end time",
                            format!("Invalid end time, must be greater than {start}"),
                        ));
                    }
                    ((end - start) * sample_rate as f64) as u64
                } else {
                    frame_count as u64
                };
                (frames / u64::from(width_pixels)) as u32
            }
        };

        if resolved < 2 {
            return Err(Error::invalid_argument("zoom", "Invalid zoom: minimum 2"));
        }

        Ok(resolved)
    }
}

/// Waveform generation settings.
#[derive(Clone, Debug, PartialEq)]
pub struct GenerateOptions {
    /// Scale selection.
    pub scale: ScaleSpec,
    /// Whether to keep each source channel separate in the waveform output.
    pub split_channels: bool,
    /// Optional post-generation amplitude scaling.
    pub amplitude_scale: Option<AmplitudeScale>,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            scale: ScaleSpec::SamplesPerPixel(256),
            split_channels: false,
            amplitude_scale: None,
        }
    }
}

/// Supported raw audio sample encodings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RawSampleFormat {
    /// Signed 8-bit integer.
    S8,
    /// Unsigned 8-bit integer.
    U8,
    /// Signed little-endian 16-bit integer.
    S16Le,
    /// Signed big-endian 16-bit integer.
    S16Be,
    /// Signed little-endian 24-bit integer.
    S24Le,
    /// Signed big-endian 24-bit integer.
    S24Be,
    /// Signed little-endian 32-bit integer.
    S32Le,
    /// Signed big-endian 32-bit integer.
    S32Be,
    /// Little-endian 32-bit float.
    F32Le,
    /// Big-endian 32-bit float.
    F32Be,
    /// Little-endian 64-bit float.
    F64Le,
    /// Big-endian 64-bit float.
    F64Be,
}

impl RawSampleFormat {
    /// Returns the canonical CLI-friendly name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::S8 => "s8",
            Self::U8 => "u8",
            Self::S16Le => "s16le",
            Self::S16Be => "s16be",
            Self::S24Le => "s24le",
            Self::S24Be => "s24be",
            Self::S32Le => "s32le",
            Self::S32Be => "s32be",
            Self::F32Le => "f32le",
            Self::F32Be => "f32be",
            Self::F64Le => "f64le",
            Self::F64Be => "f64be",
        }
    }
}

impl std::str::FromStr for RawSampleFormat {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "s8" => Ok(Self::S8),
            "u8" => Ok(Self::U8),
            "s16le" => Ok(Self::S16Le),
            "s16be" => Ok(Self::S16Be),
            "s24le" => Ok(Self::S24Le),
            "s24be" => Ok(Self::S24Be),
            "s32le" => Ok(Self::S32Le),
            "s32be" => Ok(Self::S32Be),
            "f32le" => Ok(Self::F32Le),
            "f32be" => Ok(Self::F32Be),
            "f64le" => Ok(Self::F64Le),
            "f64be" => Ok(Self::F64Be),
            _ => Err(Error::UnsupportedFormat {
                format: s.to_string(),
            }),
        }
    }
}

/// Configuration for decoding raw audio.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RawAudioConfig {
    /// Source sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u16,
    /// Raw sample encoding.
    pub sample_format: RawSampleFormat,
}

impl RawAudioConfig {
    /// Creates a raw audio configuration.
    pub fn new(
        sample_rate: u32,
        channels: u16,
        sample_format: RawSampleFormat,
    ) -> Result<Self, Error> {
        if sample_rate == 0 {
            return Err(Error::invalid_argument(
                "raw sample rate",
                "Invalid input sample rate: must be greater than zero",
            ));
        }
        if channels == 0 {
            return Err(Error::invalid_argument(
                "raw channels",
                "Invalid number of input channels: must be greater than zero",
            ));
        }
        Ok(Self {
            sample_rate,
            channels,
            sample_format,
        })
    }
}

/// Generates a waveform from in-memory PCM samples.
pub fn generate_waveform_from_pcm(
    pcm: &PcmAudio,
    options: &GenerateOptions,
) -> Result<Waveform, Error> {
    let samples_per_pixel = options.scale.resolve(pcm.sample_rate, pcm.frame_count())?;
    let output_channels = if options.split_channels {
        pcm.channels
    } else {
        1
    };
    let mut waveform = Waveform::new(pcm.sample_rate, samples_per_pixel, output_channels)?;

    let channels = usize::from(pcm.channels);
    let output_channels_usize = usize::from(output_channels);
    let mut mins = vec![i16::MAX; output_channels_usize];
    let mut maxs = vec![i16::MIN; output_channels_usize];
    let mut count = 0_u32;

    for frame in pcm.samples.chunks_exact(channels) {
        if output_channels == 1 {
            let sample = frame.iter().map(|value| i32::from(*value)).sum::<i32>() / channels as i32;
            mins[0] = mins[0].min(sample as i16);
            maxs[0] = maxs[0].max(sample as i16);
        } else {
            for (channel, sample) in frame.iter().enumerate() {
                mins[channel] = mins[channel].min(*sample);
                maxs[channel] = maxs[channel].max(*sample);
            }
        }

        count += 1;
        if count == samples_per_pixel {
            flush_frame(&mut waveform, &mins, &maxs)?;
            mins.fill(i16::MAX);
            maxs.fill(i16::MIN);
            count = 0;
        }
    }

    if count > 0 {
        flush_frame(&mut waveform, &mins, &maxs)?;
    }

    match options.amplitude_scale {
        Some(scale) => waveform.scale_amplitude(scale),
        None => Ok(waveform),
    }
}

/// Generates a waveform from raw audio bytes.
pub fn generate_waveform_from_raw_reader<R: Read>(
    mut reader: R,
    config: &RawAudioConfig,
    options: &GenerateOptions,
) -> Result<Waveform, Error> {
    let pcm = decode_raw_audio_reader(&mut reader, config)?;
    generate_waveform_from_pcm(&pcm, options)
}

/// Decodes raw audio bytes into interleaved 16-bit PCM.
pub fn decode_raw_audio_reader<R: Read>(
    mut reader: R,
    config: &RawAudioConfig,
) -> Result<PcmAudio, Error> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    parse_raw_audio(&bytes, config)
}

/// Generates a waveform from an audio file path using Symphonia.
#[cfg(feature = "decode")]
pub fn generate_waveform_from_path(
    path: impl AsRef<Path>,
    options: &GenerateOptions,
) -> Result<Waveform, Error> {
    let path = path.as_ref();
    let format = AudioFormat::from_path(path).ok_or_else(|| Error::UnsupportedFormat {
        format: path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string(),
    })?;
    let file = File::open(path)?;
    generate_waveform_from_reader(file, Some(format), options)
}

/// Decodes an audio file path into interleaved 16-bit PCM using Symphonia.
#[cfg(feature = "decode")]
pub fn decode_audio_from_path(path: impl AsRef<Path>) -> Result<PcmAudio, Error> {
    let path = path.as_ref();
    let format = AudioFormat::from_path(path).ok_or_else(|| Error::UnsupportedFormat {
        format: path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string(),
    })?;
    let file = File::open(path)?;
    decode_audio_from_reader(file, Some(format))
}

/// Generates a waveform from an arbitrary seekable audio reader using Symphonia.
#[cfg(feature = "decode")]
pub fn generate_waveform_from_reader<R: Read + Seek + Send + Sync + 'static>(
    reader: R,
    format_hint: Option<AudioFormat>,
    options: &GenerateOptions,
) -> Result<Waveform, Error> {
    let pcm = decode_audio_from_reader(reader, format_hint)?;
    generate_waveform_from_pcm(&pcm, options)
}

/// Decodes an arbitrary seekable audio reader into interleaved 16-bit PCM using Symphonia.
#[cfg(feature = "decode")]
pub fn decode_audio_from_reader<R: Read + Seek + Send + Sync + 'static>(
    reader: R,
    format_hint: Option<AudioFormat>,
) -> Result<PcmAudio, Error> {
    decode_audio_reader(reader, format_hint)
}

fn flush_frame(waveform: &mut Waveform, mins: &[i16], maxs: &[i16]) -> Result<(), Error> {
    let points = mins
        .iter()
        .zip(maxs.iter())
        .map(|(min, max)| WaveformPoint {
            min: *min,
            max: *max,
        })
        .collect::<Vec<_>>();
    waveform.push_frame(&points)
}

fn parse_raw_audio(bytes: &[u8], config: &RawAudioConfig) -> Result<PcmAudio, Error> {
    let width = raw_sample_width(config.sample_format);
    if !bytes.len().is_multiple_of(width) {
        return Err(Error::invalid_data(
            "Raw audio byte length is not aligned to the sample format",
        ));
    }

    let mut samples = Vec::with_capacity(bytes.len() / width);
    for chunk in bytes.chunks_exact(width) {
        samples.push(parse_raw_sample(chunk, config.sample_format));
    }
    PcmAudio::new(config.sample_rate, config.channels, samples)
}

fn raw_sample_width(format: RawSampleFormat) -> usize {
    match format {
        RawSampleFormat::S8 | RawSampleFormat::U8 => 1,
        RawSampleFormat::S16Le | RawSampleFormat::S16Be => 2,
        RawSampleFormat::S24Le | RawSampleFormat::S24Be => 3,
        RawSampleFormat::S32Le
        | RawSampleFormat::S32Be
        | RawSampleFormat::F32Le
        | RawSampleFormat::F32Be => 4,
        RawSampleFormat::F64Le | RawSampleFormat::F64Be => 8,
    }
}

fn parse_raw_sample(bytes: &[u8], format: RawSampleFormat) -> i16 {
    match format {
        RawSampleFormat::S8 => i16::from(i8::from_ne_bytes([bytes[0]])) << 8,
        RawSampleFormat::U8 => (i16::from(bytes[0]) - 128) << 8,
        RawSampleFormat::S16Le => i16::from_le_bytes([bytes[0], bytes[1]]),
        RawSampleFormat::S16Be => i16::from_be_bytes([bytes[0], bytes[1]]),
        RawSampleFormat::S24Le => {
            clamp_float_to_i16(sign_extend_24([bytes[0], bytes[1], bytes[2]]) as f64 / 256.0)
        }
        RawSampleFormat::S24Be => {
            clamp_float_to_i16(sign_extend_24([bytes[2], bytes[1], bytes[0]]) as f64 / 256.0)
        }
        RawSampleFormat::S32Le => clamp_float_to_i16(
            i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64 / 65_536.0,
        ),
        RawSampleFormat::S32Be => clamp_float_to_i16(
            i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64 / 65_536.0,
        ),
        RawSampleFormat::F32Le => clamp_float_to_i16(
            f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
                * f64::from(i16::MAX),
        ),
        RawSampleFormat::F32Be => clamp_float_to_i16(
            f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
                * f64::from(i16::MAX),
        ),
        RawSampleFormat::F64Le => clamp_float_to_i16(
            f64::from_le_bytes(bytes.try_into().expect("checked width")) * f64::from(i16::MAX),
        ),
        RawSampleFormat::F64Be => clamp_float_to_i16(
            f64::from_be_bytes(bytes.try_into().expect("checked width")) * f64::from(i16::MAX),
        ),
    }
}

fn sign_extend_24(bytes: [u8; 3]) -> i32 {
    let sign = if bytes[2] & 0x80 != 0 { 0xFF } else { 0x00 };
    i32::from_le_bytes([bytes[0], bytes[1], bytes[2], sign])
}

fn clamp_float_to_i16(value: f64) -> i16 {
    value.clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

#[cfg(feature = "decode")]
pub(crate) fn decode_audio_reader<R: Read + Seek + Send + Sync + 'static>(
    mut reader: R,
    format_hint: Option<AudioFormat>,
) -> Result<PcmAudio, Error> {
    let mut hint = Hint::new();
    if let Some(format) = format_hint {
        if format == AudioFormat::Opus {
            return Err(Error::UnsupportedFormat {
                format: "opus".to_string(),
            });
        }
        hint.with_extension(format.as_str());
    }

    let byte_len = reader.stream_position().ok().and_then(|position| {
        let len = reader.seek(std::io::SeekFrom::End(0)).ok();
        let _ = reader.seek(std::io::SeekFrom::Start(position));
        len
    });
    let source = MediaSourceStream::new(
        Box::new(ReadSeekMediaSource::new(reader, byte_len)),
        Default::default(),
    );
    let mut format = get_probe().probe(
        &hint,
        source,
        FormatOptions::default(),
        MetadataOptions::default(),
    )?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or(Error::MissingMetadata {
            name: "default track",
        })?;
    let codec_params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .ok_or(Error::MissingMetadata {
            name: "audio codec parameters",
        })?;
    let sample_rate = codec_params.sample_rate.ok_or(Error::MissingMetadata {
        name: "sample_rate",
    })?;
    let channels = codec_params
        .channels
        .as_ref()
        .ok_or(Error::MissingMetadata { name: "channels" })?
        .count() as u16;
    let mut decoder =
        get_codecs().make_audio_decoder(codec_params, &AudioDecoderOptions::default())?;

    let mut samples = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(error) => return Err(error.into()),
        };

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(error) => return Err(error.into()),
        };

        let decoded: GenericAudioBufferRef<'_> = decoded;
        let start = samples.len();
        samples.resize(start + decoded.samples_interleaved(), 0);
        decoded.copy_to_slice_interleaved(&mut samples[start..]);
    }

    PcmAudio::new(sample_rate, channels, samples)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{
        GenerateOptions, PcmAudio, RawAudioConfig, RawSampleFormat, ScaleSpec,
        generate_waveform_from_pcm, parse_raw_audio, parse_raw_sample,
    };
    use crate::{AmplitudeScale, WaveformPoint};

    #[test]
    fn validates_pcm_audio_construction() {
        let pcm = PcmAudio::new(48_000, 2, vec![1, 2, 3, 4]).expect("pcm");
        assert_eq!(pcm.frame_count(), 2);
        assert_eq!(pcm.duration_seconds(), 2.0 / 48_000.0);

        let error = PcmAudio::new(0, 1, vec![1]).expect_err("invalid sample rate");
        assert_eq!(
            error.to_string(),
            "Invalid input sample rate: must be greater than zero"
        );

        let error = PcmAudio::new(48_000, 0, vec![1]).expect_err("invalid channels");
        assert_eq!(
            error.to_string(),
            "Invalid number of input channels: must be greater than zero"
        );

        let error = PcmAudio::new(48_000, 2, vec![1, 2, 3]).expect_err("unaligned samples");
        assert_eq!(
            error.to_string(),
            "Interleaved PCM sample count must be divisible by the channel count"
        );
    }

    #[test]
    fn resolves_scale_specifications_and_rejects_invalid_values() {
        assert_eq!(
            ScaleSpec::SamplesPerPixel(64)
                .resolve(48_000, 96_000)
                .expect("samples per pixel"),
            64
        );
        assert_eq!(
            ScaleSpec::PixelsPerSecond(100)
                .resolve(48_000, 96_000)
                .expect("pixels per second"),
            480
        );
        assert_eq!(
            ScaleSpec::FitWidth {
                width_pixels: 400,
                time_range: Some((0.0, 4.0)),
            }
            .resolve(48_000, 0)
            .expect("fit width"),
            480
        );

        let error = ScaleSpec::PixelsPerSecond(0)
            .resolve(48_000, 0)
            .expect_err("invalid pixels per second");
        assert_eq!(
            error.to_string(),
            "Invalid pixels per second: must be greater than zero"
        );

        let error = ScaleSpec::FitWidth {
            width_pixels: 0,
            time_range: None,
        }
        .resolve(48_000, 96_000)
        .expect_err("invalid width");
        assert_eq!(error.to_string(), "Invalid image width: minimum 1");

        let error = ScaleSpec::FitWidth {
            width_pixels: 400,
            time_range: Some((5.0, 4.0)),
        }
        .resolve(48_000, 96_000)
        .expect_err("invalid range");
        assert_eq!(
            error.to_string(),
            "Invalid end time, must be greater than 5"
        );

        let error = ScaleSpec::FitWidth {
            width_pixels: 100_000,
            time_range: None,
        }
        .resolve(48_000, 96_000)
        .expect_err("zoom too small");
        assert_eq!(error.to_string(), "Invalid zoom: minimum 2");
    }

    #[test]
    fn parses_raw_sample_formats_and_validates_raw_audio_config() {
        assert_eq!(
            RawSampleFormat::from_str("s16le").expect("raw sample format"),
            RawSampleFormat::S16Le
        );
        assert_eq!(
            RawSampleFormat::from_str("F64BE").expect("raw sample format"),
            RawSampleFormat::F64Be
        );
        let error = RawSampleFormat::from_str("pcm").expect_err("unsupported format");
        assert_eq!(error.to_string(), "Unsupported format: pcm");

        let config = RawAudioConfig::new(44_100, 2, RawSampleFormat::S16Le).expect("config");
        assert_eq!(config.sample_rate, 44_100);
        assert_eq!(config.channels, 2);

        let error =
            RawAudioConfig::new(0, 1, RawSampleFormat::S16Le).expect_err("invalid sample rate");
        assert_eq!(
            error.to_string(),
            "Invalid input sample rate: must be greater than zero"
        );

        let error =
            RawAudioConfig::new(44_100, 0, RawSampleFormat::S16Le).expect_err("invalid channels");
        assert_eq!(
            error.to_string(),
            "Invalid number of input channels: must be greater than zero"
        );
    }

    #[test]
    fn decodes_representative_raw_sample_formats() {
        let cases = [
            (RawSampleFormat::S8, vec![0x80], i16::MIN),
            (RawSampleFormat::U8, vec![0xff], 32_512),
            (RawSampleFormat::S16Le, vec![0x34, 0x12], 0x1234),
            (RawSampleFormat::S16Be, vec![0x12, 0x34], 0x1234),
            (RawSampleFormat::S24Le, vec![0x00, 0x00, 0x01], 256),
            (RawSampleFormat::S24Be, vec![0x01, 0x00, 0x00], 256),
            (RawSampleFormat::S32Le, vec![0x00, 0x00, 0x01, 0x00], 1),
            (RawSampleFormat::S32Be, vec![0x00, 0x01, 0x00, 0x00], 1),
            (
                RawSampleFormat::F32Le,
                1.0_f32.to_le_bytes().to_vec(),
                i16::MAX,
            ),
            (
                RawSampleFormat::F32Be,
                1.0_f32.to_be_bytes().to_vec(),
                i16::MAX,
            ),
            (
                RawSampleFormat::F64Le,
                1.0_f64.to_le_bytes().to_vec(),
                i16::MAX,
            ),
            (
                RawSampleFormat::F64Be,
                1.0_f64.to_be_bytes().to_vec(),
                i16::MAX,
            ),
        ];

        for (format, bytes, expected) in cases {
            assert_eq!(parse_raw_sample(&bytes, format), expected, "{format:?}");
        }
    }

    #[test]
    fn decodes_raw_audio_and_rejects_unaligned_buffers() {
        let config = RawAudioConfig::new(16_000, 1, RawSampleFormat::S16Le).expect("config");
        let pcm = parse_raw_audio(&[0x01, 0x00, 0xff, 0xff], &config).expect("parse raw");
        assert_eq!(pcm.samples(), &[1, -1]);

        let error = parse_raw_audio(&[0x01], &config).expect_err("unaligned bytes");
        assert_eq!(
            error.to_string(),
            "Raw audio byte length is not aligned to the sample format"
        );
    }

    #[test]
    fn generates_waveforms_from_pcm_for_mixed_and_split_channels() {
        let pcm = PcmAudio::new(48_000, 2, vec![100, 300, 200, 400, -100, -300, -200, -400])
            .expect("pcm");

        let mixed = generate_waveform_from_pcm(
            &pcm,
            &GenerateOptions {
                scale: ScaleSpec::SamplesPerPixel(2),
                split_channels: false,
                amplitude_scale: None,
            },
        )
        .expect("mixed waveform");
        assert_eq!(mixed.channels(), 1);
        assert_eq!(
            mixed.point(0, 0).expect("first point"),
            WaveformPoint { min: 200, max: 300 }
        );
        assert_eq!(
            mixed.point(0, 1).expect("second point"),
            WaveformPoint {
                min: -300,
                max: -200,
            }
        );

        let split = generate_waveform_from_pcm(
            &pcm,
            &GenerateOptions {
                scale: ScaleSpec::SamplesPerPixel(2),
                split_channels: true,
                amplitude_scale: Some(AmplitudeScale::Fixed(2.0)),
            },
        )
        .expect("split waveform");
        assert_eq!(split.channels(), 2);
        assert_eq!(
            split.point(0, 0).expect("left point"),
            WaveformPoint { min: 200, max: 400 }
        );
        assert_eq!(
            split.point(1, 1).expect("right point"),
            WaveformPoint {
                min: -800,
                max: -600,
            }
        );
    }
}
