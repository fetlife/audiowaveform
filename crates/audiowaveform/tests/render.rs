mod support;

use audiowaveform::{
    AmplitudeScale, Color, ColorScheme, RenderOptions, RenderStyle, ScaleSpec, WaveformColors,
    render_waveform,
};
#[cfg(feature = "decode")]
use audiowaveform::{GenerateOptions, generate_waveform_from_path};

use self::support::{assert_png_image_matches_fixture, load_waveform};

#[test]
fn renders_default_waveform_image_fixture() {
    let waveform = load_waveform("test_file_stereo_8bit_64spp_wav.dat")
        .resample(ScaleSpec::SamplesPerPixel(128))
        .expect("resample waveform");
    let image = render_waveform(&waveform, &RenderOptions::default()).expect("render image");

    assert_png_image_matches_fixture(&image, "test_file_stereo_dat_128spp.png");
}

#[test]
fn renders_waveform_images_with_custom_render_options() {
    let waveform = load_waveform("test_file_stereo_8bit_64spp_wav.dat")
        .resample(ScaleSpec::SamplesPerPixel(128))
        .expect("resample waveform");
    let two_channel = load_waveform("test_file_2channel_8bit_64spp_wav.dat")
        .resample(ScaleSpec::SamplesPerPixel(128))
        .expect("resample two-channel waveform");

    let cases = [
        (
            "test_file_stereo_dat_128spp_no_axis_labels.png",
            RenderOptions {
                axis_labels: false,
                ..RenderOptions::default()
            },
            &waveform,
        ),
        (
            "test_file_stereo_dat_128spp_scale_1.5.png",
            RenderOptions {
                amplitude_scale: AmplitudeScale::Fixed(1.5),
                ..RenderOptions::default()
            },
            &waveform,
        ),
        (
            "test_file_stereo_dat_128spp_scale_auto.png",
            RenderOptions {
                amplitude_scale: AmplitudeScale::Auto,
                ..RenderOptions::default()
            },
            &waveform,
        ),
        (
            "test_file_stereo_dat_128spp_offset.png",
            RenderOptions {
                start_time: 0.5,
                ..RenderOptions::default()
            },
            &waveform,
        ),
        (
            "test_file_stereo_dat_128spp_square_bars.png",
            RenderOptions {
                style: RenderStyle::Bars {
                    width: 8,
                    gap: 4,
                    style: audiowaveform::BarStyle::Square,
                },
                ..RenderOptions::default()
            },
            &waveform,
        ),
        (
            "test_file_stereo_dat_128spp_rounded_bars.png",
            RenderOptions {
                style: RenderStyle::Bars {
                    width: 8,
                    gap: 4,
                    style: audiowaveform::BarStyle::Rounded,
                },
                ..RenderOptions::default()
            },
            &waveform,
        ),
        (
            "test_file_stereo_dat_128spp_square_bars_offset.png",
            RenderOptions {
                start_time: 0.5,
                style: RenderStyle::Bars {
                    width: 8,
                    gap: 4,
                    style: audiowaveform::BarStyle::Square,
                },
                ..RenderOptions::default()
            },
            &waveform,
        ),
        (
            "test_file_2channel_8bit_64spp_wav.png",
            RenderOptions::default(),
            &two_channel,
        ),
        (
            "test_file_2channel_8bit_64spp_wav_bars.png",
            RenderOptions {
                style: RenderStyle::Bars {
                    width: 8,
                    gap: 4,
                    style: audiowaveform::BarStyle::Square,
                },
                ..RenderOptions::default()
            },
            &two_channel,
        ),
    ];

    for (fixture, options, source) in cases {
        let image = render_waveform(source, &options).expect("render image");
        assert_png_image_matches_fixture(&image, fixture);
    }
}

#[test]
fn renders_waveforms_with_custom_colors() {
    let waveform = load_waveform("test_file_stereo_8bit_64spp_wav.dat")
        .resample(ScaleSpec::SamplesPerPixel(128))
        .expect("resample waveform");
    let audition = render_waveform(
        &waveform,
        &RenderOptions {
            colors: ColorScheme::Audition.palette(),
            ..RenderOptions::default()
        },
    )
    .expect("render audition waveform");
    assert_png_image_matches_fixture(&audition, "test_file_stereo_dat_128spp_audition.png");

    let custom = render_waveform(
        &waveform,
        &RenderOptions {
            colors: WaveformColors {
                border: Color::opaque(255, 0, 0),
                background: Color::rgba(0, 255, 0, 128),
                waveform: vec![Color::opaque(0, 0, 255)],
                axis_label: Color::opaque(255, 255, 255),
            },
            ..RenderOptions::default()
        },
    )
    .expect("render custom waveform");
    assert_png_image_matches_fixture(&custom, "test_file_stereo_dat_128spp_colors.png");
}

#[cfg(feature = "decode")]
#[test]
fn renders_audio_pipeline_waveform_images() {
    let stereo = generate_waveform_from_path(
        support::fixture_path("test_file_stereo.wav"),
        &GenerateOptions {
            scale: ScaleSpec::SamplesPerPixel(128),
            split_channels: false,
            amplitude_scale: None,
        },
    )
    .expect("generate stereo waveform");
    let stereo_image = render_waveform(&stereo, &RenderOptions::default()).expect("render stereo");
    assert_png_image_matches_fixture(&stereo_image, "test_file_stereo_wav_128spp.png");

    let split = generate_waveform_from_path(
        support::fixture_path("test_file_stereo.wav"),
        &GenerateOptions {
            scale: ScaleSpec::SamplesPerPixel(128),
            split_channels: true,
            amplitude_scale: None,
        },
    )
    .expect("generate split waveform");
    let split_image = render_waveform(
        &split,
        &RenderOptions {
            colors: WaveformColors {
                border: Color::opaque(255, 0, 0),
                background: Color::rgba(0, 255, 0, 128),
                waveform: vec![Color::opaque(0, 0, 255), Color::opaque(0, 0, 128)],
                axis_label: Color::opaque(255, 255, 255),
            },
            ..RenderOptions::default()
        },
    )
    .expect("render split waveform");
    assert_png_image_matches_fixture(&split_image, "test_file_stereo_wav_split_channels.png");
}

#[cfg(feature = "decode")]
#[test]
fn renders_fit_width_waveform_images_from_audio() {
    let wav = generate_waveform_from_path(
        support::fixture_path("test_file_stereo.wav"),
        &GenerateOptions {
            scale: ScaleSpec::FitWidth {
                width_pixels: 500,
                time_range: None,
            },
            split_channels: false,
            amplitude_scale: None,
        },
    )
    .expect("generate fit-width wav");
    let wav_image = render_waveform(
        &wav,
        &RenderOptions {
            width: 500,
            height: 150,
            ..RenderOptions::default()
        },
    )
    .expect("render fit-width wav");
    assert_png_image_matches_fixture(&wav_image, "test_file_stereo_wav_500.png");

    let mp3 = generate_waveform_from_path(
        support::fixture_path("test_file_stereo.mp3"),
        &GenerateOptions {
            scale: ScaleSpec::FitWidth {
                width_pixels: 500,
                time_range: None,
            },
            split_channels: false,
            amplitude_scale: None,
        },
    )
    .expect("generate fit-width mp3");
    let mp3_image = render_waveform(
        &mp3,
        &RenderOptions {
            width: 500,
            height: 150,
            ..RenderOptions::default()
        },
    )
    .expect("render fit-width mp3");
    assert_png_image_matches_fixture(&mp3_image, "test_file_stereo_mp3_500.png");
}

#[test]
fn renders_reference_images_for_amplitude_scale_fixtures() {
    let single = load_waveform("test_file_image_amplitude_scale_1channel.dat")
        .resample(ScaleSpec::SamplesPerPixel(64))
        .expect("resample single");
    let single_image = render_waveform(&single, &RenderOptions::default()).expect("render single");
    assert_png_image_matches_fixture(
        &single_image,
        "test_file_image_amplitude_scale_1channel.png",
    );

    let dual = load_waveform("test_file_image_amplitude_scale_2channel.dat")
        .resample(ScaleSpec::SamplesPerPixel(64))
        .expect("resample dual");
    let dual_image = render_waveform(&dual, &RenderOptions::default()).expect("render dual");
    assert_png_image_matches_fixture(&dual_image, "test_file_image_amplitude_scale_2channel.png");
}
