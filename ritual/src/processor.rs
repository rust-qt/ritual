use ritual_common::errors::{bail, err_msg, Result, ResultExt};

use crate::config::Config;
use crate::cpp_checker::cpp_checker_step;
use crate::cpp_explicit_destructors::add_explicit_destructors_step;
use crate::cpp_ffi_generator::cpp_ffi_generator_step;
use crate::cpp_parser::cpp_parser_step;
use crate::cpp_template_instantiator::find_template_instantiations_step;
use crate::cpp_template_instantiator::instantiate_templates_step;
use crate::crate_writer::crate_writer_step;
use crate::database::{CppDatabaseItem, Database};
use crate::rust_generator::clear_rust_info_step;
use crate::rust_generator::rust_generator_step;
use crate::type_allocation_places::choose_allocation_places_step;
use crate::workspace::Workspace;
use itertools::Itertools;
use log::{error, info};
use ritual_common::utils::MapIfOk;
use std::cmp::Ordering;
use std::fmt;
use std::iter::once;
use std::path::PathBuf;
//use cpp_post_processor::cpp_post_process;

/// Creates output and cache directories if they don't exist.
/// Returns `Err` if any path in `config` is invalid or relative.
fn check_all_paths(config: &Config) -> Result<()> {
    let check_dir = |path: &PathBuf| -> Result<()> {
        if !path.is_absolute() {
            bail!(
                "Only absolute paths allowed. Relative path: {}",
                path.display()
            );
        }
        if !path.exists() {
            bail!("Directory doesn't exist: {}", path.display());
        }
        if !path.is_dir() {
            bail!("Path is not a directory: {}", path.display());
        }
        Ok(())
    };

    if let Some(path) = config.crate_template_path() {
        check_dir(path)?;
    }
    for path in config.cpp_build_paths().include_paths() {
        check_dir(path)?;
    }
    for path in config.cpp_build_paths().lib_paths() {
        check_dir(path)?;
    }
    for path in config.cpp_build_paths().framework_paths() {
        check_dir(path)?;
    }
    for path in config.target_include_paths() {
        check_dir(path)?;
    }
    Ok(())
}

pub struct ProcessorData<'a> {
    pub workspace: &'a mut Workspace,
    pub config: &'a Config,
    pub current_database: &'a mut Database,
    pub dep_databases: &'a [Database],
}

impl<'a> ProcessorData<'a> {
    pub fn all_databases(&self) -> Vec<&Database> {
        once(&self.current_database as &_)
            .chain(self.dep_databases.iter())
            .collect()
    }
    pub fn all_items(&self) -> Vec<&CppDatabaseItem> {
        once(&self.current_database as &_)
            .chain(self.dep_databases.iter())
            .flat_map(|d| d.cpp_items.iter())
            .collect()
    }
}

pub struct ProcessingStep {
    pub name: String,
    pub is_const: bool,
    pub function: Box<Fn(&mut ProcessorData) -> Result<()>>,
}

impl fmt::Debug for ProcessingStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessingStep")
            .field("name", &self.name)
            .field("is_const", &self.is_const)
            .finish()
    }
}

#[derive(Debug)]
pub struct ProcessingSteps {
    all_steps: Vec<ProcessingStep>,
    main_procedure: Vec<String>,
}

impl Default for ProcessingSteps {
    fn default() -> Self {
        let main_steps = vec![
            cpp_parser_step(),
            add_explicit_destructors_step(),
            choose_allocation_places_step(),
            find_template_instantiations_step(),
            instantiate_templates_step(),
            cpp_ffi_generator_step(),
            // TODO: generate_slot_wrappers
            cpp_checker_step(),
            rust_generator_step(),
            crate_writer_step(),
            steps::build_crate(),
        ];

        let main_procedure = main_steps.iter().map(|s| s.name.clone()).collect();

        let mut all_steps = main_steps;
        all_steps.extend(vec![
            steps::print_database(),
            steps::clear_ffi(),
            clear_rust_info_step(),
        ]);
        Self {
            all_steps,
            main_procedure,
        }
    }
}

impl ProcessingSteps {
    pub fn add_after(&mut self, after: &[&str], step: ProcessingStep) -> Result<()> {
        let indexes: Vec<usize> = after.iter().map_if_ok(|s| {
            self.main_procedure
                .iter()
                .position(|a| a == s)
                .ok_or_else(|| err_msg(format!("requested step not found: {}", s)))
        })?;

        let max_index = indexes
            .into_iter()
            .max()
            .ok_or_else(|| err_msg("no steps provided"))?;
        self.main_procedure.insert(max_index + 1, step.name.clone());
        self.all_steps.push(step);
        Ok(())
    }
}

impl ProcessingStep {
    pub fn new<S: Into<String>, F: 'static + Fn(&mut ProcessorData) -> Result<()>>(
        name: S,
        function: F,
    ) -> ProcessingStep {
        ProcessingStep {
            name: name.into(),
            is_const: false,
            function: Box::new(function),
        }
    }
    pub fn new_const<S: Into<String>, F: 'static + Fn(&mut ProcessorData) -> Result<()>>(
        name: S,
        function: F,
    ) -> ProcessingStep {
        let name = name.into();
        ProcessingStep {
            name,
            is_const: true,
            function: Box::new(function),
        }
    }
}

mod steps {
    use crate::processor::ProcessingStep;
    use log::trace;
    use ritual_common::utils::run_command;
    use std::process::Command;

    pub fn print_database() -> ProcessingStep {
        ProcessingStep::new_const("print_database", |data| {
            for item in &data.current_database.cpp_items {
                trace!(
                    "[database_item] cpp_data={}; source={:?}",
                    item.cpp_data.to_string(),
                    item.source
                );
                for ffi_item in &item.ffi_items {
                    trace!("[database_item]   ffi_item={:?}", ffi_item);
                }
            }
            Ok(())
        })
    }

    pub fn clear_ffi() -> ProcessingStep {
        ProcessingStep::new("clear_ffi", |data| {
            for item in &mut data.current_database.cpp_items {
                item.ffi_items.clear();
                item.is_cpp_ffi_processed = false;
            }
            Ok(())
        })
    }

    pub fn build_crate() -> ProcessingStep {
        ProcessingStep::new_const("build_crate", |data| {
            let path = data
                .workspace
                .crate_path(&data.current_database.crate_name)?;
            for cargo_cmd in &["update", "build", "test", "doc"] {
                let mut command = Command::new("cargo");
                command.arg(cargo_cmd);
                command.current_dir(&path);
                run_command(&mut command)?;
            }
            Ok(())
        })
    }
}

#[derive(Debug)]
struct MainItemRef<'a> {
    step: &'a ProcessingStep,
    run_after: &'a [String],
}

impl PartialEq for MainItemRef<'_> {
    fn eq(&self, other: &MainItemRef) -> bool {
        self.step.name == other.step.name
    }
}

impl PartialOrd for MainItemRef<'_> {
    fn partial_cmp(&self, other: &MainItemRef) -> Option<Ordering> {
        if self.run_after.contains(&other.step.name) {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Less)
        }
    }
}
impl Eq for MainItemRef<'_> {}
impl Ord for MainItemRef<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[allow(clippy::useless_let_if_seq)]
pub fn process(workspace: &mut Workspace, config: &Config, step_names: &[String]) -> Result<()> {
    info!("Processing crate: {}", config.crate_properties().name());
    check_all_paths(&config)?;

    // TODO: allow to remove any prefix through `Config` (#25)
    #[allow(unused_variables)]
    let remove_qt_prefix = config.crate_properties().name().starts_with("qt_");

    info!("Loading current crate data");
    let mut current_database = workspace
        .load_or_create_crate(config.crate_properties().name())
        .with_context(|_| "failed to load current crate data")?;

    if !config.dependent_cpp_crates().is_empty() {
        info!("Loading dependencies");
    }
    let dependent_cpp_crates = config.dependent_cpp_crates().iter().map_if_ok(|name| {
        workspace
            .load_crate(name)
            .with_context(|_| "failed to load dependency")
    })?;

    let mut current_database_saved = true;

    if current_database.crate_version != config.crate_properties().version() {
        current_database.crate_version = config.crate_properties().version().to_string();
        current_database_saved = false;
    }

    let mut steps_result = Ok(());

    for step_name in step_names {
        if steps_result.is_err() {
            break;
        }

        let step_names = if step_name == "main" {
            config.processing_steps().main_procedure.clone()
        } else if step_name.starts_with("from:") {
            let start_step = &step_name["from:".len()..];
            let start_index = config
                .processing_steps()
                .main_procedure
                .iter()
                .position(|s| s == start_step)
                .ok_or_else(|| err_msg(format!("requested step not found: {}", start_step)))?;
            config.processing_steps().main_procedure[start_index..].to_vec()
        } else {
            vec![step_name.to_string()]
        };

        for step_name in step_names {
            let step = if let Some(item) = config
                .processing_steps()
                .all_steps
                .iter()
                .find(|item| item.name == step_name)
            {
                item
            } else {
                bail!(
                    "Unknown operation: {}. Supported operations: main, {}.",
                    step_name,
                    config
                        .processing_steps()
                        .all_steps
                        .iter()
                        .map(|item| &item.name)
                        .join(", ")
                );
            };

            info!("Running processing step: {}", &step.name);

            let mut data = ProcessorData {
                workspace,
                current_database: &mut current_database,
                dep_databases: &dependent_cpp_crates,
                config,
            };

            if !step.is_const {
                current_database_saved = false;
            }

            if let Err(err) = (step.function)(&mut data) {
                steps_result = Err(err);
                error!("Step failed! Aborting...");
                break;
            }
        }
    }

    /*
    if exec_config.write_dependencies_local_paths {
    log::status(
    "Output Cargo.toml file will contain local paths of used dependencies \
      (use --no-local-paths to disable).",
    );
    } else {
    log::status(
    "Local paths will not be written to the output crate. Make sure all dependencies \
      are published before trying to compile the crate.",
    );
    }

    */

    /*
    parser_cpp_data.detect_signals_and_slots(
      &dependent_cpp_crates,
    )?;
    // TODO: rename `cpp_data_filters` to `parser_cpp_data_filters`
    if config.has_cpp_data_filters() {
      log::status("Running custom filters for C++ parser data");
      for filter in config.cpp_data_filters() {
        filter(&mut parser_cpp_data).with_context(
          || "cpp_data_filter failed",
        )?;
      }
    }

    log::status("Post-processing parse result");
    let r = cpp_post_process(
      parser_cpp_data,
      dependent_cpp_crates,
      config.type_allocation_places(),
    )?;

    //...

    */

    for database in dependent_cpp_crates {
        workspace.put_crate(database, true);
    }

    if !current_database_saved {
        info!("Saving data");
    }

    workspace.put_crate(current_database, current_database_saved);
    workspace.save_data()?;

    steps_result
}
