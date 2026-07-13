mod support;

use assert_cmd::Command;
use audiowaveform::Waveform;
use predicates::prelude::*;

use self::support::{
    assert_png_bytes_match_fixture, assert_png_file_matches_fixture,
    assert_wav_file_matches_fixture, fixture_path, named_temp_file, read_fixture,
};

#[test]
fn prints_help_and_version() {
    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: audiowaveform [OPTIONS]"))
        .stdout(predicate::str::contains(
            "Generate waveform data and images from audio",
        ));

    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("AudioWaveform v"));
}

#[test]
fn requires_input_and_output_configuration() {
    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .assert()
        .failure()
        .stderr("Error: Must specify either input filename or input format\n");

    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args(["--input-format", "wav"])
        .assert()
        .failure()
        .stderr("Error: Must specify either output filename or output format\n");
}

#[test]
fn rejects_invalid_enum_values_via_clap() {
    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args(["--colors", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'test'"))
        .stderr(predicate::str::contains("possible values"));
}

#[test]
fn rejects_non_finite_numeric_values() {
    let input = fixture_path("test_file_stereo_8bit_64spp_wav.dat");
    let input = input.to_str().expect("utf8");

    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args([
            "-q",
            "-i",
            input,
            "--output-format",
            "png",
            "-z",
            "64",
            "--start",
            "inf",
        ])
        .assert()
        .failure()
        .stderr("Invalid start time: minimum 0\n");

    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args([
            "-q",
            "-i",
            input,
            "--output-format",
            "png",
            "-z",
            "64",
            "--amplitude-scale",
            "NaN",
        ])
        .assert()
        .failure()
        .stderr("Error: Invalid amplitude scale: must be a positive number\n");
}

#[test]
fn rejects_raw_channel_counts_that_do_not_fit_the_library_type() {
    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args([
            "-q",
            "--input-format",
            "raw",
            "--output-format",
            "wav",
            "--raw-samplerate",
            "48000",
            "--raw-channels",
            "65537",
            "--raw-format",
            "s16le",
        ])
        .assert()
        .failure()
        .stderr("Invalid number of input channels: maximum 65535\n");
}

#[test]
fn generates_dat_output_to_file_and_stdout() {
    let output = named_temp_file(".dat");
    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args([
            "-i",
            fixture_path("test_file_stereo.wav").to_str().expect("utf8"),
            "-o",
            output.path().to_str().expect("utf8"),
            "-b",
            "8",
            "-z",
            "64",
        ])
        .assert()
        .success()
        .stderr("Done\n");
    assert_eq!(
        std::fs::read(output.path()).expect("read output"),
        read_fixture("test_file_stereo_8bit_64spp_wav.dat")
    );

    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args([
            "--input-format",
            "wav",
            "--output-format",
            "dat",
            "-b",
            "8",
            "-z",
            "64",
        ])
        .write_stdin(read_fixture("test_file_stereo.wav"))
        .assert()
        .success()
        .stdout(read_fixture("test_file_stereo_8bit_64spp_wav.dat"))
        .stderr("Done\n");
}

#[test]
fn generates_json_and_text_outputs_to_stdout() {
    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args([
            "--input-format",
            "wav",
            "--output-format",
            "json",
            "-b",
            "8",
            "-z",
            "64",
        ])
        .write_stdin(read_fixture("test_file_stereo.wav"))
        .assert()
        .success()
        .stdout(read_fixture("test_file_stereo_8bit_64spp_wav.json"))
        .stderr("Done\n");

    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args([
            "-i",
            fixture_path("test_file_stereo_8bit_64spp_wav.dat")
                .to_str()
                .expect("utf8"),
            "--output-format",
            "txt",
        ])
        .assert()
        .success()
        .stdout(read_fixture("test_file_stereo_8bit_64spp_wav.txt"))
        .stderr("Done\n");
}

#[test]
fn applies_fixed_amplitude_scaling_to_waveform_data_output() {
    let unscaled_output = named_temp_file(".json");
    let scaled_output = named_temp_file(".json");

    for (output, amplitude_scale) in [(&unscaled_output, "1.0"), (&scaled_output, "2.0")] {
        Command::cargo_bin("audiowaveform")
            .expect("binary")
            .args([
                "-q",
                "-i",
                fixture_path("test_file_stereo.wav").to_str().expect("utf8"),
                "-o",
                output.path().to_str().expect("utf8"),
                "-z",
                "64",
                "--amplitude-scale",
                amplitude_scale,
            ])
            .assert()
            .success();
    }

    let unscaled = Waveform::load_from_path(unscaled_output.path(), None).expect("unscaled");
    let scaled = Waveform::load_from_path(scaled_output.path(), None).expect("scaled");
    let expected = unscaled
        .interleaved_samples()
        .iter()
        .map(|value| (i32::from(*value) * 2).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16)
        .collect::<Vec<_>>();

    assert_eq!(scaled.interleaved_samples(), expected);
}

#[test]
fn generates_png_output_to_file_and_stdout() {
    let output = named_temp_file(".png");
    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args([
            "-i",
            fixture_path("test_file_stereo_8bit_64spp_wav.dat")
                .to_str()
                .expect("utf8"),
            "-o",
            output.path().to_str().expect("utf8"),
            "-z",
            "128",
        ])
        .assert()
        .success()
        .stderr("Done\n");
    assert_png_file_matches_fixture(output.path(), "test_file_stereo_dat_128spp.png");

    let output = Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args([
            "-i",
            fixture_path("test_file_stereo_8bit_64spp_wav.dat")
                .to_str()
                .expect("utf8"),
            "--output-format",
            "png",
            "-z",
            "128",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_png_bytes_match_fixture(&output, "test_file_stereo_dat_128spp.png");
}

#[test]
fn transcodes_audio_to_wav_output() {
    let output = named_temp_file(".wav");
    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args([
            "-i",
            fixture_path("test_file_mono.mp3").to_str().expect("utf8"),
            "-o",
            output.path().to_str().expect("utf8"),
            "--output-format",
            "wav",
        ])
        .assert()
        .success()
        .stderr("Done\n");

    assert_wav_file_matches_fixture(output.path(), "test_file_mono_converted.wav", 1);
}

#[test]
fn quiet_mode_suppresses_done_output() {
    let output = named_temp_file(".dat");
    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args([
            "-q",
            "-i",
            fixture_path("test_file_stereo.wav").to_str().expect("utf8"),
            "-o",
            output.path().to_str().expect("utf8"),
            "-b",
            "8",
            "-z",
            "64",
        ])
        .assert()
        .success()
        .stderr("");
}

#[test]
fn rejects_unsupported_output_combinations() {
    Command::cargo_bin("audiowaveform")
        .expect("binary")
        .args([
            "-i",
            fixture_path("test_file_stereo.wav").to_str().expect("utf8"),
            "--output-format",
            "mp3",
        ])
        .assert()
        .failure()
        .stderr("Can't generate mp3 format output from wav format input\n");
}
