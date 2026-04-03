# audiowaveform

[![CI](https://github.com/fetlife/audiowaveform/actions/workflows/rust.yml/badge.svg)](https://github.com/fetlife/audiowaveform/actions/workflows/rust.yml)

`audiowaveform` is a Rust library and CLI for generating waveform data from audio,
serializing waveform files, rendering PNG waveform images, and transcoding audio
to PCM16 WAV.

This repository is the canonical home of the Rust rewrite:
[github.com/fetlife/audiowaveform](https://github.com/fetlife/audiowaveform).

It is a Rust rewrite of the original BBC `audiowaveform` project:
[github.com/bbc/audiowaveform](https://github.com/bbc/audiowaveform).
The original project was created by Chris Needham and contributors at BBC
Research & Development.

The repository contains a Rust-only workspace:

- `crates/audiowaveform`: reusable library crate
- `crates/audiowaveform-cli`: thin `audiowaveform` command-line wrapper

![Example Waveform](doc/example.png "Example Waveform")

## Features

- Decode MP3, WAV, FLAC, Ogg/Vorbis, and raw PCM or floating-point audio
- Generate `.dat`, `.json`, and `.txt` waveform files
- Render PNG waveform images in pure Rust
- Transcode decoded audio or raw PCM input to PCM16 WAV
- Use path-based, stream-based, or in-memory APIs from the library crate

Opus is intentionally unsupported in the current Rust implementation.

## Quick Start

Build the workspace:

```sh
cargo build --workspace
```

Run the CLI:

```sh
cargo run -p audiowaveform-cli -- -i test/data/test_file_stereo.wav -o output.dat
```

Install the CLI locally:

```sh
cargo install --path crates/audiowaveform-cli
```

Run the test suite:

```sh
cargo test --workspace
```

## Library Usage

Generate waveform data from an audio file:

```rust,no_run
use audiowaveform::{GenerateOptions, WaveformFormat, generate_waveform_from_path};

fn main() -> Result<(), audiowaveform::Error> {
    let waveform = generate_waveform_from_path("input.mp3", &GenerateOptions::default())?;
    waveform.save_to_path("output.dat", Some(WaveformFormat::Dat))?;
    Ok(())
}
```

Render a PNG from a stored waveform:

```rust,no_run
use std::fs::File;

use audiowaveform::{RenderOptions, Waveform, write_waveform_png};

fn main() -> Result<(), audiowaveform::Error> {
    let waveform = Waveform::load_from_path("input.dat", None)?;
    write_waveform_png(&waveform, &RenderOptions::default(), File::create("output.png")?)?;
    Ok(())
}
```

Generate waveform data from in-memory PCM:

```rust
use audiowaveform::{GenerateOptions, PcmAudio, generate_waveform_from_pcm};

fn main() -> Result<(), audiowaveform::Error> {
    let pcm = PcmAudio::new(48_000, 1, vec![0_i16; 48_000])?;
    let waveform = generate_waveform_from_pcm(&pcm, &GenerateOptions::default())?;
    assert!(!waveform.is_empty());
    Ok(())
}
```

Additional examples live in `crates/audiowaveform/examples`.

## CLI Usage

Generate `.dat` waveform data:

```sh
audiowaveform -i input.wav -o output.dat -z 128 -b 8
```

Render a PNG from waveform data:

```sh
audiowaveform -i input.dat -o output.png -w 1000 -h 200
```

Generate JSON waveform data from compressed audio:

```sh
audiowaveform -i input.flac -o output.json --pixels-per-second 50
```

Convert raw PCM to WAV:

```sh
audiowaveform -i input.raw -o output.wav --input-format raw --raw-samplerate 48000 --raw-channels 2 --raw-format s16le
```

See all options with:

```sh
audiowaveform --help
```

## Supported Formats

Audio input:

- `mp3`
- `wav` and `w64`
- `flac`
- `ogg` and `oga`
- `raw`

Waveform input:

- `dat`
- `json`

Waveform output:

- `dat`
- `json`
- `txt`

Image output:

- `png`

Audio output:

- `wav`

## Documentation

- Waveform file formats: [doc/DataFormat.md](doc/DataFormat.md)
- CLI man page: [doc/audiowaveform.1](doc/audiowaveform.1)
- Waveform format man page: [doc/audiowaveform.5](doc/audiowaveform.5)

Generate local API docs with:

```sh
cargo doc --workspace --no-deps
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for workflow, documentation, and testing expectations.

## License

`audiowaveform` is released under the GPL-3.0-or-later license. See [COPYING](COPYING).
