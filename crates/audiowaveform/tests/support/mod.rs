#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use audiowaveform::Waveform;
use image::{RgbaImage, load_from_memory};
use tempfile::{Builder, NamedTempFile};

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/data")
        .join(name)
}

pub fn rust_png_fixture_path(name: &str) -> PathBuf {
    fixture_path("rust").join(name)
}

pub fn read_fixture(name: &str) -> Vec<u8> {
    fs::read(fixture_path(name)).expect("fixture bytes")
}

pub fn load_waveform(name: &str) -> Waveform {
    Waveform::load_from_path(fixture_path(name), None).expect("load waveform fixture")
}

pub fn named_temp_file(suffix: &str) -> NamedTempFile {
    Builder::new()
        .suffix(suffix)
        .tempfile()
        .expect("create temp file")
}

pub fn assert_bytes_eq(actual: &[u8], fixture: &str) {
    assert_eq!(actual, read_fixture(fixture), "fixture mismatch: {fixture}");
}

pub fn assert_file_bytes_eq(actual: impl AsRef<Path>, fixture: &str) {
    let actual = fs::read(actual).expect("read output file");
    assert_bytes_eq(&actual, fixture);
}

pub fn assert_png_bytes_match_fixture(actual: &[u8], fixture: &str) {
    let actual = load_from_memory(actual)
        .expect("decode actual png")
        .into_rgba8();
    assert_png_image_matches_fixture(&actual, fixture);
}

pub fn assert_png_file_matches_fixture(actual: impl AsRef<Path>, fixture: &str) {
    let actual = image::open(actual).expect("open actual png").into_rgba8();
    assert_png_image_matches_fixture(&actual, fixture);
}

pub fn assert_png_image_matches_fixture(actual: &RgbaImage, fixture: &str) {
    let expected = image::open(rust_png_fixture_path(fixture))
        .expect("open expected png")
        .into_rgba8();
    assert_eq!(
        actual.dimensions(),
        expected.dimensions(),
        "png dimensions mismatch for {fixture}"
    );
    assert_eq!(
        actual.as_raw(),
        expected.as_raw(),
        "png pixels mismatch for {fixture}"
    );
}
