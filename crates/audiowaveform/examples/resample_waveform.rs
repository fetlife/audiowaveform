use std::env;
use std::fs::File;

use audiowaveform::{ScaleSpec, Waveform, WaveformFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let input = args
        .next()
        .unwrap_or_else(|| "fixtures/test_file_stereo_8bit_64spp_wav.dat".to_string());
    let output = args.next().unwrap_or_else(|| "resampled.dat".to_string());

    let waveform = Waveform::load_from_path(&input, None)?;
    let resampled = waveform.resample(ScaleSpec::SamplesPerPixel(128))?;
    let output_file = File::create(output)?;
    resampled.write_to_writer(output_file, WaveformFormat::Dat, None)?;

    Ok(())
}
