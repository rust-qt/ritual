//! Generator of Rust-Qt crates.
//!
//! See [README](https://github.com/rust-qt/ritual)
//! for more information.

use crate::config::{CrateDependencySource, CrateProperties, GlobalConfig};
use crate::cpp_parser::{self, Context2};
use crate::workspace::Workspace;
use crate::{crate_writer, rustifier, search_db};
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
    /// One or more (comma separated) crates to process (e.g. `qt_core,qt_gui`) or "all"
    pub crates: String,
    #[arg(long)]
    /// Version of the output crates.
    pub output_crates_version: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Parse,
    Generate,
    Search { name: String },
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

    Logger::with(LogSpecification::default(LevelFilter::Debug).build())
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

    let crates = if options.crates == "all" {
        let all = config.all_crate_names();
        if all.is_empty() {
            bail!("\"all\" is not supported as crate name specifier");
        }
        all.to_vec()
    } else {
        options
            .crates
            .split(',')
            .map(|s| s.to_string())
            .collect_vec()
    };

    match &options.command {
        Command::Parse | Command::Generate => {
            for crate_name in &crates {
                let create_config = config
                    .create_config_hook()
                    .ok_or_else(|| err_msg("create_config_hook is missing"))?;

                let mut config = create_config(CrateProperties::new(
                    crate_name,
                    options.output_crates_version.as_deref().unwrap_or("0.1.0"),
                ))?;

                if let Some(local_paths) = options.local_paths {
                    config.set_write_dependencies_local_paths(local_paths);
                }

                let mut deps = Vec::new();
                for dep in config.crate_properties().dependencies() {
                    if dep.source() == &CrateDependencySource::CurrentWorkspace {
                        deps.push(workspace.load_database2(dep.name())?);
                    }
                }
                let mut main_db = workspace.load_or_create_database2(
                    config.crate_properties().name(),
                    config.crate_properties().version(),
                )?;
                let mut ctx = Context2 {
                    current_database: &mut main_db,
                    dependencies: &deps.iter().collect_vec(),
                    config: &config,
                    workspace: &workspace,
                };

                match &options.command {
                    Command::Parse => {
                        info!("running cpp parser");
                        cpp_parser::run(ctx)?;
                        info!("saving database");
                        workspace.save_database2(&main_db)?;
                    }
                    Command::Generate => {
                        info!("generating crate");
                        let code = rustifier::run(ctx.reborrow())?;
                        crate_writer::run(ctx, &code)?;
                    }
                    Command::Search { .. } => unreachable!(),
                }
            }
        }
        Command::Search { name } => {
            let mut dbs = Vec::new();
            for c in crates {
                dbs.push(workspace.load_database2(&c)?);
            }
            search_db::run(&dbs, name);
        }
    }

    info!("ritual finished");
    Ok(())
}
