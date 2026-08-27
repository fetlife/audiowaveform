# audiowaveform for Ruby

The `audiowaveform` gem provides native Ruby bindings for the Rust
`audiowaveform` library. It generates waveform data in process without invoking
the command-line program. Audio decoding runs without Ruby's global VM lock, so
other Ruby threads can continue while a waveform is generated.

## Installation

Add the gem to your bundle:

```ruby
gem "audiowaveform", github: "fetlife/audiowaveform"
```

The source gem compiles a Rust extension during installation. Precompiled gems
can be published for supported platforms so application deployments do not need
a Rust toolchain. RubyGems.org publication is not configured; `ruby-vX.Y.Z`
tags attach the source gem to a release in FetLife's GitHub repository.

## Usage

Generate waveform data from an audio file:

```ruby
require "audiowaveform"

waveform = AudioWaveform.generate(
  "recording.mp3",
  samples_per_pixel: 256,
  split_channels: false
)

waveform.sample_rate       # => 44_100
waveform.samples_per_pixel # => 256
waveform.channels          # => 1
waveform.length            # => number of waveform points per channel
waveform.data              # => interleaved [min, max, ...] samples
```

Save the generated waveform as binary DAT, JSON, or text:

```ruby
waveform.save("recording.dat", bits: 8)
waveform.save("recording.json", bits: 16)

json = waveform.to_json(bits: 8)
binary = waveform.to_dat(bits: 16)
```

Use `pixels_per_second` instead of `samples_per_pixel` when a time-based scale
is more convenient:

```ruby
waveform = AudioWaveform.generate("recording.flac", pixels_per_second: 100)
```

Amplitude can be scaled with a numeric multiplier or normalized automatically:

```ruby
AudioWaveform.generate("quiet.wav", amplitude_scale: 1.5)
AudioWaveform.generate("quiet.wav", amplitude_scale: :auto)
```

Supported input formats are WAV/W64, MP3, FLAC, and Ogg/Vorbis. Opus and raw
PCM input are not currently exposed by the gem.

## Development

From the repository root:

```sh
bundle install
bundle exec rake
bundle exec rake build
```

The gem has its own semantic version and is packaged with `ruby-vX.Y.Z` tags.
