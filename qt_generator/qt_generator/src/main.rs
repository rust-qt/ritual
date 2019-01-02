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

use cpp_to_rust_generator::common::errors::{ChainErr, Result};
use cpp_to_rust_generator::common::file_utils::canonicalize;
use cpp_to_rust_generator::common::log;
use cpp_to_rust_generator::processor;
use cpp_to_rust_generator::workspace::Workspace;
use lib_configs::make_config;
use qt_generator_common::all_crate_names;
use std::path::PathBuf;

mod detect_signal_argument_types;
mod detect_signals_and_slots;
mod doc_decoder;
mod doc_parser;
mod fix_header_names;
mod lib_configs;
mod versions;

fn run(matches: ::clap::ArgMatches) -> Result<()> {
    let workspace_path = canonicalize(&PathBuf::from(
        matches
            .value_of("workspace")
            .chain_err(|| "clap arg missing")?,
    ))?;

    log::status(format!("Workspace: {}", workspace_path.display()));
    let mut workspace = Workspace::new(workspace_path)?;
    workspace.set_disable_logging(matches.is_present("disable-logging"))?;
    let mut was_any_action = false;

    let crates: Vec<_> = matches
        .values_of("crates")
        .chain_err(|| "clap arg missing")?
        .collect();

    let final_crates = if crates.iter().any(|x| *x == "all") {
        all_crate_names().iter().map(|x| *x).collect()
    } else {
        crates
    };

    let operations: Vec<_> = matches
        .values_of("operations")
        .chain_err(|| "clap arg missing")?
        .map(|s| s.to_lowercase())
        .collect();

    if operations.is_empty() {
        log::error("No action requested. Run \"qt_generator --help\".");
        return Ok(());
    }

    for crate_name in &final_crates {
        let config = make_config(&crate_name)?;
        was_any_action = true;
        processor::process(&mut workspace, &config, &operations)?;
    }

    //workspace.save_data()?;
    if was_any_action {
        log::status("qt_generator finished");
    } else {
        log::error("No action requested. Run \"qt_generator --help\".");
    }
    Ok(())
}

pub fn main() {
    let result = {
        use clap::{App, Arg};
        const ABOUT: &'static str = "Generates rust_qt crates using cpp_to_rust";
        const AFTER_HELP: &'static str =
            "\
             Example:\n    qt_generator -w /path/to/workspace -p all -g\n\n\
             See https://github.com/rust-qt/cpp_to_rust for more details.";
        const WORKSPACE_DIR_HELP: &'static str = "Directory for output and temporary files";
        const OPERATIONS_HELP: &'static str = "Operations to perform";
        const DISABLE_LOGGING_HELP: &'static str = "Disable creating log files";

        let crates_help = format!(
            "Process libraries (Qt modules). Specify \"all\" \
             to process all supported modules or specify one or multiple of the following: {}.",
            all_crate_names().join(", ")
        );

        run(App::new("qt_generator")
            .about(ABOUT)
            .after_help(AFTER_HELP)
            .arg(
                Arg::with_name("workspace")
                    .short("w")
                    .long("workspace")
                    .value_name("WORKSPACE")
                    .help(WORKSPACE_DIR_HELP)
                    .takes_value(true)
                    .required(true),
            )
            .arg(
                Arg::with_name("crates")
                    .short("c")
                    .long("crates")
                    .value_name("crate_name1 crate_name2")
                    .help(&crates_help)
                    .takes_value(true)
                    .multiple(true)
                    .required(true)
                    .use_delimiter(false),
            )
            .arg(
                Arg::with_name("operations")
                    .short("op")
                    .long("operations")
                    .value_name("operation1 operation2")
                    .help(&OPERATIONS_HELP)
                    .takes_value(true)
                    .multiple(true)
                    .required(true)
                    .use_delimiter(false),
            )
            .arg(
                Arg::with_name("disable-logging")
                    .long("disable-logging")
                    .help(DISABLE_LOGGING_HELP),
            )
            .get_matches())
    };
    if let Err(err) = result {
        err.display_report();
        ::std::process::exit(1);
    }
}
