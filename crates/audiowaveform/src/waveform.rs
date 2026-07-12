use std::fs::File;
use std::io::{BufReader, BufWriter, ErrorKind, Read, Write};
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::Deserialize;

use crate::audio::ScaleSpec;
use crate::{Error, WaveformFormat};

const FLAG_8_BIT: u32 = 0x0000_0001;
const MAX_CHANNELS: usize = 24;

/// A single waveform point containing the minimum and maximum sample value for a bucket.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaveformPoint {
    /// Minimum sample value for the bucket.
    pub min: i16,
    /// Maximum sample value for the bucket.
    pub max: i16,
}

/// Waveform amplitude post-scaling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AmplitudeScale {
    /// Scale using a fixed multiplier.
    Fixed(f64),
    /// Scale automatically so the current range fills the full 16-bit output range.
    Auto,
}

/// Immutable-ish waveform value containing metadata and interleaved min/max data.
#[derive(Clone, Debug, PartialEq)]
pub struct Waveform {
    sample_rate: u32,
    samples_per_pixel: u32,
    channels: u16,
    storage_bits: u8,
    data: Vec<i16>,
}

#[derive(Debug, Deserialize)]
struct JsonWaveform {
    version: i32,
    channels: Option<u16>,
    sample_rate: u32,
    samples_per_pixel: u32,
    bits: u8,
    length: usize,
    data: Vec<i32>,
}

impl Waveform {
    /// Creates an empty waveform with the given metadata.
    ///
    /// ```rust
    /// use audiowaveform::Waveform;
    ///
    /// let waveform = Waveform::new(48_000, 256, 1)?;
    /// assert!(waveform.is_empty());
    /// # Ok::<(), audiowaveform::Error>(())
    /// ```
    pub fn new(sample_rate: u32, samples_per_pixel: u32, channels: u16) -> Result<Self, Error> {
        Self::validate_metadata(sample_rate, samples_per_pixel, channels)?;
        Ok(Self {
            sample_rate,
            samples_per_pixel,
            channels,
            storage_bits: 16,
            data: Vec::new(),
        })
    }

    /// Creates a waveform from already interleaved min/max samples.
    pub fn from_interleaved_samples(
        sample_rate: u32,
        samples_per_pixel: u32,
        channels: u16,
        data: Vec<i16>,
        storage_bits: u8,
    ) -> Result<Self, Error> {
        Self::validate_metadata(sample_rate, samples_per_pixel, channels)?;
        Self::validate_storage_bits(storage_bits)?;
        let frame_width = usize::from(channels) * 2;
        if !data.len().is_multiple_of(frame_width) {
            return Err(Error::invalid_data(
                "Waveform data length must be divisible by two samples per channel",
            ));
        }
        Ok(Self {
            sample_rate,
            samples_per_pixel,
            channels,
            storage_bits,
            data,
        })
    }

    /// Loads a waveform from a path, inferring the format from the extension when omitted.
    pub fn load_from_path(
        path: impl AsRef<Path>,
        format: Option<WaveformFormat>,
    ) -> Result<Self, Error> {
        let path = path.as_ref();
        let resolved = format
            .or_else(|| WaveformFormat::from_path(path))
            .ok_or_else(|| Error::UnsupportedFormat {
                format: path
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_string(),
            })?;
        let file = File::open(path)?;
        Self::load_from_reader(BufReader::new(file), resolved)
    }

    /// Loads a waveform from an arbitrary reader.
    pub fn load_from_reader<R: Read>(reader: R, format: WaveformFormat) -> Result<Self, Error> {
        match format {
            WaveformFormat::Dat => Self::read_dat(reader),
            WaveformFormat::Json => Self::read_json(reader),
            WaveformFormat::Txt => Err(Error::UnsupportedFormat {
                format: "txt".to_string(),
            }),
        }
    }

    /// Saves the waveform to a path, inferring the format from the extension when omitted.
    pub fn save_to_path(
        &self,
        path: impl AsRef<Path>,
        format: Option<WaveformFormat>,
    ) -> Result<(), Error> {
        let path = path.as_ref();
        let resolved = format
            .or_else(|| WaveformFormat::from_path(path))
            .ok_or_else(|| Error::UnsupportedFormat {
                format: path
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_string(),
            })?;
        let file = File::create(path)?;
        self.write_to_writer(BufWriter::new(file), resolved, None)
    }

    /// Writes the waveform to an arbitrary writer.
    pub fn write_to_writer<W: Write>(
        &self,
        writer: W,
        format: WaveformFormat,
        bits: Option<u8>,
    ) -> Result<(), Error> {
        let bits = bits.unwrap_or(self.storage_bits);
        Self::validate_storage_bits(bits)?;
        match format {
            WaveformFormat::Dat => self.write_dat(writer, bits),
            WaveformFormat::Json => self.write_json(writer, bits),
            WaveformFormat::Txt => self.write_txt(writer, bits),
        }
    }

    /// Returns the sample rate of the source audio.
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Returns the number of source samples represented by each waveform point.
    pub const fn samples_per_pixel(&self) -> u32 {
        self.samples_per_pixel
    }

    /// Returns the number of waveform channels.
    pub const fn channels(&self) -> u16 {
        self.channels
    }

    /// Returns the preferred serialized bit depth.
    pub const fn storage_bits(&self) -> u8 {
        self.storage_bits
    }

    /// Returns the number of waveform points per channel.
    pub fn len(&self) -> usize {
        self.data.len() / (usize::from(self.channels) * 2)
    }

    /// Returns `true` when the waveform contains no points.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the duration represented by the waveform in seconds.
    pub fn duration_seconds(&self) -> f64 {
        self.len() as f64 * self.samples_per_pixel as f64 / self.sample_rate as f64
    }

    /// Appends a frame containing one point per channel.
    pub fn push_frame(&mut self, points: &[WaveformPoint]) -> Result<(), Error> {
        if points.len() != usize::from(self.channels) {
            return Err(Error::invalid_argument(
                "points",
                format!(
                    "Expected {} points, received {}",
                    self.channels,
                    points.len()
                ),
            ));
        }
        for point in points {
            self.data.push(point.min);
            self.data.push(point.max);
        }
        Ok(())
    }

    /// Returns the waveform point at `index` for the given `channel`.
    pub fn point(&self, channel: u16, index: usize) -> Option<WaveformPoint> {
        if channel >= self.channels || index >= self.len() {
            return None;
        }
        let offset = self.offset(channel, index);
        Some(WaveformPoint {
            min: self.data[offset],
            max: self.data[offset + 1],
        })
    }

    /// Returns the interleaved internal min/max data.
    pub fn interleaved_samples(&self) -> &[i16] {
        &self.data
    }

    /// Returns a copy of the waveform with its serialized bit-depth preference changed.
    pub fn with_storage_bits(mut self, bits: u8) -> Result<Self, Error> {
        Self::validate_storage_bits(bits)?;
        self.storage_bits = bits;
        Ok(self)
    }

    /// Returns the amplitude scale needed to fill the 16-bit range over the provided region.
    pub fn auto_amplitude_scale(&self, start_index: usize, end_index: usize) -> Result<f64, Error> {
        if start_index >= end_index || end_index > self.len() {
            return Err(Error::invalid_argument(
                "amplitude range",
                "Invalid amplitude scaling range",
            ));
        }

        let mut low = i32::MAX;
        let mut high = i32::MIN;
        for index in start_index..end_index {
            for channel in 0..self.channels {
                let point = self.point(channel, index).expect("point bounds checked");
                low = low.min(i32::from(point.min));
                high = high.max(i32::from(point.max));
            }
        }

        let high_scale = if high == 0 {
            1.0
        } else {
            32767.0 / f64::from(high)
        };
        let low_scale = if low == 0 {
            1.0
        } else {
            32767.0 / f64::from(low)
        };

        Ok(high_scale.min(low_scale).abs())
    }

    /// Scales all waveform points by the provided amplitude strategy.
    pub fn scale_amplitude(&self, scale: AmplitudeScale) -> Result<Self, Error> {
        let multiplier = match scale {
            AmplitudeScale::Fixed(multiplier) => {
                if multiplier < 0.0 {
                    return Err(Error::invalid_argument(
                        "amplitude scale",
                        "Invalid amplitude scale: must be a positive number",
                    ));
                }
                multiplier
            }
            AmplitudeScale::Auto if self.is_empty() => 1.0,
            AmplitudeScale::Auto => self.auto_amplitude_scale(0, self.len())?,
        };

        let mut scaled = self.clone();
        for sample in &mut scaled.data {
            *sample = clamp_scaled(i32::from(*sample), multiplier);
        }
        Ok(scaled)
    }

    /// Resamples the waveform to a coarser scale.
    ///
    /// The resulting waveform always has the same sample rate and channel count
    /// as the original.
    pub fn resample(&self, scale: ScaleSpec) -> Result<Self, Error> {
        let total_frames = self.len() * self.samples_per_pixel as usize;
        let output_samples_per_pixel = scale.resolve(self.sample_rate, total_frames)?;
        if output_samples_per_pixel == self.samples_per_pixel {
            return Ok(self.clone());
        }
        if output_samples_per_pixel < self.samples_per_pixel {
            return Err(Error::invalid_argument(
                "zoom",
                format!("Invalid zoom, minimum: {}", self.samples_per_pixel),
            ));
        }

        let input_samples_per_pixel = self.samples_per_pixel;
        let mut output = Self::new(self.sample_rate, output_samples_per_pixel, self.channels)?;
        output.storage_bits = self.storage_bits;

        let channels = usize::from(self.channels);
        let mut min = vec![0_i16; channels];
        let mut max = vec![0_i16; channels];
        if !self.is_empty() {
            for channel in 0..self.channels {
                let point = self.point(channel, 0).expect("point within bounds");
                min[channel as usize] = point.min;
                max[channel as usize] = point.max;
            }
        }

        let mut input_index = 0_usize;
        let mut output_index = 0_usize;
        let mut last_input_index = 0_usize;

        while input_index < self.len() {
            while sample_at_pixel(output_index, output_samples_per_pixel)
                / input_samples_per_pixel as usize
                == input_index
            {
                if output_index > 0 {
                    flush_resampled_frame(&mut output, &min, &max)?;
                }
                last_input_index = input_index;
                output_index += 1;

                let current = sample_at_pixel(output_index, output_samples_per_pixel);
                let previous = sample_at_pixel(output_index - 1, output_samples_per_pixel);
                if current != previous {
                    min.fill(i16::MAX);
                    max.fill(i16::MIN);
                }
            }

            let mut stop = sample_at_pixel(output_index, output_samples_per_pixel)
                / input_samples_per_pixel as usize;
            stop = stop.min(self.len());
            while input_index < stop {
                for channel in 0..self.channels {
                    let point = self
                        .point(channel, input_index)
                        .expect("point within bounds");
                    let channel_index = channel as usize;
                    if point.min < min[channel_index] {
                        min[channel_index] = point.min;
                    }
                    if point.max > max[channel_index] {
                        max[channel_index] = point.max;
                    }
                }
                input_index += 1;
            }
        }

        if input_index != last_input_index {
            flush_resampled_frame(&mut output, &min, &max)?;
        }

        Ok(output)
    }

    fn validate_metadata(
        sample_rate: u32,
        samples_per_pixel: u32,
        channels: u16,
    ) -> Result<(), Error> {
        if sample_rate == 0 {
            return Err(Error::invalid_argument(
                "sample rate",
                "Invalid sample rate: minimum 1 Hz",
            ));
        }
        if samples_per_pixel < 2 {
            return Err(Error::invalid_argument(
                "samples per pixel",
                "Invalid samples per pixel: minimum 2",
            ));
        }
        if channels == 0 || usize::from(channels) > MAX_CHANNELS {
            return Err(Error::invalid_argument(
                "channels",
                format!("Invalid channels: must be between 1 and {MAX_CHANNELS}"),
            ));
        }
        Ok(())
    }

    fn validate_storage_bits(bits: u8) -> Result<(), Error> {
        if bits != 8 && bits != 16 {
            return Err(Error::invalid_argument(
                "bits",
                "Invalid bits: must be either 8 or 16",
            ));
        }
        Ok(())
    }

    fn offset(&self, channel: u16, index: usize) -> usize {
        (index * usize::from(self.channels) + usize::from(channel)) * 2
    }

    fn read_dat<R: Read>(mut reader: R) -> Result<Self, Error> {
        let version = reader.read_i32::<LittleEndian>()?;
        if version != 1 && version != 2 {
            return Err(Error::invalid_data(format!(
                "Cannot load data file version: {version}"
            )));
        }
        let flags = reader.read_u32::<LittleEndian>()?;
        let sample_rate = reader.read_u32::<LittleEndian>()?;
        let samples_per_pixel = reader.read_u32::<LittleEndian>()?;
        let length = reader.read_u32::<LittleEndian>()? as usize;
        let channels = if version == 2 {
            reader.read_u32::<LittleEndian>()? as u16
        } else {
            1
        };
        Self::validate_metadata(sample_rate, samples_per_pixel, channels)?;

        let bits = if flags & FLAG_8_BIT != 0 { 8 } else { 16 };
        let mut data = Vec::with_capacity(length * usize::from(channels) * 2);
        if bits == 8 {
            for _ in 0..length * usize::from(channels) {
                let Some(min_value) = read_optional_i8(&mut reader)? else {
                    break;
                };
                let Some(max_value) = read_optional_i8(&mut reader)? else {
                    break;
                };
                data.push(i16::from(min_value) * 256);
                data.push(i16::from(max_value) * 256);
            }
        } else {
            for _ in 0..length * usize::from(channels) * 2 {
                let Some(value) = read_optional_i16(&mut reader)? else {
                    break;
                };
                data.push(value);
            }
        }
        Self::from_interleaved_samples(sample_rate, samples_per_pixel, channels, data, bits)
    }

    fn read_json<R: Read>(reader: R) -> Result<Self, Error> {
        let json: JsonWaveform = serde_json::from_reader(reader)?;
        if json.version != 1 && json.version != 2 {
            return Err(Error::invalid_data("Invalid version: expecting 1 or 2"));
        }
        let channels = json.channels.unwrap_or(1);
        Self::validate_metadata(json.sample_rate, json.samples_per_pixel, channels)?;
        Self::validate_storage_bits(json.bits)?;

        let expected = json
            .length
            .checked_mul(usize::from(channels))
            .and_then(|value| value.checked_mul(2))
            .ok_or_else(|| Error::invalid_data("Waveform length is too large"))?;
        if json.data.len() != expected {
            return Err(Error::invalid_data(format!(
                "Length mismatch: expected {expected} values, found {}",
                json.data.len()
            )));
        }

        let mut data = Vec::with_capacity(expected);
        if json.bits == 8 {
            for value in json.data {
                if !(-128..=127).contains(&value) {
                    return Err(Error::invalid_data(format!(
                        "Data value out of range: {value}"
                    )));
                }
                data.push((value as i16) * 256);
            }
        } else {
            for value in json.data {
                if !(-32768..=32767).contains(&value) {
                    return Err(Error::invalid_data(format!(
                        "Data value out of range: {value}"
                    )));
                }
                data.push(value as i16);
            }
        }

        Self::from_interleaved_samples(
            json.sample_rate,
            json.samples_per_pixel,
            channels,
            data,
            json.bits,
        )
    }

    fn write_dat<W: Write>(&self, mut writer: W, bits: u8) -> Result<(), Error> {
        let version = if self.channels == 1 { 1_i32 } else { 2_i32 };
        writer.write_i32::<LittleEndian>(version)?;
        let flags = if bits == 8 { FLAG_8_BIT } else { 0 };
        writer.write_u32::<LittleEndian>(flags)?;
        writer.write_u32::<LittleEndian>(self.sample_rate)?;
        writer.write_u32::<LittleEndian>(self.samples_per_pixel)?;
        writer.write_u32::<LittleEndian>(self.len() as u32)?;
        if version == 2 {
            writer.write_u32::<LittleEndian>(u32::from(self.channels))?;
        }
        if bits == 8 {
            for value in &self.data {
                writer.write_i8((value / 256) as i8)?;
            }
        } else {
            for value in &self.data {
                writer.write_i16::<LittleEndian>(*value)?;
            }
        }
        Ok(())
    }

    fn write_txt<W: Write>(&self, mut writer: W, bits: u8) -> Result<(), Error> {
        for index in 0..self.len() {
            for channel in 0..self.channels {
                if channel > 0 {
                    writer.write_all(b",")?;
                }
                let point = self.point(channel, index).expect("point within range");
                if bits == 8 {
                    write!(writer, "{},{}", point.min / 256, point.max / 256)?;
                } else {
                    write!(writer, "{},{}", point.min, point.max)?;
                }
            }
            writer.write_all(b"\n")?;
        }
        Ok(())
    }

    fn write_json<W: Write>(&self, mut writer: W, bits: u8) -> Result<(), Error> {
        write!(
            writer,
            "{{\"version\":2,\"channels\":{},\"sample_rate\":{},\"samples_per_pixel\":{},\"bits\":{},\"length\":{},\"data\":[",
            self.channels,
            self.sample_rate,
            self.samples_per_pixel,
            bits,
            self.len()
        )?;
        for (index, value) in self.data.iter().enumerate() {
            if index > 0 {
                writer.write_all(b",")?;
            }
            let serialized = if bits == 8 { value / 256 } else { *value };
            write!(writer, "{serialized}")?;
        }
        writer.write_all(b"]}\n")?;
        Ok(())
    }
}

fn clamp_scaled(value: i32, multiplier: f64) -> i16 {
    let scaled = (f64::from(value) * multiplier).clamp(f64::from(i16::MIN), f64::from(i16::MAX));
    scaled as i16
}

fn read_optional_i8<R: Read>(reader: &mut R) -> Result<Option<i8>, Error> {
    match reader.read_i8() {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == ErrorKind::UnexpectedEof => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn read_optional_i16<R: Read>(reader: &mut R) -> Result<Option<i16>, Error> {
    match reader.read_i16::<LittleEndian>() {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == ErrorKind::UnexpectedEof => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn sample_at_pixel(index: usize, samples_per_pixel: u32) -> usize {
    index * samples_per_pixel as usize
}

fn flush_resampled_frame(waveform: &mut Waveform, min: &[i16], max: &[i16]) -> Result<(), Error> {
    let points = min
        .iter()
        .zip(max.iter())
        .map(|(min, max)| WaveformPoint {
            min: *min,
            max: *max,
        })
        .collect::<Vec<_>>();
    waveform.push_frame(&points)
}

#[cfg(test)]
mod tests {
    use super::{AmplitudeScale, Waveform, WaveformFormat, WaveformPoint};
    use crate::audio::ScaleSpec;

    fn sample_waveform() -> Waveform {
        let mut waveform = Waveform::new(48_000, 64, 1).expect("waveform");
        waveform
            .push_frame(&[WaveformPoint { min: -10, max: 20 }])
            .expect("first frame");
        waveform
            .push_frame(&[WaveformPoint { min: -30, max: 40 }])
            .expect("second frame");
        waveform
    }

    #[test]
    fn validates_waveform_metadata_and_frame_shapes() {
        let error = Waveform::new(0, 64, 1).expect_err("invalid sample rate");
        assert_eq!(error.to_string(), "Invalid sample rate: minimum 1 Hz");

        let error = Waveform::new(48_000, 1, 1).expect_err("invalid scale");
        assert_eq!(error.to_string(), "Invalid samples per pixel: minimum 2");

        let error = Waveform::new(48_000, 64, 25).expect_err("invalid channels");
        assert_eq!(
            error.to_string(),
            "Invalid channels: must be between 1 and 24"
        );

        let error = Waveform::from_interleaved_samples(48_000, 64, 2, vec![1, 2, 3], 16)
            .expect_err("unaligned waveform data");
        assert_eq!(
            error.to_string(),
            "Waveform data length must be divisible by two samples per channel"
        );

        let mut waveform = Waveform::new(48_000, 64, 2).expect("waveform");
        let error = waveform
            .push_frame(&[WaveformPoint { min: 0, max: 1 }])
            .expect_err("wrong frame width");
        assert_eq!(error.to_string(), "Expected 2 points, received 1");
    }

    #[test]
    fn scales_waveform_amplitude_using_fixed_and_auto_modes() {
        let waveform = sample_waveform();

        let fixed = waveform
            .scale_amplitude(AmplitudeScale::Fixed(2.0))
            .expect("scale fixed");
        assert_eq!(
            fixed.point(0, 0).expect("scaled point"),
            WaveformPoint { min: -20, max: 40 }
        );

        let auto = waveform
            .scale_amplitude(AmplitudeScale::Auto)
            .expect("scale auto");
        assert_eq!(
            auto.point(0, 1).expect("auto point"),
            WaveformPoint {
                min: -32_767,
                max: 32_767,
            }
        );

        let error = waveform
            .scale_amplitude(AmplitudeScale::Fixed(-1.0))
            .expect_err("negative scale");
        assert_eq!(
            error.to_string(),
            "Invalid amplitude scale: must be a positive number"
        );
    }

    #[test]
    fn resamples_waveforms_to_a_coarser_scale() {
        let mut waveform = Waveform::new(48_000, 512, 1).expect("waveform");
        for point in [
            WaveformPoint { min: 0, max: 0 },
            WaveformPoint { min: -10, max: 10 },
            WaveformPoint { min: 0, max: 0 },
            WaveformPoint { min: -5, max: 7 },
            WaveformPoint { min: -5, max: 7 },
            WaveformPoint { min: 0, max: 0 },
            WaveformPoint { min: 0, max: 0 },
            WaveformPoint { min: 0, max: 0 },
            WaveformPoint { min: 0, max: 0 },
            WaveformPoint { min: -2, max: 2 },
        ] {
            waveform.push_frame(&[point]).expect("push point");
        }

        let resampled = waveform
            .resample(ScaleSpec::SamplesPerPixel(1024))
            .expect("resample waveform");
        assert_eq!(resampled.len(), 5);
        assert_eq!(resampled.samples_per_pixel(), 1024);
        assert_eq!(
            resampled.point(0, 0).expect("point 0"),
            WaveformPoint { min: -10, max: 10 }
        );
        assert_eq!(
            resampled.point(0, 1).expect("point 1"),
            WaveformPoint { min: -5, max: 7 }
        );
        assert_eq!(
            resampled.point(0, 2).expect("point 2"),
            WaveformPoint { min: -5, max: 7 }
        );
        assert_eq!(
            resampled.point(0, 3).expect("point 3"),
            WaveformPoint { min: 0, max: 0 }
        );
        assert_eq!(
            resampled.point(0, 4).expect("point 4"),
            WaveformPoint { min: -2, max: 2 }
        );

        let error = waveform
            .resample(ScaleSpec::SamplesPerPixel(256))
            .expect_err("finer zoom should fail");
        assert_eq!(error.to_string(), "Invalid zoom, minimum: 512");
    }

    #[test]
    fn serializes_text_and_json_for_two_channel_waveforms() {
        let waveform = Waveform::from_interleaved_samples(
            44_100,
            256,
            2,
            vec![-1024, 1024, -2048, 2048, -3072, 3072, -4096, 4096],
            16,
        )
        .expect("waveform");

        let mut txt = Vec::new();
        waveform
            .write_to_writer(&mut txt, WaveformFormat::Txt, Some(8))
            .expect("write txt");
        assert_eq!(
            String::from_utf8(txt).expect("utf8"),
            "-4,4,-8,8\n-12,12,-16,16\n"
        );

        let mut json = Vec::new();
        waveform
            .write_to_writer(&mut json, WaveformFormat::Json, Some(16))
            .expect("write json");
        assert_eq!(
            String::from_utf8(json).expect("utf8"),
            "{\"version\":2,\"channels\":2,\"sample_rate\":44100,\"samples_per_pixel\":256,\"bits\":16,\"length\":2,\"data\":[-1024,1024,-2048,2048,-3072,3072,-4096,4096]}\n"
        );
    }
}
