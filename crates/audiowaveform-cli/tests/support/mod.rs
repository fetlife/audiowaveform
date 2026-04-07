#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use hound::WavReader;
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

pub fn assert_wav_file_matches_fixture(
    actual: impl AsRef<Path>,
    fixture: &str,
    sample_tolerance: i16,
) {
    let actual = WavReader::open(actual).expect("open actual wav");
    let expected = WavReader::open(fixture_path(fixture)).expect("open expected wav");
    assert_wav_eq(actual, expected, fixture, sample_tolerance);
}

fn assert_wav_eq<R1: std::io::Read, R2: std::io::Read>(
    mut actual: WavReader<R1>,
    mut expected: WavReader<R2>,
    fixture: &str,
    sample_tolerance: i16,
) {
    assert_eq!(
        actual.spec(),
        expected.spec(),
        "wav spec mismatch for {fixture}"
    );
    assert_eq!(
        actual.duration(),
        expected.duration(),
        "wav duration mismatch for {fixture}"
    );

    let actual_samples = actual
        .samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .expect("read actual wav samples");
    let expected_samples = expected
        .samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .expect("read expected wav samples");

    assert_eq!(
        actual_samples.len(),
        expected_samples.len(),
        "wav sample count mismatch for {fixture}"
    );

    for (index, (actual, expected)) in actual_samples
        .iter()
        .zip(expected_samples.iter())
        .enumerate()
    {
        let difference = i32::from(*actual) - i32::from(*expected);
        assert!(
            difference.abs() <= i32::from(sample_tolerance),
            "wav sample mismatch for {fixture} at index {index}: actual={actual}, expected={expected}, tolerance={sample_tolerance}"
        );
    }
}
