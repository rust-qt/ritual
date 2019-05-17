use crate::config::Config;
use crate::database::{CppDatabaseItem, CppFfiDatabaseItem, Database};
use crate::workspace::Workspace;
use crate::{
    cpp_checker, cpp_ffi_generator, cpp_implicit_methods, cpp_omitting_arguments, cpp_parser,
    cpp_template_instantiator, crate_writer, rust_generator,
};
use itertools::Itertools;
use log::{error, info, trace};
use regex::Regex;
use ritual_common::env_var_names;
use ritual_common::errors::{bail, err_msg, format_err, Result, ResultExt};
use ritual_common::utils::{run_command, MapIfOk};
use std::cmp::Ordering;
use std::fmt;
use std::iter::once;
use std::ops::Bound;
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
                &format!("add_implicit_methods{}", suffix),
                cpp_implicit_methods::run,
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
        s.add_custom("clear_cpp_checks", |data| {
            data.current_database.clear_cpp_checks();
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

    if let Some(version) = config.cpp_lib_version() {
        info!("Current C++ library version: {}", version);
    }

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

    let step_index = |name| {
        config
            .processing_steps()
            .main_procedure
            .iter()
            .position(|s| s == &name)
            .ok_or_else(|| format_err!("requested step not found: {}", name))
    };

    let step_ranges = step_names.iter().map_if_ok(|step_name| {
        let range = parse_steps_spec(step_name)?;
        let start_index = match range.0 {
            Bound::Included(name) => step_index(name)?,
            Bound::Excluded(name) => step_index(name)? + 1,
            Bound::Unbounded => 0,
        };
        let end_index = match range.1 {
            Bound::Included(name) => step_index(name)? + 1,
            Bound::Excluded(name) => step_index(name)?,
            Bound::Unbounded => config.processing_steps().main_procedure.len(),
        };
        let range = config
            .processing_steps()
            .main_procedure
            .get(start_index..end_index)
            .ok_or_else(|| err_msg("invalid steps range"))?;
        if range.is_empty() {
            bail!("empty steps range");
        }
        Ok(range)
    })?;

    for step_range in step_ranges {
        if steps_result.is_err() {
            break;
        }

        for step_name in step_range {
            let step = if let Some(item) = config
                .processing_steps()
                .all_steps
                .iter()
                .find(|item| &item.name == step_name)
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

fn parse_steps_spec(text: &str) -> Result<(Bound<String>, Bound<String>)> {
    if text == "main" {
        return Ok((Bound::Unbounded, Bound::Unbounded));
    }

    let re = Regex::new(r"^[[:word:]]+$").unwrap();
    if re.is_match(text) {
        return Ok((
            Bound::Included(text.to_string()),
            Bound::Included(text.to_string()),
        ));
    }

    let re = Regex::new(r"^([\[\(])([[:word:]]*)\.\.([[:word:]]*)([\]\)])$").unwrap();
    let captures = re
        .captures(text)
        .ok_or_else(|| err_msg("invalid step range"))?;

    fn parse_bound(step: &str, bound_char: &str) -> Result<Bound<String>> {
        Ok(match bound_char {
            "[" | "]" => {
                if step.is_empty() {
                    Bound::Unbounded
                } else {
                    Bound::Included(step.to_string())
                }
            }
            "(" | ")" => {
                if step.is_empty() {
                    bail!("invalid bound")
                } else {
                    Bound::Excluded(step.to_string())
                }
            }
            _ => bail!("invalid bound"),
        })
    }

    let from_bound = parse_bound(&captures[2], &captures[1])?;
    let to_bound = parse_bound(&captures[3], &captures[4])?;
    Ok((from_bound, to_bound))
}

#[test]
fn test_parse_steps_spec() {
    assert_eq!(
        parse_steps_spec("main").unwrap(),
        (Bound::Unbounded, Bound::Unbounded)
    );
    assert_eq!(
        parse_steps_spec("t1").unwrap(),
        (
            Bound::Included("t1".to_string()),
            Bound::Included("t1".to_string())
        )
    );
    assert_eq!(
        parse_steps_spec("[..]").unwrap(),
        (Bound::Unbounded, Bound::Unbounded)
    );
    assert_eq!(
        parse_steps_spec("[t1..]").unwrap(),
        (Bound::Included("t1".to_string()), Bound::Unbounded)
    );
    assert_eq!(
        parse_steps_spec("(t1..]").unwrap(),
        (Bound::Excluded("t1".to_string()), Bound::Unbounded)
    );
    assert_eq!(
        parse_steps_spec("[..t1)").unwrap(),
        (Bound::Unbounded, Bound::Excluded("t1".to_string()))
    );
    assert_eq!(
        parse_steps_spec("[..t1]").unwrap(),
        (Bound::Unbounded, Bound::Included("t1".to_string()))
    );
    assert_eq!(
        parse_steps_spec("[t1..t2]").unwrap(),
        (
            Bound::Included("t1".to_string()),
            Bound::Included("t2".to_string())
        )
    );
    assert_eq!(
        parse_steps_spec("[t1..t2)").unwrap(),
        (
            Bound::Included("t1".to_string()),
            Bound::Excluded("t2".to_string())
        )
    );
    assert_eq!(
        parse_steps_spec("(t1..t2]").unwrap(),
        (
            Bound::Excluded("t1".to_string()),
            Bound::Included("t2".to_string())
        )
    );
    assert_eq!(
        parse_steps_spec("(t1..t2)").unwrap(),
        (
            Bound::Excluded("t1".to_string()),
            Bound::Excluded("t2".to_string())
        )
    );

    assert!(parse_steps_spec("(t1..)").is_err());
    assert!(parse_steps_spec("[t1..)").is_err());
    assert!(parse_steps_spec("(..t1)").is_err());
    assert!(parse_steps_spec("(..t1]").is_err());
    assert!(parse_steps_spec("(..)").is_err());
    assert!(parse_steps_spec("[t1..t2[").is_err());
}
