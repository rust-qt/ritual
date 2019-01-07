//! Generator of Rust-Qt crates.
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust/tree/master/qt_generator/qt_generator)
//! for more information.

#![allow(clippy::collapsible_if)]

use crate::lib_configs::make_config;
use log::{error, info};
use qt_ritual_common::all_crate_names;
use ritual::processor;
use ritual::workspace::Workspace;
use ritual_common::errors::{err_msg, FancyUnwrap, Result};
use ritual_common::file_utils::canonicalize;
use ritual_common::file_utils::path_to_str;
use std::path::PathBuf;
mod detect_signal_argument_types;
mod detect_signals_and_slots;
mod doc_decoder;
mod doc_parser;
mod fix_header_names;
mod lib_configs;
mod versions;
use flexi_logger::{Duplicate, LevelFilter, LogSpecification, Logger};

fn run(matches: &clap::ArgMatches) -> Result<()> {
    let workspace_path = canonicalize(&PathBuf::from(
        matches
            .value_of("workspace")
            .ok_or_else(|| err_msg("clap arg missing"))?,
    ))?;

    info!("Workspace: {}", workspace_path.display());
    let mut workspace = Workspace::new(workspace_path)?;

    Logger::with(LogSpecification::default(LevelFilter::Trace).build())
        .log_to_file()
        .directory(path_to_str(&workspace.log_path()?)?)
        .print_message()
        .duplicate_to_stderr(Duplicate::Info)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    if let Some(value) = matches.value_of("local-paths") {
        workspace.set_write_dependencies_local_paths(value == "1")?;
    }

    let mut was_any_action = false;

    let crates: Vec<_> = matches
        .values_of("crates")
        .ok_or_else(|| err_msg("clap arg missing"))?
        .collect();

    let final_crates = if crates.iter().any(|x| *x == "all") {
        all_crate_names().to_vec()
    } else {
        crates
    };

    let operations: Vec<_> = matches
        .values_of("operations")
        .ok_or_else(|| err_msg("clap arg missing"))?
        .map(|s| s.to_lowercase())
        .collect();

    if operations.is_empty() {
        error!("No action requested. Run \"qt_generator --help\".");
        return Ok(());
    }

    for crate_name in &final_crates {
        let config = make_config(&crate_name)?;
        was_any_action = true;
        processor::process(&mut workspace, &config, &operations)?;
    }

    //workspace.save_data()?;
    if was_any_action {
        info!("qt_generator finished");
    } else {
        error!("No action requested. Run \"qt_generator --help\".");
    }
    Ok(())
}

pub fn main() {
    use clap::{App, Arg};
    const ABOUT: &str = "Generates rust_qt crates using cpp_to_rust";
    const AFTER_HELP: &str = "\
                              Example:\n    qt_generator -w /path/to/workspace -p all -g\n\n\
                              See https://github.com/rust-qt/cpp_to_rust for more details.";
    const WORKSPACE_DIR_HELP: &str = "Directory for output and temporary files";
    const OPERATIONS_HELP: &str = "Operations to perform";
    //const DISABLE_LOGGING_HELP: &str = "Disable creating log files";

    let crates_help = format!(
        "Process libraries (Qt modules). Specify \"all\" \
         to process all supported modules or specify one or multiple of the following: {}.",
        all_crate_names().join(", ")
    );

    let args = App::new("qt_generator")
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
        //        .arg(
        //            Arg::with_name("disable-logging")
        //                .long("disable-logging")
        //                .help(DISABLE_LOGGING_HELP),
        //        )
        .arg(
            Arg::with_name("local-paths")
                .long("local-paths")
                .takes_value(true),
        )
        .get_matches();

    run(&args).fancy_unwrap();
}
