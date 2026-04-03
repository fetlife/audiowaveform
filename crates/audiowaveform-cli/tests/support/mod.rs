#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use image::{RgbaImage, load_from_memory};
use tempfile::{Builder, NamedTempFile};

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

pub fn read_fixture(name: &str) -> Vec<u8> {
    fs::read(fixture_path(name)).expect("fixture bytes")
}

pub fn named_temp_file(suffix: &str) -> NamedTempFile {
    Builder::new()
        .suffix(suffix)
        .tempfile()
        .expect("create temp file")
}

pub fn assert_png_file_matches_fixture(actual: impl AsRef<Path>, fixture: &str) {
    let actual = image::open(actual).expect("open actual png").into_rgba8();
    let expected = image::open(fixture_path(fixture))
        .expect("open expected png")
        .into_rgba8();
    assert_png_eq(&actual, &expected, fixture);
}

pub fn assert_png_bytes_match_fixture(actual: &[u8], fixture: &str) {
    let actual = load_from_memory(actual)
        .expect("decode actual png")
        .into_rgba8();
    let expected = image::open(fixture_path(fixture))
        .expect("open expected png")
        .into_rgba8();
    assert_png_eq(&actual, &expected, fixture);
}

fn assert_png_eq(actual: &RgbaImage, expected: &RgbaImage, fixture: &str) {
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
