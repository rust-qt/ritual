//! Generator of Rust-Qt crates.
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust/tree/master/qt_generator/qt_generator)
//! for more information.

use crate::config::GlobalConfig;
use crate::processor;
use crate::workspace::Workspace;
use flexi_logger::{Duplicate, LevelFilter, LogSpecification, Logger};
use itertools::Itertools;
use log::{error, info};
use ritual_common::errors::{bail, err_msg, Result};
use ritual_common::file_utils::{canonicalize, create_dir, path_to_str};
use ritual_common::target::current_target;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
/// Generates rust_qt crates using ritual.
/// See [ritual](https://github.com/rust-qt/ritual) for more details.
pub struct Options {
    #[structopt(parse(from_os_str))]
    /// Directory for output and temporary files
    pub workspace: PathBuf,
    #[structopt(long = "local-paths")]
    /// Write local paths to `ritual` crates in generated `Cargo.toml`
    pub local_paths: Option<bool>,
    #[structopt(short = "c", long = "crates", required = true)]
    /// Crates to process (e.g. `qt_core`)
    pub crates: Vec<String>,
    #[structopt(short = "o", long = "operations", required = true)]
    /// Operations to perform
    pub operations: Vec<String>,
}

pub fn run_from_args(config: GlobalConfig) -> Result<()> {
    run(Options::from_args(), config)
}

pub fn run(options: Options, mut config: GlobalConfig) -> Result<()> {
    if !options.workspace.exists() {
        create_dir(&options.workspace)?;
    }
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
    info!("Current target: {}", current_target().short_text());

    if let Some(local_paths) = options.local_paths {
        workspace.set_write_dependencies_local_paths(local_paths)?;
    }

    let mut was_any_action = false;

    let final_crates = if options.crates.iter().any(|x| *x == "all") {
        let all = config.all_crate_names();
        if all.is_empty() {
            bail!("\"all\" is not supported as crate name specifier");
        }
        all.to_vec()
    } else {
        options.crates.clone()
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
        let create_config = config
            .create_config_hook()
            .ok_or_else(|| err_msg("create_config_hook is missing"))?;

        let config = create_config(&crate_name)?;
        was_any_action = true;
        processor::process(&mut workspace, &config, &operations)?;
    }

    //workspace.save_data()?;
    if was_any_action {
        info!("ritual finished");
    } else {
        error!("No action requested. Run \"qt_generator --help\".");
    }
    Ok(())
}
