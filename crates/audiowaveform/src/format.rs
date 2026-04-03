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
