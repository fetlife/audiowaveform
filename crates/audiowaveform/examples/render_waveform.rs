use std::env;
use std::fs::File;

use audiowaveform::{RenderOptions, Waveform, write_waveform_png};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let input = args
        .next()
        .unwrap_or_else(|| "test/data/test_file_stereo_8bit_64spp_wav.dat".to_string());
    let output = args.next().unwrap_or_else(|| "output.png".to_string());

    let waveform = Waveform::load_from_path(&input, None)?;
    let output_file = File::create(output)?;
    write_waveform_png(&waveform, &RenderOptions::default(), output_file)?;

    Ok(())
}
