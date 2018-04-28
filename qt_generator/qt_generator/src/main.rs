//! Generator of Rust-Qt crates.
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust/tree/master/qt_generator/qt_generator)
//! for more information.

extern crate clap;
extern crate compress;
extern crate cpp_to_rust_generator;
extern crate qt_generator_common;
extern crate regex;
extern crate rusqlite;
extern crate select as html_parser;

//use cpp_to_rust_generator::common::errors::{Result, ChainErr};
//use cpp_to_rust_generator::common::file_utils::{create_dir_all, canonicalize};
//use std::path::PathBuf;

mod executor;
mod doc_decoder;
mod doc_parser;
mod fix_header_names;
mod lib_configs;
mod versions;

mod new_impl;

fn main() {
  new_impl::new_main()
}

/*
fn main() {
  let result = {
    use clap::{Arg, App};
    const ABOUT: &'static str = "Generates rust_qt crates using cpp_to_rust";
    const AFTER_HELP: &'static str = "See https://github.com/rust-qt/cpp_to_rust for more details.";
    const CACHE_DIR_HELP: &'static str = "Directory for cache and temporary files";
    const OUTPUT_DIR_HELP: &'static str = "Directory for generated crates";
    const LIBS_HELP: &'static str = "Libraries (Qt modules) to process. Specify \"all\" \
      to process all supported modules or specify one or multiple of the following: \
      core, gui, widgets, ui_tools, 3d_core, 3d_render, 3d_input, 3d_logic, 3d_extras, all.";
    const CACHE_USAGE_HELP: &'static str = "Cache usage for repeated execution";
    const CACHE_USAGE_LONG_HELP: &'static str = "Cache usage for repeated execution:\n\
                                                 0 - no cache usage (default),\n\
                                                 1 - use raw C++ data,\n\
                                                 2 - use prepared C++ data,\n\
                                                 3 - use all and allow complete skips";
    const DEBUG_LOGGING_HELP: &'static str = "Debug logging mode";
    const DEBUG_LOGGING_LONG_HELP: &'static str = "Debug logging mode:\n\
      \"print\" - print to stderr;\n\"save\" - save to cache directory;\n\
      \"disable\" - disable (default)";
    const QUIET_HELP: &'static str = "Don't output status messages to stderr";
    const DONT_WRITE_CACHE_HELP: &'static str = "Don't write files for dependency processing";
    const NO_LOCAL_PATHS_HELP: &'static str = "Don't write local paths to output Cargo.toml file";


    run(
      App::new("cpp_to_rust")
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .arg(
          Arg::with_name("cache-dir")
            .short("c")
            .long("cache-dir")
            .value_name("DIR")
            .help(CACHE_DIR_HELP)
            .takes_value(true)
            .required(true),
        )
        .arg(
          Arg::with_name("output-dir")
            .short("o")
            .long("output-dir")
            .value_name("DIR")
            .help(OUTPUT_DIR_HELP)
            .takes_value(true)
            .required(true),
        )
        .arg(
          Arg::with_name("libs")
            .short("l")
            .long("libs")
            .value_name("LIB1 LIB2 LIB3")
            .help(LIBS_HELP)
            .takes_value(true)
            .required(true)
            .multiple(true)
            .use_delimiter(false),
        )
        .arg(
          Arg::with_name("cache-usage")
            .short("C")
            .long("cache-usage")
            .value_name("N")
            .possible_values(&["0", "1", "2", "3"])
            .default_value("0")
            .hide_default_value(true)
            .hide_possible_values(true)
            .help(CACHE_USAGE_HELP)
            .long_help(CACHE_USAGE_LONG_HELP)
            .takes_value(true),
        )
        .arg(
          Arg::with_name("debug-logging")
            .long("debug-logging")
            .value_name("mode")
            .possible_values(&["print", "save", "disable"])
            .default_value("disable")
            .hide_default_value(true)
            .hide_possible_values(true)
            .help(DEBUG_LOGGING_HELP)
            .long_help(DEBUG_LOGGING_LONG_HELP)
            .takes_value(true),
        )
        .arg(
          Arg::with_name("dont-write-cache")
            .long("dont-write-cache")
            .help(DONT_WRITE_CACHE_HELP),
        )
        .arg(Arg::with_name("quiet").long("quiet").short("q").help(
          QUIET_HELP,
        ))
        .arg(
          Arg::with_name("no-local-paths")
            .long("no-local-paths")
            .help(NO_LOCAL_PATHS_HELP),
        )
        .get_matches(),
    )
  };
  if let Err(err) = result {
    err.display_report();
    std::process::exit(1);
  }
}
*/
