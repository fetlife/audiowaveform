use std::str::FromStr;

use crate::Error;

/// A single RGBA color.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Color {
    /// Red channel.
    pub red: u8,
    /// Green channel.
    pub green: u8,
    /// Blue channel.
    pub blue: u8,
    /// Alpha channel, where `255` is fully opaque.
    pub alpha: u8,
}

impl Color {
    /// Creates an opaque color from RGB values.
    pub const fn opaque(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    /// Creates a color from RGBA values.
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Returns `true` when the color uses transparency.
    pub const fn has_alpha(self) -> bool {
        self.alpha < 255
    }
}

impl FromStr for Color {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !(s.len() == 6 || s.len() == 8) || !s.as_bytes().iter().all(u8::is_ascii_hexdigit) {
            return Err(Error::invalid_argument("color", "Invalid color value"));
        }

        let red = u8::from_str_radix(&s[0..2], 16)
            .map_err(|_| Error::invalid_argument("color", "Invalid color value"))?;
        let green = u8::from_str_radix(&s[2..4], 16)
            .map_err(|_| Error::invalid_argument("color", "Invalid color value"))?;
        let blue = u8::from_str_radix(&s[4..6], 16)
            .map_err(|_| Error::invalid_argument("color", "Invalid color value"))?;
        let alpha = if s.len() == 8 {
            u8::from_str_radix(&s[6..8], 16)
                .map_err(|_| Error::invalid_argument("color", "Invalid color value"))?
        } else {
            255
        };

        Ok(Self::rgba(red, green, blue, alpha))
    }
}

/// A named color scheme matching the historical audiowaveform presets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorScheme {
    /// Audacity-like colors.
    Audacity,
    /// Adobe Audition-like colors.
    Audition,
}

impl ColorScheme {
    /// Returns the full color palette associated with the scheme.
    pub fn palette(self) -> WaveformColors {
        match self {
            Self::Audacity => WaveformColors::audacity(),
            Self::Audition => WaveformColors::audition(),
        }
    }
}

impl FromStr for ColorScheme {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "audacity" => Ok(Self::Audacity),
            "audition" => Ok(Self::Audition),
            _ => Err(Error::invalid_argument(
                "colors",
                format!("Unknown color scheme: {s}"),
            )),
        }
    }
}

/// Complete color settings for waveform rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WaveformColors {
    /// Border color.
    pub border: Color,
    /// Background color.
    pub background: Color,
    /// Per-channel waveform colors. Channels wrap if there are fewer colors than channels.
    pub waveform: Vec<Color>,
    /// Axis-label color.
    pub axis_label: Color,
}

impl WaveformColors {
    /// Returns the Audacity-inspired palette.
    pub fn audacity() -> Self {
        Self {
            border: Color::opaque(0, 0, 0),
            background: Color::opaque(214, 214, 214),
            waveform: vec![Color::opaque(63, 77, 155)],
            axis_label: Color::opaque(0, 0, 0),
        }
    }

    /// Returns the Adobe Audition-inspired palette.
    pub fn audition() -> Self {
        Self {
            border: Color::opaque(157, 157, 157),
            background: Color::opaque(0, 63, 34),
            waveform: vec![Color::opaque(134, 252, 199)],
            axis_label: Color::opaque(190, 190, 190),
        }
    }

    /// Returns `true` when any configured color uses transparency.
    pub fn has_alpha(&self) -> bool {
        self.border.has_alpha()
            || self.background.has_alpha()
            || self.axis_label.has_alpha()
            || self.waveform.iter().any(|color| color.has_alpha())
    }
}

impl Default for WaveformColors {
    fn default() -> Self {
        Self::audacity()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{Color, ColorScheme, WaveformColors};

    #[test]
    fn parses_rgb_and_rgba_colors() {
        assert_eq!(
            Color::from_str("123456").expect("rgb"),
            Color::rgba(0x12, 0x34, 0x56, 0xff)
        );
        assert_eq!(
            Color::from_str("abcdef80").expect("rgba"),
            Color::rgba(0xab, 0xcd, 0xef, 0x80)
        );
        assert_eq!(
            Color::from_str("A1B2C3").expect("uppercase"),
            Color::rgba(0xa1, 0xb2, 0xc3, 0xff)
        );
    }

    #[test]
    fn rejects_invalid_color_strings() {
        for value in ["", "12345", "gggggg", "123456789"] {
            let error = Color::from_str(value).expect_err("invalid color");
            assert_eq!(error.to_string(), "Invalid color value");
        }
    }

    #[test]
    fn parses_color_schemes_and_detects_alpha() {
        assert_eq!(
            ColorScheme::from_str("audacity").expect("audacity"),
            ColorScheme::Audacity
        );
        assert_eq!(
            ColorScheme::from_str("AUDITION").expect("audition"),
            ColorScheme::Audition
        );
        let error = ColorScheme::from_str("unknown").expect_err("unknown scheme");
        assert_eq!(error.to_string(), "Unknown color scheme: unknown");

        let mut colors = WaveformColors::default();
        assert!(!colors.has_alpha());
        colors.background = Color::rgba(0, 0, 0, 128);
        assert!(colors.has_alpha());
    }
}
