use std::io;

use thiserror::Error;

/// Errors returned by the `audiowaveform` library.
#[derive(Debug, Error)]
pub enum Error {
    /// Returned when an argument or option is invalid.
    #[error("{message}")]
    InvalidArgument {
        /// The logical argument name that failed validation.
        name: &'static str,
        /// Human-readable validation message.
        message: String,
    },

    /// Returned when waveform or audio data is malformed.
    #[error("{message}")]
    InvalidData {
        /// Human-readable validation message.
        message: String,
    },

    /// Returned when a format name or feature is unsupported.
    #[error("Unsupported format: {format}")]
    UnsupportedFormat {
        /// The unsupported format identifier.
        format: String,
    },

    /// Returned when a required value is missing from structured data.
    #[error("Missing value: {name}")]
    MissingValue {
        /// The missing field name.
        name: &'static str,
    },

    /// Returned when required stream metadata is unavailable.
    #[error("Missing metadata: {name}")]
    MissingMetadata {
        /// The missing metadata field name.
        name: &'static str,
    },

    /// Returned for I/O failures.
    #[error(transparent)]
    Io(#[from] io::Error),

    /// Returned for JSON parsing failures.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Returned for Symphonia decode failures.
    #[cfg(feature = "decode")]
    #[error(transparent)]
    Symphonia(#[from] symphonia::core::errors::Error),

    /// Returned for WAV encoding failures.
    #[cfg(feature = "wav")]
    #[error(transparent)]
    Hound(#[from] hound::Error),

    /// Returned for PNG encoding failures.
    #[cfg(feature = "render")]
    #[error(transparent)]
    PngEncoding(#[from] png::EncodingError),
}

impl Error {
    pub(crate) fn invalid_argument(name: &'static str, message: impl Into<String>) -> Self {
        Self::InvalidArgument {
            name,
            message: message.into(),
        }
    }

    pub(crate) fn invalid_data(message: impl Into<String>) -> Self {
        Self::InvalidData {
            message: message.into(),
        }
    }
}
