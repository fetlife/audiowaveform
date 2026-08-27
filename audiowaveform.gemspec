# frozen_string_literal: true

require_relative "bindings/ruby/lib/audiowaveform/version"

Gem::Specification.new do |spec|
  spec.name = "audiowaveform"
  spec.version = AudioWaveform::VERSION
  spec.authors = ["Andrii Dmytrenko", "BBC Research and Development"]
  spec.summary = "Generate audio waveform data using the Rust audiowaveform library"
  spec.description = <<~DESCRIPTION.strip
    Native Ruby bindings for generating, inspecting, and serializing waveform
    data from WAV, MP3, FLAC, and Ogg/Vorbis audio.
  DESCRIPTION
  spec.homepage = "https://github.com/fetlife/audiowaveform"
  spec.license = "GPL-3.0-or-later"
  spec.required_ruby_version = ">= 3.2"

  spec.metadata["allowed_push_host"] = "https://rubygems.org"
  spec.metadata["cargo_crate_name"] = "audiowaveform-ruby"
  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = spec.homepage
  spec.metadata["changelog_uri"] = "#{spec.homepage}/blob/master/bindings/ruby/CHANGELOG.md"
  spec.metadata["rubygems_mfa_required"] = "true"

  spec.files = Dir.chdir(__dir__) do
    Dir[
      "bindings/ruby/CHANGELOG.md",
      "bindings/ruby/Cargo.lock",
      "bindings/ruby/Cargo.toml",
      "bindings/ruby/README.md",
      "bindings/ruby/ext/**/*",
      "bindings/ruby/lib/**/*",
      "crates/audiowaveform/**/*",
      "crates/audiowaveform-cli/**/*",
      "Cargo.lock",
      "Cargo.toml",
      "COPYING",
      "README.md",
      "sig/**/*",
    ].reject do |path|
      File.directory?(path) || path.match?(/\.(?:bundle|dll|dylib|so)\z/)
    end
  end
  spec.require_paths = ["bindings/ruby/lib"]
  spec.extensions = ["bindings/ruby/ext/audiowaveform/extconf.rb"]

  spec.add_dependency "rb_sys", "~> 0.9"
end
