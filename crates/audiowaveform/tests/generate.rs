use std::fs;
use std::path::PathBuf;

use audiowaveform::{GenerateOptions, ScaleSpec, WaveformFormat, generate_waveform_from_path};

fn data_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/data")
        .join(name)
}

#[test]
fn generates_expected_binary_waveform_from_wav() {
    let waveform = generate_waveform_from_path(
        data_path("test_file_stereo.wav"),
        &GenerateOptions {
            scale: ScaleSpec::SamplesPerPixel(64),
            split_channels: false,
            amplitude_scale: None,
        },
    )
    .expect("generate waveform");

    let mut output = Vec::new();
    waveform
        .write_to_writer(&mut output, WaveformFormat::Dat, Some(8))
        .expect("write dat");

    assert_eq!(
        output,
        fs::read(data_path("test_file_stereo_8bit_64spp_wav.dat")).expect("fixture bytes"),
    );
}
