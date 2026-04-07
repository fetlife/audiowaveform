#[cfg(feature = "decode")]
use std::fs::File;
use std::io::Write;
#[cfg(feature = "decode")]
use std::io::{Read, Seek};
#[cfg(feature = "decode")]
use std::path::Path;

use hound::{SampleFormat, WavSpec, WavWriter};

#[cfg(feature = "decode")]
use crate::AudioFormat;
#[cfg(feature = "decode")]
use crate::audio::decode_audio_reader;
use crate::{Error, PcmAudio};

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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::write_pcm_as_wav;
    use crate::PcmAudio;

    #[test]
    fn writes_pcm_audio_as_valid_wav_bytes() {
        let pcm = PcmAudio::new(44_100, 1, vec![0, 1, -1, 2, -2]).expect("pcm");
        let mut wav = Vec::new();
        write_pcm_as_wav(&pcm, &mut wav).expect("write wav");

        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");

        let mut reader = hound::WavReader::new(Cursor::new(wav)).expect("read wav");
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().sample_rate, 44_100);
        assert_eq!(
            reader
                .samples::<i16>()
                .collect::<Result<Vec<_>, _>>()
                .expect("samples"),
            vec![0, 1, -1, 2, -2]
        );
    }
}
