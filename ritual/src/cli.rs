//! Generator of Rust-Qt crates.
//!
//! See [README](https://github.com/rust-qt/ritual)
//! for more information.

use crate::config::{CrateDependencySource, CrateProperties, GlobalConfig};
use crate::cpp_parser::{self, CppParserContext};
use crate::workspace::Workspace;
use clap::{Parser, Subcommand};
use flexi_logger::{Duplicate, LevelFilter, LogSpecification, Logger};
use itertools::Itertools;
use log::info;
use ritual_common::errors::{bail, err_msg, Result};
use ritual_common::file_utils::{canonicalize, create_dir, path_to_str};
use ritual_common::target::current_target;
use std::path::PathBuf;

#[derive(Debug, Parser)]
/// Generates rust_qt crates using ritual.
/// See [ritual](https://github.com/rust-qt/ritual) for more details.
pub struct Options {
    #[arg(short, long)]
    /// Directory for output and temporary files
    pub workspace: PathBuf,
    #[arg(short, long)]
    /// Write local paths to `ritual` crates in generated `Cargo.toml`
    pub local_paths: Option<bool>,
    #[arg(short, long, required = true)]
    /// Crates to process (e.g. `qt_core`)
    pub crates: Vec<String>,
    #[arg(long)]
    /// Version of the output crates.
    pub output_crates_version: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Parse,
}

pub fn run_from_args(config: GlobalConfig) -> Result<()> {
    run(Options::parse(), config)
}

pub fn run(options: Options, mut config: GlobalConfig) -> Result<()> {
    if !options.workspace.exists() {
        create_dir(&options.workspace)?;
    }
    let workspace_path = canonicalize(options.workspace)?;

    let workspace = Workspace::new(workspace_path.clone())?;

    Logger::with(LogSpecification::default(LevelFilter::Trace).build())
        .log_to_file()
        .directory(path_to_str(&workspace.log_path())?)
        .suppress_timestamp()
        .append()
        .print_message()
        .duplicate_to_stderr(Duplicate::Info)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed: {}", e));

    info!("");
    info!("Workspace: {}", workspace_path.display());
    info!("Current target: {}", current_target().short_text());

    let final_crates = if options.crates.iter().any(|x| *x == "all") {
        let all = config.all_crate_names();
        if all.is_empty() {
            bail!("\"all\" is not supported as crate name specifier");
        }
        all.to_vec()
    } else {
        options.crates.clone()
    };

    for crate_name in &final_crates {
        let create_config = config
            .create_config_hook()
            .ok_or_else(|| err_msg("create_config_hook is missing"))?;

        let mut config = create_config(CrateProperties::new(
            crate_name,
            options.output_crates_version.as_deref().unwrap_or("0.1"),
        ))?;

        if let Some(local_paths) = options.local_paths {
            config.set_write_dependencies_local_paths(local_paths);
        }

        match options.command {
            Command::Parse => {
                let mut deps = Vec::new();
                for dep in config.crate_properties().dependencies() {
                    if dep.source() == &CrateDependencySource::CurrentWorkspace {
                        deps.push(workspace.load_database2(dep.name(), false)?);
                    }
                }
                let mut main_db = workspace.load_database2(crate_name, true)?;
                let data = CppParserContext {
                    current_database: &mut main_db,
                    dependencies: &deps.iter().collect_vec(),
                    config: &config,
                    workspace: &workspace,
                };
                cpp_parser::run(data)?;
                workspace.save_database2(&main_db)?;
            }
        }
    }

    info!("ritual finished");
    Ok(())
}
