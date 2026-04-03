use std::fs::File;
use std::io::{self, Cursor, Read, Write};
use std::path::Path;
use std::process::ExitCode;
use std::str::FromStr;

use audiowaveform::{
    AmplitudeScale, AudioFormat, BarStyle, Color, ColorScheme, Error, GenerateOptions,
    RawAudioConfig, RawSampleFormat, RenderOptions, RenderStyle, ScaleSpec, Waveform,
    WaveformColors, WaveformFormat, decode_audio_from_reader, decode_raw_audio_reader,
    generate_waveform_from_raw_reader, generate_waveform_from_reader, write_pcm_as_wav,
    write_waveform_png,
};
use clap::{CommandFactory, Parser};

#[derive(Debug, Parser)]
#[command(
    name = "audiowaveform",
    disable_version_flag = true,
    disable_help_flag = true
)]
struct Cli {
    #[arg(long = "help")]
    help: bool,

    #[arg(short = 'v', long = "version")]
    version: bool,

    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    #[arg(short = 'i', long = "input-filename")]
    input_filename: Option<String>,

    #[arg(short = 'o', long = "output-filename")]
    output_filename: Option<String>,

    #[arg(long = "split-channels")]
    split_channels: bool,

    #[arg(long = "input-format")]
    input_format: Option<String>,

    #[arg(long = "output-format")]
    output_format: Option<String>,

    #[arg(short = 'z', long = "zoom")]
    zoom: Option<String>,

    #[arg(long = "pixels-per-second")]
    pixels_per_second: Option<i32>,

    #[arg(short = 'b', long = "bits")]
    bits: Option<i32>,

    #[arg(short = 's', long = "start", default_value_t = 0.0)]
    start: f64,

    #[arg(short = 'e', long = "end")]
    end: Option<f64>,

    #[arg(short = 'w', long = "width", default_value_t = 800)]
    width: i32,

    #[arg(short = 'h', long = "height", default_value_t = 250)]
    height: i32,

    #[arg(short = 'c', long = "colors", default_value = "audacity")]
    color_scheme: String,

    #[arg(long = "border-color")]
    border_color: Option<String>,

    #[arg(long = "background-color")]
    background_color: Option<String>,

    #[arg(long = "waveform-color")]
    waveform_color: Option<String>,

    #[arg(long = "waveform-style", default_value = "normal")]
    waveform_style: String,

    #[arg(long = "bar-width", default_value_t = 8)]
    bar_width: i32,

    #[arg(long = "bar-gap", default_value_t = 4)]
    bar_gap: i32,

    #[arg(long = "bar-style", default_value = "square")]
    bar_style: String,

    #[arg(long = "axis-label-color")]
    axis_label_color: Option<String>,

    #[arg(long = "no-axis-labels")]
    no_axis_labels: bool,

    #[arg(long = "with-axis-labels")]
    with_axis_labels: bool,

    #[arg(long = "amplitude-scale", default_value = "1.0")]
    amplitude_scale: String,

    #[arg(long = "compression", default_value_t = -1)]
    compression: i32,

    #[arg(long = "raw-samplerate")]
    raw_sample_rate: Option<i32>,

    #[arg(long = "raw-channels")]
    raw_channels: Option<i32>,

    #[arg(long = "raw-format")]
    raw_format: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CliFormat {
    Mp3,
    Wav,
    Flac,
    Ogg,
    Opus,
    Raw,
    Dat,
    Json,
    Txt,
    Png,
}

impl CliFormat {
    fn from_name(name: &str) -> Result<Self, String> {
        match name.to_ascii_lowercase().as_str() {
            "mp3" => Ok(Self::Mp3),
            "wav" | "w64" => Ok(Self::Wav),
            "flac" => Ok(Self::Flac),
            "ogg" | "oga" => Ok(Self::Ogg),
            "opus" => Ok(Self::Opus),
            "raw" => Ok(Self::Raw),
            "dat" => Ok(Self::Dat),
            "json" => Ok(Self::Json),
            "txt" => Ok(Self::Txt),
            "png" => Ok(Self::Png),
            _ => Err(format!("Unknown file format: {name}")),
        }
    }

    fn from_path(path: &str) -> Result<Self, String> {
        let extension = Path::new(path)
            .extension()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("Unknown file format: {path}"))?;
        Self::from_name(extension)
    }

    fn is_audio_input(self) -> bool {
        matches!(
            self,
            Self::Mp3 | Self::Wav | Self::Flac | Self::Ogg | Self::Opus | Self::Raw
        )
    }

    fn is_waveform_input(self) -> bool {
        matches!(self, Self::Dat | Self::Json)
    }

    fn as_audio_format(self) -> Option<AudioFormat> {
        match self {
            Self::Mp3 => Some(AudioFormat::Mp3),
            Self::Wav => Some(AudioFormat::Wav),
            Self::Flac => Some(AudioFormat::Flac),
            Self::Ogg => Some(AudioFormat::Ogg),
            Self::Opus => Some(AudioFormat::Opus),
            Self::Raw => Some(AudioFormat::Raw),
            _ => None,
        }
    }

    fn as_waveform_format(self) -> Option<WaveformFormat> {
        match self {
            Self::Dat => Some(WaveformFormat::Dat),
            Self::Json => Some(WaveformFormat::Json),
            Self::Txt => Some(WaveformFormat::Txt),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Flac => "flac",
            Self::Ogg => "ogg",
            Self::Opus => "opus",
            Self::Raw => "raw",
            Self::Dat => "dat",
            Self::Json => "json",
            Self::Txt => "txt",
            Self::Png => "png",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ParsedAmplitudeScale {
    Fixed(f64),
    Auto,
}

struct Logger {
    quiet: bool,
}

impl Logger {
    fn info(&self, message: impl AsRef<str>) {
        if !self.quiet {
            eprintln!("{}", message.as_ref());
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.help {
        let mut command = Cli::command();
        if command.print_help().is_ok() {
            println!();
        }
        return ExitCode::SUCCESS;
    }
    if cli.version {
        println!("AudioWaveform v{}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let logger = Logger { quiet: cli.quiet };
    if cli.height < 1 {
        return Err("Invalid image height: minimum 1".to_string());
    }
    let input_format = resolve_format(
        cli.input_filename.as_deref(),
        cli.input_format.as_deref(),
        true,
    )?;
    let output_format = resolve_format(
        cli.output_filename.as_deref(),
        cli.output_format.as_deref(),
        false,
    )?;
    let bits = resolve_bits(cli.bits)?;
    let compression = resolve_compression(cli.compression)?;
    let amplitude = parse_amplitude_scale(&cli.amplitude_scale)?;
    let colors = resolve_colors(&cli)?;
    let axis_labels = if cli.with_axis_labels {
        true
    } else {
        !cli.no_axis_labels
    };
    let render_style = resolve_render_style(&cli)?;
    let scale = resolve_scale(&cli)?;
    let raw_config = if input_format == CliFormat::Raw {
        Some(resolve_raw_audio_config(&cli)?)
    } else {
        None
    };
    let has_resample = cli.zoom.is_some() || cli.pixels_per_second.is_some() || cli.end.is_some();

    if input_format.is_audio_input() && output_format == CliFormat::Wav {
        let bytes = read_input_bytes(cli.input_filename.as_deref())?;
        let mut output = create_output(cli.output_filename.as_deref())?;
        match input_format {
            CliFormat::Raw => {
                let pcm = decode_raw_audio_reader(
                    Cursor::new(bytes),
                    raw_config.as_ref().expect("validated raw config"),
                )
                .map_err(stringify_error)?;
                write_pcm_as_wav(&pcm, &mut output).map_err(stringify_error)?;
            }
            _ => {
                let pcm =
                    decode_audio_from_reader(Cursor::new(bytes), input_format.as_audio_format())
                        .map_err(stringify_error)?;
                write_pcm_as_wav(&pcm, &mut output).map_err(stringify_error)?;
            }
        }
    } else if input_format.is_audio_input()
        && matches!(output_format, CliFormat::Dat | CliFormat::Json)
    {
        let waveform = generate_waveform_from_input(
            cli.input_filename.as_deref(),
            input_format,
            raw_config.as_ref(),
            GenerateOptions {
                scale,
                split_channels: cli.split_channels,
                amplitude_scale: match amplitude {
                    ParsedAmplitudeScale::Auto => Some(AmplitudeScale::Auto),
                    ParsedAmplitudeScale::Fixed(_) => None,
                },
            },
        )?;
        let mut output = create_output(cli.output_filename.as_deref())?;
        waveform
            .write_to_writer(
                &mut output,
                output_format.as_waveform_format().expect("waveform format"),
                Some(bits.unwrap_or(16) as u8),
            )
            .map_err(stringify_error)?;
    } else if input_format.is_waveform_input()
        && matches!(
            output_format,
            CliFormat::Dat | CliFormat::Json | CliFormat::Txt
        )
        && !has_resample
    {
        let waveform = load_waveform_input(cli.input_filename.as_deref(), input_format)?;
        let mut output = create_output(cli.output_filename.as_deref())?;
        waveform
            .write_to_writer(
                &mut output,
                output_format.as_waveform_format().expect("waveform format"),
                bits.map(|bits| bits as u8),
            )
            .map_err(stringify_error)?;
    } else if input_format.is_waveform_input()
        && matches!(output_format, CliFormat::Dat | CliFormat::Json)
        && has_resample
    {
        let waveform = load_waveform_input(cli.input_filename.as_deref(), input_format)?;
        let resampled = waveform.resample(scale).map_err(stringify_error)?;
        let mut output = create_output(cli.output_filename.as_deref())?;
        resampled
            .write_to_writer(
                &mut output,
                output_format.as_waveform_format().expect("waveform format"),
                bits.map(|bits| bits as u8),
            )
            .map_err(stringify_error)?;
    } else if (input_format.is_audio_input() || input_format.is_waveform_input())
        && output_format == CliFormat::Png
    {
        let waveform = if input_format.is_audio_input() {
            generate_waveform_from_input(
                cli.input_filename.as_deref(),
                input_format,
                raw_config.as_ref(),
                GenerateOptions {
                    scale,
                    split_channels: cli.split_channels,
                    amplitude_scale: None,
                },
            )?
        } else {
            let waveform = load_waveform_input(cli.input_filename.as_deref(), input_format)?;
            waveform.resample(scale).map_err(stringify_error)?
        };

        let render_options = RenderOptions {
            width: cli.width as u32,
            height: cli.height as u32,
            start_time: cli.start,
            amplitude_scale: match amplitude {
                ParsedAmplitudeScale::Auto => AmplitudeScale::Auto,
                ParsedAmplitudeScale::Fixed(value) => AmplitudeScale::Fixed(value),
            },
            axis_labels,
            style: render_style,
            colors,
            png_compression_level: compression.map(|value| value as u8),
        };

        let mut output = create_output(cli.output_filename.as_deref())?;
        write_waveform_png(&waveform, &render_options, &mut output).map_err(stringify_error)?;
    } else {
        return Err(format!(
            "Can't generate {} format output from {} format input",
            output_format.name(),
            input_format.name()
        ));
    }

    logger.info("Done");
    Ok(())
}

fn resolve_format(
    filename: Option<&str>,
    explicit: Option<&str>,
    input: bool,
) -> Result<CliFormat, String> {
    if let Some(explicit) = explicit {
        return CliFormat::from_name(explicit);
    }
    if let Some(filename) = filename {
        return CliFormat::from_path(filename);
    }
    Err(if input {
        "Error: Must specify either input filename or input format".to_string()
    } else {
        "Error: Must specify either output filename or output format".to_string()
    })
}

fn resolve_bits(bits: Option<i32>) -> Result<Option<i32>, String> {
    match bits {
        Some(8 | 16) | None => Ok(bits),
        Some(_) => Err("Error: Invalid bits: must be either 8 or 16".to_string()),
    }
}

fn resolve_compression(compression: i32) -> Result<Option<i32>, String> {
    if (-1..=9).contains(&compression) {
        Ok((compression >= 0).then_some(compression))
    } else {
        Err(
            "Error: Invalid compression level: must be from 0 (none) to 9 (best), or -1 (default)"
                .to_string(),
        )
    }
}

fn parse_amplitude_scale(value: &str) -> Result<ParsedAmplitudeScale, String> {
    if value == "auto" {
        return Ok(ParsedAmplitudeScale::Auto);
    }
    let parsed = value
        .parse::<f64>()
        .map_err(|_| "Error: Invalid amplitude scale: must be a number".to_string())?;
    if parsed < 0.0 {
        Err("Error: Invalid amplitude scale: must be a positive number".to_string())
    } else {
        Ok(ParsedAmplitudeScale::Fixed(parsed))
    }
}

fn resolve_scale(cli: &Cli) -> Result<ScaleSpec, String> {
    if cli.zoom.is_some() && cli.end.is_some() {
        return Err("Specify either --end or --zoom but not both".to_string());
    }
    if cli.pixels_per_second.is_some() && cli.end.is_some() {
        return Err("Specify either --end or --pixels-per-second but not both".to_string());
    }
    if cli.zoom.is_some() && cli.pixels_per_second.is_some() {
        return Err("Specify either --zoom or --pixels-per-second but not both".to_string());
    }
    if cli.width < 1 {
        return Err("Invalid image width: minimum 1".to_string());
    }

    if let Some(end) = cli.end {
        return Ok(ScaleSpec::FitWidth {
            width_pixels: cli.width as u32,
            time_range: Some((cli.start, end)),
        });
    }
    if let Some(pixels_per_second) = cli.pixels_per_second {
        if pixels_per_second <= 0 {
            return Err("Invalid pixels per second: must be greater than zero".to_string());
        }
        return Ok(ScaleSpec::PixelsPerSecond(pixels_per_second as u32));
    }

    match cli.zoom.as_deref() {
        Some("auto") => Ok(ScaleSpec::FitWidth {
            width_pixels: cli.width as u32,
            time_range: None,
        }),
        Some(value) => {
            let zoom = value
                .parse::<u32>()
                .map_err(|_| "Error: Invalid zoom: must be a number or 'auto'".to_string())?;
            Ok(ScaleSpec::SamplesPerPixel(zoom))
        }
        None => Ok(ScaleSpec::SamplesPerPixel(256)),
    }
}

fn resolve_colors(cli: &Cli) -> Result<WaveformColors, String> {
    let mut colors = ColorScheme::from_str(&cli.color_scheme)
        .map_err(stringify_error)?
        .palette();

    if let Some(value) = &cli.border_color {
        colors.border = Color::from_str(value).map_err(stringify_error)?;
    }
    if let Some(value) = &cli.background_color {
        colors.background = Color::from_str(value).map_err(stringify_error)?;
    }
    if let Some(value) = &cli.axis_label_color {
        colors.axis_label = Color::from_str(value).map_err(stringify_error)?;
    }
    if let Some(value) = &cli.waveform_color {
        let waveform = value
            .split(',')
            .map(|item| Color::from_str(item).map_err(stringify_error))
            .collect::<Result<Vec<_>, _>>()?;
        if !waveform.is_empty() {
            colors.waveform = waveform;
        }
    }

    Ok(colors)
}

fn resolve_render_style(cli: &Cli) -> Result<RenderStyle, String> {
    match cli.waveform_style.as_str() {
        "normal" => Ok(RenderStyle::Normal),
        "bars" => {
            if cli.bar_width < 1 {
                return Err("Invalid bar width: minimum 1".to_string());
            }
            if cli.bar_gap < 0 {
                return Err("Invalid bar gap: minimum 0".to_string());
            }
            let style = match cli.bar_style.as_str() {
                "square" => BarStyle::Square,
                "rounded" => BarStyle::Rounded,
                value => return Err(format!("Unknown waveform bar style: {value}")),
            };
            Ok(RenderStyle::Bars {
                width: cli.bar_width as u32,
                gap: cli.bar_gap as u32,
                style,
            })
        }
        value => Err(format!("Unknown waveform style: {value}")),
    }
}

fn resolve_raw_audio_config(cli: &Cli) -> Result<RawAudioConfig, String> {
    let sample_rate = cli
        .raw_sample_rate
        .ok_or_else(|| "Error: Missing --raw-samplerate option".to_string())?;
    let channels = cli
        .raw_channels
        .ok_or_else(|| "Error: Missing --raw-channels option".to_string())?;
    let sample_format = cli
        .raw_format
        .as_deref()
        .ok_or_else(|| "Error: Missing --raw-format option".to_string())?;
    if sample_rate <= 0 {
        return Err("Invalid input sample rate: must be greater than zero".to_string());
    }
    if channels <= 0 {
        return Err("Invalid number of input channels: must be greater than zero".to_string());
    }

    let sample_format = RawSampleFormat::from_str(sample_format).map_err(stringify_error)?;
    RawAudioConfig::new(sample_rate as u32, channels as u16, sample_format).map_err(stringify_error)
}

fn generate_waveform_from_input(
    filename: Option<&str>,
    format: CliFormat,
    raw: Option<&RawAudioConfig>,
    options: GenerateOptions,
) -> Result<Waveform, String> {
    let bytes = read_input_bytes(filename)?;
    match format {
        CliFormat::Raw => generate_waveform_from_raw_reader(
            Cursor::new(bytes),
            raw.expect("validated raw config"),
            &options,
        )
        .map_err(stringify_error),
        _ => generate_waveform_from_reader(Cursor::new(bytes), format.as_audio_format(), &options)
            .map_err(stringify_error),
    }
}

fn load_waveform_input(filename: Option<&str>, format: CliFormat) -> Result<Waveform, String> {
    let bytes = read_input_bytes(filename)?;
    Waveform::load_from_reader(
        Cursor::new(bytes),
        format.as_waveform_format().expect("waveform input"),
    )
    .map_err(stringify_error)
}

fn read_input_bytes(filename: Option<&str>) -> Result<Vec<u8>, String> {
    if is_stdio_filename(filename) {
        let mut buffer = Vec::new();
        io::stdin()
            .lock()
            .read_to_end(&mut buffer)
            .map_err(|error| error.to_string())?;
        Ok(buffer)
    } else {
        std::fs::read(filename.expect("checked stdio")).map_err(|error| error.to_string())
    }
}

fn create_output(filename: Option<&str>) -> Result<Box<dyn Write>, String> {
    if is_stdio_filename(filename) {
        Ok(Box::new(io::stdout()))
    } else {
        File::create(filename.expect("checked stdio"))
            .map(|file| Box::new(file) as Box<dyn Write>)
            .map_err(|error| error.to_string())
    }
}

fn is_stdio_filename(filename: Option<&str>) -> bool {
    filename.is_none() || filename == Some("-")
}

fn stringify_error(error: Error) -> String {
    error.to_string()
}
