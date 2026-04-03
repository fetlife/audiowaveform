#![cfg(feature = "wav")]

mod support;

use std::io::Cursor;

use audiowaveform::{PcmAudio, write_pcm_as_wav};

#[cfg(feature = "decode")]
use self::support::{assert_bytes_eq, fixture_path, named_temp_file};

#[test]
fn writes_empty_wav_header_for_empty_pcm() {
    let pcm = PcmAudio::new(44_100, 1, Vec::new()).expect("pcm");
    let mut wav = Vec::new();
    write_pcm_as_wav(&pcm, &mut wav).expect("write wav");

    assert_eq!(wav.len(), 44);
}

#[test]
fn writes_expected_pcm_wav_structure() {
    let pcm = PcmAudio::new(44_100, 1, vec![0; 1024]).expect("pcm");
    let mut wav = Vec::new();
    write_pcm_as_wav(&pcm, &mut wav).expect("write wav");

    assert_eq!(wav.len(), 44 + 1024 * 2);

    let mut reader = hound::WavReader::new(Cursor::new(wav)).expect("open wav");
    assert_eq!(reader.spec().channels, 1);
    assert_eq!(reader.spec().sample_rate, 44_100);
    assert_eq!(reader.spec().bits_per_sample, 16);
    assert_eq!(reader.duration(), 1024);
    assert!(
        reader
            .samples::<i16>()
            .all(|sample| sample.expect("sample") == 0)
    );
}

#[cfg(feature = "decode")]
#[test]
fn transcodes_audio_to_expected_wav_fixture() {
    let output = named_temp_file(".wav");
    audiowaveform::transcode_audio_path_to_wav_path(
        fixture_path("test_file_mono.mp3"),
        output.path(),
    )
    .expect("transcode wav");

    assert_bytes_eq(
        &std::fs::read(output.path()).expect("read wav"),
        "rust/test_file_mono_converted.wav",
    );
}
