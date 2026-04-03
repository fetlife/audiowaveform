use std::env;
use std::f32::consts::PI;

use audiowaveform::{GenerateOptions, PcmAudio, WaveformFormat, generate_waveform_from_pcm};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args()
        .nth(1)
        .unwrap_or_else(|| "pcm-output.json".to_string());

    let sample_rate = 48_000_u32;
    let frequency = 220.0_f32;
    let frames = sample_rate as usize;
    let mut samples = Vec::with_capacity(frames);
    for frame in 0..frames {
        let angle = (frame as f32 / sample_rate as f32) * frequency * PI * 2.0;
        samples.push((angle.sin() * i16::MAX as f32 * 0.5) as i16);
    }

    let pcm = PcmAudio::new(sample_rate, 1, samples)?;
    let waveform = generate_waveform_from_pcm(&pcm, &GenerateOptions::default())?;
    waveform.save_to_path(output, Some(WaveformFormat::Json))?;

    Ok(())
}
