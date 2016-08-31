extern crate cpp_to_rust;

use std::path::PathBuf;

fn print_usage() {
  cpp_to_rust::log::info("Usage:");
  cpp_to_rust::log::info("\tcargo run source_dir output_dir");
  cpp_to_rust::log::info("");
  cpp_to_rust::log::info("See https://github.com/rust-qt/cpp_to_rust for more information.");
}

use cpp_to_rust::launcher::{BuildProfile, InvokationMethod, BuildEnvironment, run};

fn main() {
  let arguments: Vec<_> = std::env::args().collect();
  if arguments.len() != 3 {
    print_usage();
    return;
  }

  run(BuildEnvironment {
    invokation_method: InvokationMethod::CommandLine,
    source_dir_path: PathBuf::from(&arguments[1]),
    output_dir_path: PathBuf::from(&arguments[2]),
    num_jobs: None,
    build_profile: BuildProfile::Debug,
  });
}
