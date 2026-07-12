use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use image::{Rgba, RgbaImage};
use png::{BitDepth, ColorType, Compression, Encoder, FilterType};

use crate::Error;
use crate::{AmplitudeScale, Color, Waveform, WaveformColors};

/// Available waveform bar styles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BarStyle {
    /// Square-ended bars.
    Square,
    /// Rounded bars.
    Rounded,
}

/// Waveform rendering style.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderStyle {
    /// Draw each waveform point as a vertical line.
    Normal,
    /// Draw waveform points as grouped bars.
    Bars {
        /// Width of each bar in pixels.
        width: u32,
        /// Gap between bars in pixels.
        gap: u32,
        /// Bar end-cap style.
        style: BarStyle,
    },
}

/// Options controlling waveform rendering.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderOptions {
    /// Output image width in pixels.
    pub width: u32,
    /// Output image height in pixels.
    pub height: u32,
    /// Start time offset in seconds.
    pub start_time: f64,
    /// Amplitude scaling applied during rendering.
    pub amplitude_scale: AmplitudeScale,
    /// Whether to render time-axis labels.
    pub axis_labels: bool,
    /// Drawing style.
    pub style: RenderStyle,
    /// Render palette.
    pub colors: WaveformColors,
    /// Optional PNG compression level from `0` to `9`.
    pub png_compression_level: Option<u8>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 800,
            height: 250,
            start_time: 0.0,
            amplitude_scale: AmplitudeScale::Fixed(1.0),
            axis_labels: true,
            style: RenderStyle::Normal,
            colors: WaveformColors::default(),
            png_compression_level: None,
        }
    }
}

/// Renders a waveform into an RGBA image buffer.
pub fn render_waveform(waveform: &Waveform, options: &RenderOptions) -> Result<RgbaImage, Error> {
    if options.width == 0 {
        return Err(Error::invalid_argument(
            "image width",
            "Invalid image width: minimum 1",
        ));
    }
    if options.height == 0 {
        return Err(Error::invalid_argument(
            "image height",
            "Invalid image height: minimum 1",
        ));
    }
    if !options.start_time.is_finite() || options.start_time < 0.0 {
        return Err(Error::invalid_argument(
            "start time",
            "Invalid start time: minimum 0",
        ));
    }
    if waveform.is_empty() {
        return Err(Error::invalid_argument("waveform", "Empty waveform buffer"));
    }

    let mut image = RgbaImage::from_pixel(
        options.width,
        options.height,
        rgba(options.colors.background),
    );

    if options.axis_labels {
        draw_border(&mut image, rgba(options.colors.border));
    }

    match options.style {
        RenderStyle::Normal => draw_waveform_lines(&mut image, waveform, options)?,
        RenderStyle::Bars { width, gap, style } => {
            if width == 0 {
                return Err(Error::invalid_argument(
                    "bar width",
                    "Invalid bar width: minimum 1",
                ));
            }
            draw_waveform_bars(&mut image, waveform, options, width, gap, style)?
        }
    }

    if options.axis_labels {
        draw_time_axis_labels(&mut image, waveform, options);
    }

    Ok(image)
}

impl Waveform {
    /// Renders the waveform into an RGBA image buffer.
    pub fn render(&self, options: &RenderOptions) -> Result<RgbaImage, Error> {
        render_waveform(self, options)
    }

    /// Writes the waveform as a PNG image to an arbitrary writer.
    pub fn write_png<W: Write>(&self, options: &RenderOptions, writer: W) -> Result<(), Error> {
        write_waveform_png(self, options, writer)
    }
}

/// Writes a rendered waveform PNG to an arbitrary writer.
pub fn write_waveform_png<W: Write>(
    waveform: &Waveform,
    options: &RenderOptions,
    writer: W,
) -> Result<(), Error> {
    let image = render_waveform(waveform, options)?;
    let mut encoder = Encoder::new(writer, image.width(), image.height());
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_filter(FilterType::NoFilter);
    encoder.set_compression(match options.png_compression_level {
        Some(0) => Compression::Fast,
        Some(level @ 1..=6) if level <= 3 => Compression::Fast,
        Some(7..=9) => Compression::Best,
        _ => Compression::Default,
    });
    let mut png = encoder.write_header()?;
    png.write_image_data(image.as_raw())?;
    Ok(())
}

/// Renders a waveform directly to a PNG file.
pub fn render_waveform_to_path(
    waveform: &Waveform,
    options: &RenderOptions,
    path: impl AsRef<Path>,
) -> Result<(), Error> {
    let file = File::create(path)?;
    write_waveform_png(waveform, options, BufWriter::new(file))
}

fn draw_waveform_lines(
    image: &mut RgbaImage,
    waveform: &Waveform,
    options: &RenderOptions,
) -> Result<(), Error> {
    let top_y = if options.axis_labels { 1 } else { 0 };
    let bottom_y = if options.axis_labels {
        options.height as i32 - 2
    } else {
        options.height as i32 - 1
    };
    let start_index = seconds_to_pixels(waveform, options.start_time);
    let end_index = (start_index + options.width as usize).min(waveform.len());
    let amplitude =
        resolve_amplitude_scale(waveform, options.amplitude_scale, start_index, end_index)?;
    let channels = waveform.channels() as usize;
    let mut available_height = bottom_y - top_y + 1;
    let row_height = available_height / channels as i32;
    let mut waveform_top_y = top_y;

    for channel in 0..channels {
        let waveform_bottom_y = if channel == channels - 1 {
            waveform_top_y + available_height - 1
        } else {
            waveform_top_y + row_height
        };
        let height = waveform_bottom_y - waveform_top_y + 1;
        let color = options.colors.waveform[channel % options.colors.waveform.len()];
        for (x, index) in (0..options.width as usize).zip(start_index..) {
            if index >= waveform.len() {
                break;
            }
            let point = waveform
                .point(channel as u16, index)
                .expect("point within range");
            let low = i32::from(scale_sample(point.min, amplitude)) + 32768;
            let high = i32::from(scale_sample(point.max, amplitude)) + 32768;
            let top = waveform_top_y + height - 1 - high * height / 65_536;
            let bottom = waveform_top_y + height - 1 - low * height / 65_536;
            draw_vertical_line(image, x as i32, top, bottom, rgba(color));
        }
        available_height -= row_height + 1;
        waveform_top_y += row_height + 1;
    }
    Ok(())
}

fn draw_waveform_bars(
    image: &mut RgbaImage,
    waveform: &Waveform,
    options: &RenderOptions,
    bar_width: u32,
    bar_gap: u32,
    bar_style: BarStyle,
) -> Result<(), Error> {
    let top_y = if options.axis_labels { 1 } else { 0 };
    let bottom_y = if options.axis_labels {
        options.height as i32 - 2
    } else {
        options.height as i32 - 1
    };
    let start_index = seconds_to_pixels(waveform, options.start_time);
    let end_index = (start_index + options.width as usize).min(waveform.len());
    let amplitude =
        resolve_amplitude_scale(waveform, options.amplitude_scale, start_index, end_index)?;
    let channels = waveform.channels() as usize;
    let mut available_height = bottom_y - top_y + 1;
    let row_height = available_height / channels as i32;
    let mut waveform_top_y = top_y;
    let bar_total = (bar_width + bar_gap) as usize;
    let bar_start_index = (start_index / bar_total) * bar_total;
    let bar_start_offset = bar_start_index as isize - start_index as isize;

    for channel in 0..channels {
        let waveform_bottom_y = if channel == channels - 1 {
            waveform_top_y + available_height - 1
        } else {
            waveform_top_y + row_height
        };
        let height = waveform_bottom_y - waveform_top_y + 1;
        let color = rgba(options.colors.waveform[channel % options.colors.waveform.len()]);

        let mut index = bar_start_index;
        let mut x = bar_start_offset;
        while x < options.width as isize {
            let bar_height = get_bar_height(waveform, channel as u16, index, bar_total);
            let low = i32::from(scale_sample(-(bar_height as i16), amplitude)) + 32768;
            let high = i32::from(scale_sample(bar_height as i16, amplitude)) + 32768;
            let top = waveform_top_y + height - 1 - high * height / 65_536;
            let bottom = waveform_top_y + height - 1 - low * height / 65_536;
            if top != bottom {
                if bar_style == BarStyle::Rounded && bar_width > 2 && height >= 3 {
                    let radius = if bar_width > 4 {
                        (bar_width / 4) as i32
                    } else {
                        (bar_width / 2) as i32
                    };
                    draw_rounded_rect(
                        image,
                        x as i32,
                        top,
                        x as i32 + bar_width as i32 - 1,
                        bottom,
                        radius,
                        color,
                    );
                } else {
                    fill_rect(
                        image,
                        x as i32,
                        top,
                        x as i32 + bar_width as i32 - 1,
                        bottom,
                        color,
                    );
                }
            }
            index += bar_total;
            x += bar_total as isize;
        }

        available_height -= row_height + 1;
        waveform_top_y += row_height + 1;
    }
    Ok(())
}

fn get_bar_height(waveform: &Waveform, channel: u16, start: usize, width: usize) -> i32 {
    if start >= waveform.len() {
        return 0;
    }
    let mut low = i32::MAX;
    let mut high = i32::MIN;
    for index in start..(start + width).min(waveform.len()) {
        let point = waveform.point(channel, index).expect("point within range");
        low = low.min(i32::from(point.min));
        high = high.max(i32::from(point.max));
    }
    low = low.abs().clamp(0, i32::from(i16::MAX));
    high = high.abs().clamp(0, i32::from(i16::MAX));
    low.max(high)
}

fn draw_time_axis_labels(image: &mut RgbaImage, waveform: &Waveform, options: &RenderOptions) {
    let marker_height = 10_i32;
    let interval_secs = axis_label_scale(waveform, options);
    let first_secs = round_up_to_nearest(options.start_time, interval_secs);
    let axis_label_offset_secs = first_secs as f64 - options.start_time;
    let axis_label_offset_pixels = ((axis_label_offset_secs * waveform.sample_rate() as f64)
        as usize
        / waveform.samples_per_pixel() as usize) as i32;
    let border = rgba(options.colors.border);
    let text = rgba(options.colors.axis_label);
    let mut secs = first_secs;

    loop {
        let x = axis_label_offset_pixels
            + (((secs - first_secs) as i64 * waveform.sample_rate() as i64)
                / waveform.samples_per_pixel() as i64) as i32;
        if x >= image.width() as i32 {
            break;
        }
        draw_vertical_line(image, x, 0, marker_height, border);
        draw_vertical_line(
            image,
            x,
            image.height() as i32 - 1,
            image.height() as i32 - 1 - marker_height,
            border,
        );

        let label = seconds_to_string(secs);
        let width = text_width(&label);
        let label_x = x - (width / 2) + 1;
        let label_y = image.height() as i32 - 1 - marker_height - 1 - LABEL_FONT_HEIGHT;
        if label_x >= 0 {
            draw_text(image, label_x, label_y, &label, text);
        }
        secs += interval_secs;
    }
}

fn axis_label_scale(waveform: &Waveform, options: &RenderOptions) -> i32 {
    let steps = [1_i32, 2, 5, 10, 20, 30];
    let mut base = 1_i32;
    let mut index = 0_usize;
    loop {
        let secs = base * steps[index];
        let pixels = seconds_to_pixels(waveform, secs as f64) as i32;
        if pixels < 60 {
            index += 1;
            if index == steps.len() {
                base *= 60;
                index = 0;
            }
        } else {
            let _ = options;
            return secs;
        }
    }
}

fn resolve_amplitude_scale(
    waveform: &Waveform,
    scale: AmplitudeScale,
    start_index: usize,
    end_index: usize,
) -> Result<f64, Error> {
    match scale {
        AmplitudeScale::Fixed(value) => {
            if !value.is_finite() || value < 0.0 {
                Err(Error::invalid_argument(
                    "amplitude scale",
                    "Invalid amplitude scale: must be a positive number",
                ))
            } else {
                Ok(value)
            }
        }
        AmplitudeScale::Auto if start_index >= end_index => Ok(1.0),
        AmplitudeScale::Auto => waveform.auto_amplitude_scale(start_index, end_index),
    }
}

fn draw_border(image: &mut RgbaImage, color: Rgba<u8>) {
    let width = image.width() as i32;
    let height = image.height() as i32;
    draw_horizontal_line(image, 0, width - 1, 0, color);
    draw_horizontal_line(image, 0, width - 1, height - 1, color);
    draw_vertical_line(image, 0, 0, height - 1, color);
    draw_vertical_line(image, width - 1, 0, height - 1, color);
}

fn draw_vertical_line(image: &mut RgbaImage, x: i32, y1: i32, y2: i32, color: Rgba<u8>) {
    let start = y1.min(y2);
    let end = y1.max(y2);
    for y in start..=end {
        put_pixel(image, x, y, color);
    }
}

fn draw_horizontal_line(image: &mut RgbaImage, x1: i32, x2: i32, y: i32, color: Rgba<u8>) {
    let start = x1.min(x2);
    let end = x1.max(x2);
    for x in start..=end {
        put_pixel(image, x, y, color);
    }
}

fn fill_rect(image: &mut RgbaImage, left: i32, top: i32, right: i32, bottom: i32, color: Rgba<u8>) {
    for y in top..=bottom {
        for x in left..=right {
            put_pixel(image, x, y, color);
        }
    }
}

fn draw_rounded_rect(
    image: &mut RgbaImage,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    radius: i32,
    color: Rgba<u8>,
) {
    let left_arc_x = left + radius;
    let top_arc_y = top + radius;
    let right_arc_x = right - radius;
    let bottom_arc_y = bottom - radius;
    fill_rect(image, left, top_arc_y, right, bottom_arc_y, color);
    fill_rect(image, left_arc_x, top, right_arc_x, top_arc_y, color);
    fill_rect(image, left_arc_x, bottom_arc_y, right_arc_x, bottom, color);
    fill_quarter_circle(
        image,
        left_arc_x,
        top_arc_y,
        radius,
        Quadrant::TopLeft,
        color,
    );
    fill_quarter_circle(
        image,
        right_arc_x,
        top_arc_y,
        radius,
        Quadrant::TopRight,
        color,
    );
    fill_quarter_circle(
        image,
        left_arc_x,
        bottom_arc_y,
        radius,
        Quadrant::BottomLeft,
        color,
    );
    fill_quarter_circle(
        image,
        right_arc_x,
        bottom_arc_y,
        radius,
        Quadrant::BottomRight,
        color,
    );
}

enum Quadrant {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

fn fill_quarter_circle(
    image: &mut RgbaImage,
    center_x: i32,
    center_y: i32,
    radius: i32,
    quadrant: Quadrant,
    color: Rgba<u8>,
) {
    let radius_sq = radius * radius;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius_sq {
                continue;
            }
            let allowed = match quadrant {
                Quadrant::TopLeft => dx <= 0 && dy <= 0,
                Quadrant::TopRight => dx >= 0 && dy <= 0,
                Quadrant::BottomLeft => dx <= 0 && dy >= 0,
                Quadrant::BottomRight => dx >= 0 && dy >= 0,
            };
            if allowed {
                put_pixel(image, center_x + dx, center_y + dy, color);
            }
        }
    }
}

fn draw_text(image: &mut RgbaImage, x: i32, y: i32, text: &str, color: Rgba<u8>) {
    let mut cursor_x = x;
    for character in text.chars() {
        draw_glyph(image, cursor_x, y, character, color);
        cursor_x += LABEL_FONT_ADVANCE;
    }
}

fn draw_glyph(image: &mut RgbaImage, x: i32, y: i32, character: char, color: Rgba<u8>) {
    let glyph = glyph_bitmap(character);

    for (row, bits) in glyph.iter().enumerate() {
        for column in 0..LABEL_FONT_WIDTH {
            if bits & (1 << (LABEL_FONT_WIDTH - 1 - column)) != 0 {
                put_pixel(image, x + column, y + row as i32, color);
            }
        }
    }
}

fn glyph_bitmap(character: char) -> [u8; LABEL_FONT_HEIGHT as usize] {
    match character {
        '0' => [0x1E, 0x21, 0x21, 0x23, 0x25, 0x29, 0x31, 0x21, 0x21, 0x1E],
        '1' => [0x08, 0x18, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x1C],
        '2' => [0x1E, 0x21, 0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x3F],
        '3' => [0x1E, 0x21, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x01, 0x21, 0x1E],
        '4' => [0x06, 0x0A, 0x12, 0x22, 0x3F, 0x02, 0x02, 0x02, 0x02, 0x02],
        '5' => [0x3F, 0x20, 0x20, 0x20, 0x1E, 0x01, 0x01, 0x01, 0x21, 0x1E],
        '6' => [0x0E, 0x10, 0x20, 0x20, 0x3E, 0x21, 0x21, 0x21, 0x21, 0x1E],
        '7' => [0x3F, 0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10, 0x10, 0x10],
        '8' => [0x1E, 0x21, 0x21, 0x21, 0x1E, 0x21, 0x21, 0x21, 0x21, 0x1E],
        '9' => [0x1E, 0x21, 0x21, 0x21, 0x1F, 0x01, 0x01, 0x01, 0x02, 0x1C],
        ':' => [0x00, 0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x0C, 0x00, 0x00],
        _ => [0x00; LABEL_FONT_HEIGHT as usize],
    }
}

fn text_width(text: &str) -> i32 {
    let glyph_count = text.chars().count() as i32;
    if glyph_count == 0 {
        0
    } else {
        glyph_count * LABEL_FONT_ADVANCE - LABEL_FONT_TRACKING
    }
}

fn seconds_to_string(seconds: i32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn round_up_to_nearest(value: f64, multiple: i32) -> i32 {
    if multiple == 0 {
        return 0;
    }
    let rounded_up = value.ceil() as i32;
    ((rounded_up + multiple - 1) / multiple) * multiple
}

fn seconds_to_pixels(waveform: &Waveform, seconds: f64) -> usize {
    (seconds * waveform.sample_rate() as f64 / waveform.samples_per_pixel() as f64) as usize
}

fn scale_sample(value: i16, multiplier: f64) -> i16 {
    (f64::from(value) * multiplier).clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

fn put_pixel(image: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>) {
    if x >= 0 && y >= 0 && x < image.width() as i32 && y < image.height() as i32 {
        image.put_pixel(x as u32, y as u32, color);
    }
}

fn rgba(color: Color) -> Rgba<u8> {
    Rgba([color.red, color.green, color.blue, color.alpha])
}

const LABEL_FONT_WIDTH: i32 = 6;
const LABEL_FONT_HEIGHT: i32 = 10;
const LABEL_FONT_TRACKING: i32 = 1;
const LABEL_FONT_ADVANCE: i32 = LABEL_FONT_WIDTH + LABEL_FONT_TRACKING;

#[cfg(test)]
mod tests {
    use super::{
        LABEL_FONT_HEIGHT, RenderOptions, RenderStyle, glyph_bitmap, render_waveform,
        round_up_to_nearest, seconds_to_string,
    };
    use crate::{AmplitudeScale, Waveform, WaveformColors, WaveformPoint};

    fn sample_waveform() -> Waveform {
        let mut waveform = Waveform::new(48_000, 64, 1).expect("waveform");
        waveform
            .push_frame(&[WaveformPoint {
                min: -16_384,
                max: 16_384,
            }])
            .expect("push point");
        waveform
    }

    #[test]
    fn formats_time_axis_labels() {
        assert_eq!(seconds_to_string(5), "00:05");
        assert_eq!(seconds_to_string(125), "02:05");
        assert_eq!(seconds_to_string(3_665), "01:01:05");
        assert_eq!(round_up_to_nearest(0.0, 5), 0);
        assert_eq!(round_up_to_nearest(0.1, 5), 5);
        assert_eq!(round_up_to_nearest(12.3, 10), 20);
    }

    #[test]
    fn one_glyph_uses_a_thin_stem() {
        assert_eq!(
            glyph_bitmap('1'),
            [0x08, 0x18, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x1C]
        );
    }

    #[test]
    fn glyph_fallback_and_metrics_are_stable() {
        assert_eq!(glyph_bitmap('?'), [0x00; LABEL_FONT_HEIGHT as usize]);
    }

    #[test]
    fn validates_render_options_and_waveform_state() {
        let waveform = sample_waveform();

        let error = render_waveform(
            &waveform,
            &RenderOptions {
                width: 0,
                ..RenderOptions::default()
            },
        )
        .expect_err("invalid width");
        assert_eq!(error.to_string(), "Invalid image width: minimum 1");

        let error = render_waveform(
            &waveform,
            &RenderOptions {
                height: 0,
                ..RenderOptions::default()
            },
        )
        .expect_err("invalid height");
        assert_eq!(error.to_string(), "Invalid image height: minimum 1");

        let error = render_waveform(
            &waveform,
            &RenderOptions {
                start_time: -0.1,
                ..RenderOptions::default()
            },
        )
        .expect_err("invalid start time");
        assert_eq!(error.to_string(), "Invalid start time: minimum 0");

        let error = render_waveform(
            &waveform,
            &RenderOptions {
                start_time: f64::INFINITY,
                ..RenderOptions::default()
            },
        )
        .expect_err("non-finite start time");
        assert_eq!(error.to_string(), "Invalid start time: minimum 0");

        let error = render_waveform(
            &Waveform::new(48_000, 64, 1).expect("empty waveform"),
            &RenderOptions::default(),
        )
        .expect_err("empty waveform");
        assert_eq!(error.to_string(), "Empty waveform buffer");
    }

    #[test]
    fn validates_bar_rendering_and_amplitude_ranges() {
        let waveform = sample_waveform();

        let error = render_waveform(
            &waveform,
            &RenderOptions {
                style: RenderStyle::Bars {
                    width: 0,
                    gap: 4,
                    style: super::BarStyle::Square,
                },
                ..RenderOptions::default()
            },
        )
        .expect_err("invalid bar width");
        assert_eq!(error.to_string(), "Invalid bar width: minimum 1");

        let error = render_waveform(
            &waveform,
            &RenderOptions {
                amplitude_scale: AmplitudeScale::Fixed(-1.0),
                ..RenderOptions::default()
            },
        )
        .expect_err("negative amplitude scale");
        assert_eq!(
            error.to_string(),
            "Invalid amplitude scale: must be a positive number"
        );

        let error = render_waveform(
            &waveform,
            &RenderOptions {
                amplitude_scale: AmplitudeScale::Fixed(f64::NAN),
                ..RenderOptions::default()
            },
        )
        .expect_err("non-finite amplitude scale");
        assert_eq!(
            error.to_string(),
            "Invalid amplitude scale: must be a positive number"
        );
    }

    #[test]
    fn renders_an_image_with_expected_dimensions() {
        let image = render_waveform(
            &sample_waveform(),
            &RenderOptions {
                width: 32,
                height: 20,
                colors: WaveformColors::default(),
                ..RenderOptions::default()
            },
        )
        .expect("render image");

        assert_eq!(image.dimensions(), (32, 20));
    }
}
