extern crate cpp_to_rust;
extern crate clap;

use std::path::PathBuf;

use cpp_to_rust::launcher::{BuildProfile, InvokationMethod, BuildEnvironment, run};

fn main() {
  use clap::{Arg, App};
  const ABOUT: &'static str = "Generates Rust crates from C++ libraries";
  const AFTER_HELP: &'static str = "See https://github.com/rust-qt/cpp_to_rust for more details.";
  const SOURCE_DIR_HELP: &'static str = "Source directory of the library crate containing \
                                         spec.json and additional files";
  const OUTPUT_DIR_HELP: &'static str = "Directory of the generated crate";
  const DEPENDENCIES_HELP: &'static str = "Output directories of processed dependency libraries \
                                             (separated by spaces)";

  let matches = App::new("cpp_to_rust")
    .about(ABOUT)
    .after_help(AFTER_HELP)
    .arg(Arg::with_name("source_dir")
      .short("s")
      .long("source_dir")
      .value_name("DIR")
      .help(SOURCE_DIR_HELP)
      .takes_value(true)
      .required(true))
    .arg(Arg::with_name("output_dir")
      .short("o")
      .long("output_dir")
      .value_name("DIR")
      .help(OUTPUT_DIR_HELP)
      .takes_value(true)
      .required(true))
    .arg(Arg::with_name("dependencies")
      .short("d")
      .long("dependencies")
      .value_name("DIR1 DIR2 DIR3")
      .help(DEPENDENCIES_HELP)
      .takes_value(true)
      .multiple(true)
      .use_delimiter(false))
    .get_matches();

  let dependency_paths = match matches.values_of("dependencies") {
    Some(values) => values.map(PathBuf::from).collect(),
    None => Vec::new(),
  };

  run(BuildEnvironment {
    invokation_method: InvokationMethod::CommandLine,
    source_dir_path: PathBuf::from(matches.value_of("source_dir").unwrap()),
    output_dir_path: PathBuf::from(matches.value_of("output_dir").unwrap()),
    dependency_paths: dependency_paths,
    num_jobs: None,
    build_profile: BuildProfile::Debug,
  });
}
