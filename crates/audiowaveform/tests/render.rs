use std::path::PathBuf;

use audiowaveform::{RenderOptions, Waveform, render_waveform};

fn data_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/data")
        .join(name)
}

#[test]
fn renders_waveform_image_with_expected_dimensions() {
    let waveform = Waveform::load_from_path(data_path("test_file_stereo_8bit_64spp_wav.dat"), None)
        .expect("load waveform");
    let image = render_waveform(
        &waveform,
        &RenderOptions {
            width: 1000,
            height: 300,
            start_time: 5.0,
            ..RenderOptions::default()
        },
    )
    .expect("render image");

    assert_eq!(image.width(), 1000);
    assert_eq!(image.height(), 300);
}
