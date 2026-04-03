mod support;

use std::io::Cursor;

use audiowaveform::{
    GenerateOptions, RawAudioConfig, RawSampleFormat, ScaleSpec, WaveformFormat,
    decode_audio_from_path, generate_waveform_from_path, generate_waveform_from_raw_reader,
};

use self::support::{assert_bytes_eq, fixture_path, read_fixture};

#[test]
fn generates_expected_binary_waveform_from_wav() {
    let waveform = generate_waveform_from_path(
        fixture_path("test_file_stereo.wav"),
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

    assert_bytes_eq(&output, "test_file_stereo_8bit_64spp_wav.dat");
}

#[test]
fn decodes_supported_audio_inputs_with_expected_metadata() {
    let cases = [
        ("test_file_stereo.wav", 16_000, 2, 113_519_usize),
        ("test_file_mono_float32.wav", 16_000, 1, 115_190_usize),
        ("test_file_stereo.mp3", 16_000, 2, 113_519_usize),
        ("test_file_stereo.flac", 16_000, 2, 113_519_usize),
        ("test_file_stereo.oga", 16_000, 2, 113_792_usize),
    ];

    for (fixture, sample_rate, channels, frames) in cases {
        let pcm = decode_audio_from_path(fixture_path(fixture)).expect("decode audio");
        assert_eq!(pcm.sample_rate(), sample_rate, "{fixture}");
        assert_eq!(pcm.channels(), channels, "{fixture}");
        assert_eq!(pcm.frame_count(), frames, "{fixture}");
    }
}

#[test]
fn generates_expected_waveform_bytes_from_supported_audio_inputs() {
    let cases = [
        (
            "test_file_stereo.wav",
            "test_file_stereo_8bit_64spp_wav.dat",
            "test_file_stereo_8bit_64spp_wav.json",
        ),
        (
            "test_file_stereo.mp3",
            "rust/test_file_stereo_8bit_64spp_mp3.dat",
            "rust/test_file_stereo_8bit_64spp_mp3.json",
        ),
        (
            "test_file_stereo.flac",
            "rust/test_file_stereo_8bit_64spp_flac.dat",
            "rust/test_file_stereo_8bit_64spp_flac.json",
        ),
        (
            "test_file_stereo.oga",
            "rust/test_file_stereo_8bit_64spp_oga.dat",
            "rust/test_file_stereo_8bit_64spp_oga.json",
        ),
    ];

    for (input, dat_fixture, json_fixture) in cases {
        let waveform = generate_waveform_from_path(
            fixture_path(input),
            &GenerateOptions {
                scale: ScaleSpec::SamplesPerPixel(64),
                split_channels: false,
                amplitude_scale: None,
            },
        )
        .expect("generate waveform");

        let mut dat = Vec::new();
        waveform
            .write_to_writer(&mut dat, WaveformFormat::Dat, Some(8))
            .expect("write dat");
        assert_bytes_eq(&dat, dat_fixture);

        let mut json = Vec::new();
        waveform
            .write_to_writer(&mut json, WaveformFormat::Json, Some(8))
            .expect("write json");
        assert_bytes_eq(&json, json_fixture);
    }
}

#[test]
fn generates_expected_waveform_from_float_wav_audio() {
    let waveform = generate_waveform_from_path(
        fixture_path("test_file_mono_float32.wav"),
        &GenerateOptions {
            scale: ScaleSpec::SamplesPerPixel(64),
            split_channels: false,
            amplitude_scale: None,
        },
    )
    .expect("generate waveform");

    let mut dat = Vec::new();
    waveform
        .write_to_writer(&mut dat, WaveformFormat::Dat, Some(8))
        .expect("write dat");
    assert_bytes_eq(&dat, "test_file_mono_float32_8bit_64spp.dat");
}

#[test]
fn generates_expected_waveform_from_raw_audio() {
    let raw = read_fixture("test_file_mono.raw");
    let raw_config = RawAudioConfig::new(16_000, 1, RawSampleFormat::S16Le).expect("raw config");
    let options = GenerateOptions {
        scale: ScaleSpec::SamplesPerPixel(64),
        split_channels: false,
        amplitude_scale: None,
    };

    let waveform_from_raw =
        generate_waveform_from_raw_reader(Cursor::new(raw), &raw_config, &options)
            .expect("generate waveform from raw");
    let waveform_from_wav =
        generate_waveform_from_path(fixture_path("test_file_mono.wav"), &options)
            .expect("generate waveform from wav");

    assert_eq!(waveform_from_raw, waveform_from_wav);
}

#[test]
fn generates_expected_split_channel_and_auto_scaled_waveforms() {
    let split = generate_waveform_from_path(
        fixture_path("test_file_stereo.wav"),
        &GenerateOptions {
            scale: ScaleSpec::SamplesPerPixel(64),
            split_channels: true,
            amplitude_scale: None,
        },
    )
    .expect("generate split channels");
    let mut split_dat = Vec::new();
    split
        .write_to_writer(&mut split_dat, WaveformFormat::Dat, Some(8))
        .expect("write split dat");
    assert_bytes_eq(&split_dat, "test_file_2channel_8bit_64spp_wav.dat");

    let auto = generate_waveform_from_path(
        fixture_path("test_file_stereo.wav"),
        &GenerateOptions {
            scale: ScaleSpec::SamplesPerPixel(64),
            split_channels: false,
            amplitude_scale: Some(audiowaveform::AmplitudeScale::Auto),
        },
    )
    .expect("generate auto-scaled waveform");
    let mut auto_dat = Vec::new();
    auto.write_to_writer(&mut auto_dat, WaveformFormat::Dat, Some(8))
        .expect("write auto dat");
    assert_bytes_eq(&auto_dat, "test_file_stereo_8bit_64spp_wav_auto_scale.dat");
}

#[test]
fn reader_based_generation_matches_path_based_generation() {
    let bytes = read_fixture("test_file_stereo.wav");
    let from_reader = audiowaveform::generate_waveform_from_reader(
        Cursor::new(bytes),
        Some(audiowaveform::AudioFormat::Wav),
        &GenerateOptions {
            scale: ScaleSpec::SamplesPerPixel(64),
            split_channels: false,
            amplitude_scale: None,
        },
    )
    .expect("generate from reader");
    let from_path = generate_waveform_from_path(
        fixture_path("test_file_stereo.wav"),
        &GenerateOptions {
            scale: ScaleSpec::SamplesPerPixel(64),
            split_channels: false,
            amplitude_scale: None,
        },
    )
    .expect("generate from path");

    assert_eq!(from_reader, from_path);
}

#[test]
fn rejects_opus_generation_until_supported() {
    let error = generate_waveform_from_path(
        fixture_path("test_file_stereo.opus"),
        &GenerateOptions {
            scale: ScaleSpec::SamplesPerPixel(64),
            split_channels: false,
            amplitude_scale: None,
        },
    )
    .expect_err("opus should be unsupported");

    assert_eq!(error.to_string(), "Unsupported format: opus");
}
