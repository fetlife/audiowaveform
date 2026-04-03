use std::path::Path;
use std::str::FromStr;

use crate::Error;

/// Supported audio container or source formats.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioFormat {
    /// MP3 audio.
    Mp3,
    /// WAV audio.
    Wav,
    /// FLAC audio.
    Flac,
    /// Ogg Vorbis audio.
    Ogg,
    /// Opus audio.
    Opus,
    /// Headerless raw PCM or floating-point audio.
    Raw,
}

impl AudioFormat {
    /// Returns the canonical lowercase name for the format.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Flac => "flac",
            Self::Ogg => "ogg",
            Self::Opus => "opus",
            Self::Raw => "raw",
        }
    }

    /// Infers an audio format from a filesystem path extension.
    pub fn from_path(path: impl AsRef<Path>) -> Option<Self> {
        let extension = path.as_ref().extension()?.to_str()?;
        Self::from_extension(extension)
    }

    /// Infers an audio format from a file extension string.
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_ascii_lowercase().as_str() {
            "mp3" => Some(Self::Mp3),
            "wav" | "w64" => Some(Self::Wav),
            "flac" => Some(Self::Flac),
            "ogg" | "oga" => Some(Self::Ogg),
            "opus" => Some(Self::Opus),
            "raw" => Some(Self::Raw),
            _ => None,
        }
    }
}

impl FromStr for AudioFormat {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_extension(s).ok_or_else(|| Error::UnsupportedFormat {
            format: s.to_string(),
        })
    }
}

/// Supported serialized waveform data formats.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaveformFormat {
    /// Binary `.dat` waveform data.
    Dat,
    /// Compact JSON waveform data.
    Json,
    /// Plain text CSV-like waveform data.
    Txt,
}

impl WaveformFormat {
    /// Returns the canonical lowercase name for the format.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dat => "dat",
            Self::Json => "json",
            Self::Txt => "txt",
        }
    }

    /// Infers a waveform data format from a filesystem path extension.
    pub fn from_path(path: impl AsRef<Path>) -> Option<Self> {
        let extension = path.as_ref().extension()?.to_str()?;
        Self::from_extension(extension)
    }

    /// Infers a waveform data format from a file extension string.
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_ascii_lowercase().as_str() {
            "dat" => Some(Self::Dat),
            "json" => Some(Self::Json),
            "txt" => Some(Self::Txt),
            _ => None,
        }
    }
}

impl FromStr for WaveformFormat {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_extension(s).ok_or_else(|| Error::UnsupportedFormat {
            format: s.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioFormat, WaveformFormat};

    #[test]
    fn infers_audio_formats_from_extensions_and_paths() {
        assert_eq!(AudioFormat::from_extension("mp3"), Some(AudioFormat::Mp3));
        assert_eq!(AudioFormat::from_extension("w64"), Some(AudioFormat::Wav));
        assert_eq!(AudioFormat::from_extension("oga"), Some(AudioFormat::Ogg));
        assert_eq!(AudioFormat::from_path("clip.flac"), Some(AudioFormat::Flac));
        assert_eq!(AudioFormat::from_path("clip.opus"), Some(AudioFormat::Opus));
        assert_eq!(AudioFormat::from_extension("unknown"), None);
    }

    #[test]
    fn parses_audio_format_strings() {
        assert_eq!("wav".parse::<AudioFormat>().expect("wav"), AudioFormat::Wav);
        assert_eq!("oga".parse::<AudioFormat>().expect("oga"), AudioFormat::Ogg);

        let error = "aac".parse::<AudioFormat>().expect_err("unsupported");
        assert_eq!(error.to_string(), "Unsupported format: aac");
    }

    #[test]
    fn infers_and_parses_waveform_formats() {
        assert_eq!(
            WaveformFormat::from_extension("dat"),
            Some(WaveformFormat::Dat)
        );
        assert_eq!(
            WaveformFormat::from_path("waveform.json"),
            Some(WaveformFormat::Json)
        );
        assert_eq!(
            "txt".parse::<WaveformFormat>().expect("txt"),
            WaveformFormat::Txt
        );
        assert_eq!(WaveformFormat::from_extension("png"), None);

        let error = "csv".parse::<WaveformFormat>().expect_err("unsupported");
        assert_eq!(error.to_string(), "Unsupported format: csv");
    }
}
