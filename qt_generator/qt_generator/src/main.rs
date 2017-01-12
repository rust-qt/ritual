extern crate clap;
extern crate cpp_to_rust_generator;
extern crate cpp_to_rust_common;
extern crate rusqlite;
extern crate compress;
extern crate select as html_parser;
extern crate regex;
extern crate qt_generator_common;

use cpp_to_rust_common::errors::{Result, ChainErr};
use cpp_to_rust_common::file_utils::{create_dir_all, canonicalize};
use std::path::PathBuf;

mod executor;
mod doc_decoder;
mod doc_parser;
mod fix_header_names;
mod lib_configs;

fn run(matches: clap::ArgMatches) -> Result<()> {
  let libs: Vec<_> =
    matches.values_of("libs").chain_err(|| "clap arg missing")?.map(|s| s.to_lowercase()).collect();
  let output_dir = PathBuf::from(matches.value_of("output_dir").chain_err(|| "clap arg missing")?);
  if !output_dir.exists() {
    create_dir_all(&output_dir)?;
  }
  let cache_dir = PathBuf::from(matches.value_of("cache_dir").chain_err(|| "clap arg missing")?);
  if !cache_dir.exists() {
    create_dir_all(&cache_dir)?;
  }
  executor::exec_all(libs, canonicalize(&cache_dir)?, canonicalize(&output_dir)?)
}

fn main() {
  let result = {
    use clap::{Arg, App};
    const ABOUT: &'static str = "Generates rust_qt crates using cpp_to_rust";
    const AFTER_HELP: &'static str = "See https://github.com/rust-qt/cpp_to_rust and \
                                          https://github.com/rust-qt/rust_qt_gen for more details.";
    const CACHE_DIR_HELP: &'static str = "Directory for cache and temporary files";
    const OUTPUT_DIR_HELP: &'static str = "Directory for generated crates";
    const LIBS_HELP: &'static str = "Libraries (Qt modules) to process. Supported names: \
                                     core, gui, widgets.";

    run(App::new("cpp_to_rust")
      .about(ABOUT)
      .after_help(AFTER_HELP)
      .arg(Arg::with_name("cache_dir")
        .short("c")
        .long("cache_dir")
        .value_name("DIR")
        .help(CACHE_DIR_HELP)
        .takes_value(true)
        .required(true))
      .arg(Arg::with_name("output_dir")
        .short("o")
        .long("output_dir")
        .value_name("DIR")
        .help(OUTPUT_DIR_HELP)
        .takes_value(true)
        .required(true))
      .arg(Arg::with_name("libs")
        .short("l")
        .long("libs")
        .value_name("LIB1 LIB2 LIB3")
        .help(LIBS_HELP)
        .takes_value(true)
        .required(true)
        .multiple(true)
        .use_delimiter(false))
      .get_matches())
  };
  if let Err(err) = result {
    err.display_report();
    std::process::exit(1);
  }
}
