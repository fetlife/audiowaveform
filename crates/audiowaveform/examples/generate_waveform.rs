use std::env;
use std::fs::File;

use audiowaveform::{GenerateOptions, WaveformFormat, generate_waveform_from_path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let input = args
        .next()
        .unwrap_or_else(|| "fixtures/test_file_stereo.wav".to_string());
    let output = args.next().unwrap_or_else(|| "output.dat".to_string());

    let waveform = generate_waveform_from_path(&input, &GenerateOptions::default())?;
    let output_file = File::create(output)?;
    waveform.write_to_writer(output_file, WaveformFormat::Dat, None)?;

    Ok(())
}
