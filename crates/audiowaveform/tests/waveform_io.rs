use std::fs;
use std::path::PathBuf;

use audiowaveform::{Waveform, WaveformFormat};

fn data_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/data")
        .join(name)
}

#[test]
fn round_trips_binary_waveform_fixture() {
    let fixture = data_path("test_file_stereo_8bit_64spp_wav.dat");
    let waveform = Waveform::load_from_path(&fixture, None).expect("load waveform");

    assert_eq!(waveform.sample_rate(), 16_000);
    assert_eq!(waveform.samples_per_pixel(), 64);
    assert_eq!(waveform.channels(), 1);
    assert_eq!(waveform.storage_bits(), 8);
    assert_eq!(waveform.len(), 1774);

    let mut output = Vec::new();
    waveform
        .write_to_writer(&mut output, WaveformFormat::Dat, None)
        .expect("write dat");

    assert_eq!(output, fs::read(fixture).expect("fixture bytes"));
}

#[test]
fn emits_expected_json_and_text_fixture_bytes() {
    let fixture = data_path("test_file_stereo_8bit_64spp_wav.dat");
    let waveform = Waveform::load_from_path(&fixture, None).expect("load waveform");

    let mut json = Vec::new();
    waveform
        .write_to_writer(&mut json, WaveformFormat::Json, Some(8))
        .expect("write json");
    assert_eq!(
        json,
        fs::read(data_path("test_file_stereo_8bit_64spp_wav.json")).expect("json fixture"),
    );

    let mut txt = Vec::new();
    waveform
        .write_to_writer(&mut txt, WaveformFormat::Txt, Some(8))
        .expect("write txt");
    assert_eq!(
        txt,
        fs::read(data_path("test_file_stereo_8bit_64spp_wav.txt")).expect("txt fixture"),
    );
}
