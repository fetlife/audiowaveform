# frozen_string_literal: true

require "minitest/autorun"
require "tmpdir"
require "json"
require "pathname"
require "audiowaveform"

module AudioWaveformTestSupport
  FIXTURES = File.expand_path("../../../fixtures", __dir__)

  def fixture(name)
    File.join(FIXTURES, name)
  end
end
