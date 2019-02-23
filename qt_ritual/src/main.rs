//! Generator of Rust-Qt crates.
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust/tree/master/qt_generator/qt_generator)
//! for more information.

#![allow(clippy::collapsible_if)]

use crate::lib_configs::create_config;
use itertools::Itertools;
use log::{error, info};
use qt_ritual_common::all_crate_names;
use ritual::processor;
use ritual::workspace::Workspace;
use ritual_common::errors::{FancyUnwrap, Result};
use ritual_common::file_utils::canonicalize;
use ritual_common::file_utils::path_to_str;
use std::path::PathBuf;
use structopt::StructOpt;
mod detect_signal_argument_types;
mod detect_signals_and_slots;
mod doc_decoder;
mod doc_parser;
mod fix_header_names;
mod lib_configs;
mod slot_wrappers;
mod versions;

#[cfg(test)]
#[cfg(target_os = "linux")] // TODO: fix on Windows and MacOS
mod test_moqt;

use flexi_logger::{Duplicate, LevelFilter, LogSpecification, Logger};

#[derive(Debug, StructOpt)]
/// Generates rust_qt crates using ritual.
/// See https://github.com/rust-qt/cpp_to_rust for more details.
struct Options {
    #[structopt(parse(from_os_str))]
    /// Directory for output and temporary files
    workspace: PathBuf,
    #[structopt(long = "local-paths")]
    /// Write local paths to `ritual` crates in generated `Cargo.toml`
    local_paths: Option<bool>,
    #[structopt(long = "delete-database")]
    /// Delete previously created database before processing
    delete_database: bool,
    #[structopt(short = "c", long = "crates", required = true)]
    /// Crates to process (e.g. `qt_core`)
    crates: Vec<String>,
    #[structopt(short = "o", long = "operations", required = true)]
    /// Operations to perform
    operations: Vec<String>,
}

fn run(options: Options) -> Result<()> {
    let workspace_path = canonicalize(options.workspace)?;

    let mut workspace = Workspace::new(workspace_path.clone())?;

    Logger::with(LogSpecification::default(LevelFilter::Trace).build())
        .log_to_file()
        .directory(path_to_str(&workspace.log_path())?)
        .print_message()
        .duplicate_to_stderr(Duplicate::Info)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed: {}", e));

    info!("Workspace: {}", workspace_path.display());

    if let Some(local_paths) = options.local_paths {
        workspace.set_write_dependencies_local_paths(local_paths)?;
    }

    let mut was_any_action = false;

    let final_crates = if options.crates.iter().any(|x| *x == "all") {
        all_crate_names().to_vec()
    } else {
        options.crates.iter().map(|s| s.as_str()).collect()
    };

    let operations = options
        .operations
        .iter()
        .map(|s| s.to_lowercase())
        .collect_vec();

    if operations.is_empty() {
        error!("No action requested. Run \"qt_generator --help\".");
        return Ok(());
    }

    for crate_name in &final_crates {
        if options.delete_database {
            workspace.delete_database_if_exists(&crate_name)?;
        }

        let config = create_config(&crate_name)?;
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
    run(Options::from_args()).fancy_unwrap();
}
