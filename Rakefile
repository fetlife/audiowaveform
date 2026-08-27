# frozen_string_literal: true

require "rake"

def run_ruby_task(task)
  Dir.chdir("bindings/ruby") do
    sh "bundle", "exec", "rake", task
  end
end

desc "Compile the Ruby native extension"
task :compile do
  run_ruby_task("compile")
end

desc "Run the Ruby binding tests"
task :test do
  run_ruby_task("test")
end

desc "Build the Ruby source gem"
task :build do
  run_ruby_task("build")
end

desc "Validate the bundled RBS signature"
task :rbs do
  run_ruby_task("rbs")
end

task default: %i[compile test rbs]
