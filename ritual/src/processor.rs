use crate::config::Config;
use crate::database::{CppDatabaseItem, CppFfiDatabaseItem, Database};
use crate::workspace::Workspace;
use crate::{
    cpp_checker, cpp_explicit_xstructors, cpp_ffi_generator, cpp_omitting_arguments, cpp_parser,
    cpp_template_instantiator, crate_writer, rust_generator,
};
use itertools::Itertools;
use log::{error, info, trace};
use ritual_common::env_var_names;
use ritual_common::errors::{bail, err_msg, format_err, Result, ResultExt};
use ritual_common::utils::{run_command, MapIfOk};
use std::cmp::Ordering;
use std::fmt;
use std::iter::once;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

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
    pub fn all_databases(&self) -> impl Iterator<Item = &Database> {
        once(&self.current_database as &_).chain(self.dep_databases.iter())
    }
    pub fn all_cpp_items(&self) -> impl Iterator<Item = &CppDatabaseItem> {
        once(&self.current_database as &_)
            .chain(self.dep_databases.iter())
            .flat_map(|d| d.cpp_items().iter())
    }
    pub fn all_ffi_items(&self) -> impl Iterator<Item = &CppFfiDatabaseItem> {
        once(&self.current_database as &_)
            .chain(self.dep_databases.iter())
            .flat_map(|d| d.ffi_items().iter())
    }
}

struct ProcessingStep {
    name: String,
    function: Box<dyn Fn(&mut ProcessorData<'_>) -> Result<()>>,
}

impl fmt::Debug for ProcessingStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessingStep")
            .field("name", &self.name)
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
        let mut s = ProcessingSteps {
            all_steps: Vec::new(),
            main_procedure: Vec::new(),
        };

        let push_cpp_post_processing = |s: &mut Self, suffix: &str| {
            s.push(
                &format!("add_explicit_xstructors{}", suffix),
                cpp_explicit_xstructors::run,
            );
            //            s.push(
            //                &format!("set_allocation_places{}", suffix),
            //                type_allocation_places::set_allocation_places,
            //            );
            s.push(
                &format!("find_template_instantiations{}", suffix),
                cpp_template_instantiator::find_template_instantiations,
            );
            s.push(
                &format!("instantiate_templates{}", suffix),
                cpp_template_instantiator::instantiate_templates,
            );
            s.push(
                &format!("omitting_arguments{}", suffix),
                cpp_omitting_arguments::run,
            );
            s.push(
                &format!("cpp_ffi_generator{}", suffix),
                cpp_ffi_generator::run,
            );
            s.push(&format!("cpp_checker{}", suffix), cpp_checker::run);
        };

        s.push("cpp_parser", cpp_parser::run);
        push_cpp_post_processing(&mut s, "");
        s.push("cpp_parser_stage2", cpp_parser::parse_generated_items);
        push_cpp_post_processing(&mut s, "_stage2");
        s.push("rust_generator", rust_generator::run);
        s.push("crate_writer", crate_writer::run);
        s.push("build_crate", build_crate);

        s.add_custom("clear_ffi", |data| {
            data.current_database.clear_ffi();
            Ok(())
        });
        s.add_custom("clear_rust_info", |data| {
            data.current_database.clear_rust_info();
            Ok(())
        });

        //        s.add_custom(
        //            "suggest_allocation_places",
        //            type_allocation_places::suggest_allocation_places,
        //        );
        s
    }
}

impl ProcessingSteps {
    pub fn add_after(
        &mut self,
        after: &[&str],
        name: &str,
        func: impl Fn(&mut ProcessorData<'_>) -> Result<()> + 'static,
    ) -> Result<()> {
        let indexes = after.iter().map_if_ok(|s| {
            self.main_procedure
                .iter()
                .position(|a| a == s)
                .ok_or_else(|| format_err!("requested step not found: {}", s))
        })?;

        let max_index = indexes
            .into_iter()
            .max()
            .ok_or_else(|| err_msg("no steps provided"))?;
        self.main_procedure.insert(max_index + 1, name.to_string());
        self.all_steps.push(ProcessingStep::new(name, func));
        Ok(())
    }

    pub fn push(
        &mut self,
        name: &str,
        func: impl Fn(&mut ProcessorData<'_>) -> Result<()> + 'static,
    ) {
        self.main_procedure.push(name.to_string());
        self.all_steps.push(ProcessingStep::new(name, func));
    }

    pub fn add_custom(
        &mut self,
        name: &str,
        func: impl Fn(&mut ProcessorData<'_>) -> Result<()> + 'static,
    ) {
        self.all_steps.push(ProcessingStep::new(name, func));
    }
}

impl ProcessingStep {
    pub fn new<S: Into<String>, F: 'static + Fn(&mut ProcessorData<'_>) -> Result<()>>(
        name: S,
        function: F,
    ) -> Self {
        ProcessingStep {
            name: name.into(),
            function: Box::new(function),
        }
    }
}

fn build_crate(data: &mut ProcessorData<'_>) -> Result<()> {
    data.workspace.update_cargo_toml()?;
    let path = data.workspace.path();
    let crate_name = data.config.crate_properties().name();
    //run_command(Command::new("cargo").arg("update").current_dir(path))?;

    for cargo_cmd in &["build", "test", "doc"] {
        let mut command = Command::new("cargo");
        command
            .arg(cargo_cmd)
            .arg("-p")
            .arg(crate_name)
            .current_dir(path);
        if cargo_cmd == &"doc" {
            command.env(env_var_names::RUSTDOC, "1");
        }
        run_command(&mut command)?;
    }
    Ok(())
}

#[derive(Debug)]
struct MainItemRef<'a> {
    step: &'a ProcessingStep,
    run_after: &'a [String],
}

impl PartialEq for MainItemRef<'_> {
    fn eq(&self, other: &MainItemRef<'_>) -> bool {
        self.step.name == other.step.name
    }
}

impl PartialOrd for MainItemRef<'_> {
    fn partial_cmp(&self, other: &MainItemRef<'_>) -> Option<Ordering> {
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
pub fn process(
    workspace: &mut Workspace,
    config: &Config,
    mut step_names: &[String],
) -> Result<()> {
    info!("Processing crate: {}", config.crate_properties().name());
    check_all_paths(&config)?;

    let allow_load;
    if step_names.get(0).map(|s| s.as_str()) == Some("discard") {
        allow_load = false;
        step_names = &step_names[1..];
    } else {
        allow_load = true;
        info!("Loading current crate data");
    }

    let mut current_database = workspace
        .get_database(config.crate_properties().name(), allow_load, true)
        .with_context(|_| "failed to load current crate data")?;

    if !config.dependent_cpp_crates().is_empty() {
        info!("Loading dependencies");
    }
    let dependent_cpp_crates = config.dependent_cpp_crates().iter().map_if_ok(|name| {
        workspace
            .get_database(name, true, false)
            .with_context(|_| "failed to load dependency")
    })?;

    current_database.set_crate_version(config.crate_properties().version().to_string());

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
                .ok_or_else(|| format_err!("requested step not found: {}", start_step))?;
            config.processing_steps().main_procedure[start_index..].to_vec()
        } else if step_name.starts_with("until:") {
            let end_step = &step_name["until:".len()..];
            let end_index = config
                .processing_steps()
                .main_procedure
                .iter()
                .position(|s| s == end_step)
                .ok_or_else(|| format_err!("requested step not found: {}", end_step))?;
            config.processing_steps().main_procedure[..end_index].to_vec()
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

            let started_time = Instant::now();

            if let Err(err) = (step.function)(&mut data) {
                steps_result = Err(err);
                error!("Step failed! Aborting...");
                break;
            }

            let elapsed = started_time.elapsed();
            trace!("Step '{}' completed in {:?}", step.name, elapsed);

            if elapsed > Duration::from_secs(15) {
                workspace.save_database(&mut current_database)?;
            }
        }
    }

    for database in dependent_cpp_crates {
        workspace.put_crate(database);
    }

    workspace.save_database(&mut current_database)?;
    workspace.put_crate(current_database);

    steps_result
}
