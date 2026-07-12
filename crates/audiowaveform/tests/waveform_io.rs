mod support;

use audiowaveform::{Waveform, WaveformFormat};

use self::support::{assert_bytes_eq, fixture_path, load_waveform, named_temp_file};

#[test]
fn round_trips_binary_waveform_fixture() {
    let fixture = fixture_path("test_file_stereo_8bit_64spp_wav.dat");
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

    assert_bytes_eq(&output, "test_file_stereo_8bit_64spp_wav.dat");
}

#[test]
fn emits_expected_json_and_text_fixture_bytes() {
    let fixture = fixture_path("test_file_stereo_8bit_64spp_wav.dat");
    let waveform = Waveform::load_from_path(&fixture, None).expect("load waveform");

    let mut json = Vec::new();
    waveform
        .write_to_writer(&mut json, WaveformFormat::Json, Some(8))
        .expect("write json");
    assert_bytes_eq(&json, "test_file_stereo_8bit_64spp_wav.json");

    let mut txt = Vec::new();
    waveform
        .write_to_writer(&mut txt, WaveformFormat::Txt, Some(8))
        .expect("write txt");
    assert_bytes_eq(&txt, "test_file_stereo_8bit_64spp_wav.txt");
}

#[test]
fn loads_version1_and_version2_waveform_fixtures() {
    let cases = [
        (
            "test_file_stereo_8bit_64spp_wav.dat",
            8_u8,
            1_u16,
            1774_usize,
        ),
        (
            "test_file_stereo_8bit_64spp_wav_v2.dat",
            8_u8,
            1_u16,
            1774_usize,
        ),
        (
            "test_file_stereo_16bit_64spp_wav.dat",
            16_u8,
            1_u16,
            1774_usize,
        ),
        (
            "test_file_stereo_16bit_64spp_wav_v2.dat",
            16_u8,
            1_u16,
            1774_usize,
        ),
        ("07023003_8bit_64spp_2channel.dat", 8_u8, 2_u16, 513_usize),
    ];

    for (fixture, bits, channels, length) in cases {
        let waveform = load_waveform(fixture);
        let sample_rate = if fixture == "07023003_8bit_64spp_2channel.dat" {
            44_100
        } else {
            16_000
        };
        assert_eq!(waveform.sample_rate(), sample_rate, "{fixture}");
        assert_eq!(waveform.samples_per_pixel(), 64, "{fixture}");
        assert_eq!(waveform.storage_bits(), bits, "{fixture}");
        assert_eq!(waveform.channels(), channels, "{fixture}");
        let expected_length = if fixture == "07023003_8bit_64spp_2channel.dat" {
            10_438
        } else {
            length
        };
        assert_eq!(waveform.len(), expected_length, "{fixture}");
    }
}

#[test]
fn loads_zero_length_waveform_fixture() {
    let waveform = load_waveform("zero_length.dat");
    assert_eq!(waveform.sample_rate(), 16_000);
    assert_eq!(waveform.samples_per_pixel(), 64);
    assert!(waveform.is_empty());
}

#[test]
fn loads_truncated_waveform_fixture_using_available_points() {
    let waveform = load_waveform("size_mismatch.dat");
    assert_eq!(waveform.sample_rate(), 16_000);
    assert_eq!(waveform.samples_per_pixel(), 64);
    assert_eq!(waveform.channels(), 1);
    assert_eq!(waveform.storage_bits(), 8);
    assert_eq!(waveform.len(), 1_800);
}

#[test]
fn rejects_invalid_waveform_fixtures() {
    let cases = [
        ("version3.dat", "Cannot load data file version: 3"),
        (
            "sample_rate_too_low.dat",
            "Invalid sample rate: minimum 1 Hz",
        ),
        (
            "samples_per_pixel_too_low.dat",
            "Invalid samples per pixel: minimum 2",
        ),
        (
            "too_many_channels.dat",
            "Invalid channels: must be between 1 and 24",
        ),
        (
            "not_enough_channels.dat",
            "Invalid channels: must be between 1 and 24",
        ),
    ];

    for (fixture, expected) in cases {
        let error = Waveform::load_from_path(fixture_path(fixture), None).expect_err("invalid dat");
        assert!(error.to_string().contains(expected), "{fixture}: {}", error);
    }
}

#[test]
fn rejects_invalid_json_waveform_payloads() {
    let error = Waveform::load_from_reader(
        br#"{"version":2,"channels":1,"sample_rate":44100,"samples_per_pixel":256,"bits":8,"length":2,"data":[1,2,3]}"#
            .as_slice(),
        WaveformFormat::Json,
    )
    .expect_err("json length mismatch");
    assert_eq!(
        error.to_string(),
        "Length mismatch: expected 4 values, found 3"
    );

    let error = Waveform::load_from_reader(
        br#"{"version":2,"channels":1,"sample_rate":44100,"samples_per_pixel":256,"bits":8,"length":1,"data":[999,0]}"#
            .as_slice(),
        WaveformFormat::Json,
    )
    .expect_err("json range mismatch");
    assert_eq!(error.to_string(), "Data value out of range: 999");

    let overflowing_length = format!(
        r#"{{"version":2,"channels":1,"sample_rate":44100,"samples_per_pixel":256,"bits":8,"length":{},"data":[]}}"#,
        usize::MAX
    );
    let error = Waveform::load_from_reader(overflowing_length.as_bytes(), WaveformFormat::Json)
        .expect_err("overflowing json length");
    assert_eq!(error.to_string(), "Waveform length is too large");
}

#[test]
fn saves_empty_waveform_as_header_only_dat_file() {
    let waveform = Waveform::new(44_100, 256, 1).expect("new waveform");
    let mut dat = Vec::new();
    waveform
        .write_to_writer(&mut dat, WaveformFormat::Dat, None)
        .expect("write dat");
    assert_eq!(dat.len(), 20);
}

#[test]
fn writes_expected_text_and_json_for_two_channel_waveforms() {
    let waveform = load_waveform("07023003_8bit_64spp_2channel.dat");

    let mut json = Vec::new();
    waveform
        .write_to_writer(&mut json, WaveformFormat::Json, Some(8))
        .expect("write json");
    assert_bytes_eq(&json, "07023003_8bit_64spp_2channel.json");

    let mut txt = Vec::new();
    waveform
        .write_to_writer(&mut txt, WaveformFormat::Txt, Some(8))
        .expect("write txt");
    assert_bytes_eq(&txt, "07023003_8bit_64spp_2channel.txt");
}

#[test]
fn saves_waveform_to_path_and_reloads_it() {
    let waveform = load_waveform("test_file_stereo_8bit_64spp_wav.dat");
    let file = named_temp_file(".dat");
    waveform
        .save_to_path(file.path(), Some(WaveformFormat::Dat))
        .expect("save waveform");

    let reloaded =
        Waveform::load_from_path(file.path(), Some(WaveformFormat::Dat)).expect("reload waveform");
    assert_eq!(reloaded, waveform);
}

#[test]
fn rejects_invalid_storage_bits_and_txt_loading() {
    let waveform = load_waveform("test_file_stereo_8bit_64spp_wav.dat");

    let error = waveform
        .write_to_writer(Vec::new(), WaveformFormat::Dat, Some(10))
        .expect_err("invalid bits");
    assert_eq!(error.to_string(), "Invalid bits: must be either 8 or 16");

    let error = Waveform::load_from_reader(b"-1,1\n".as_slice(), WaveformFormat::Txt)
        .expect_err("txt input unsupported");
    assert_eq!(error.to_string(), "Unsupported format: txt");
}

#[test]
fn rescales_waveforms_to_a_coarser_scale() {
    let waveform = load_waveform("test_file_stereo_8bit_64spp_wav.dat");
    let resampled = waveform
        .resample(audiowaveform::ScaleSpec::SamplesPerPixel(128))
        .expect("resample");

    assert_eq!(resampled.sample_rate(), 16_000);
    assert_eq!(resampled.samples_per_pixel(), 128);
    assert_eq!(resampled.channels(), 1);
    assert_eq!(resampled.len(), 887);
}
