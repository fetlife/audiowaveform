use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;

use hound::{SampleFormat, WavSpec, WavWriter};

#[cfg(feature = "decode")]
use crate::audio::decode_audio_reader;
use crate::{AudioFormat, Error, PcmAudio};

/// Writes interleaved PCM audio as a 16-bit WAV stream.
pub fn write_pcm_as_wav<W: Write>(pcm: &PcmAudio, mut writer: W) -> Result<(), Error> {
    let spec = WavSpec {
        channels: pcm.channels(),
        sample_rate: pcm.sample_rate(),
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut bytes = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut bytes);
        let mut wav = WavWriter::new(cursor, spec)?;
        for sample in pcm.samples() {
            wav.write_sample(*sample)?;
        }
        wav.finalize()?;
    }
    writer.write_all(&bytes)?;
    Ok(())
}

/// Decodes audio from a path and writes it as a WAV file.
#[cfg(feature = "decode")]
pub fn transcode_audio_path_to_wav_path(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<(), Error> {
    let input = input.as_ref();
    let format = AudioFormat::from_path(input).ok_or_else(|| Error::UnsupportedFormat {
        format: input
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string(),
    })?;
    let file = File::open(input)?;
    let mut output_file = File::create(output)?;
    transcode_audio_reader_to_wav_writer(file, Some(format), &mut output_file)
}

/// Decodes audio from a seekable reader and writes it as a WAV stream.
#[cfg(feature = "decode")]
pub fn transcode_audio_reader_to_wav_writer<R: Read + Seek + Send + Sync + 'static, W: Write>(
    reader: R,
    format_hint: Option<AudioFormat>,
    writer: W,
) -> Result<(), Error> {
    let pcm = decode_audio_reader(reader, format_hint)?;
    write_pcm_as_wav(&pcm, writer)
}
