use common::errors::{ChainErr, Result};
use common::file_utils::PathBufWithAdded;
use common::log;

use common::utils::MapIfOk;
use config::Config;
use cpp_checker::cpp_checker_step;
use cpp_ffi_generator::cpp_ffi_generator_step;
use cpp_parser::cpp_parser_step;

use common::string_utils::JoinWithSeparator;
use cpp_explicit_destructors::add_explicit_destructors_step;
use cpp_template_instantiator::find_template_instantiations_step;
use cpp_template_instantiator::instantiate_templates_step;
use database::{Database, DatabaseItem};
use html_logger::HtmlLogger;
use std::fmt;
use std::iter::once;
use std::path::PathBuf;
use type_allocation_places::choose_allocation_places_step;
use workspace::Workspace;
//use cpp_post_processor::cpp_post_process;

/// Creates output and cache directories if they don't exist.
/// Returns `Err` if any path in `config` is invalid or relative.
fn check_all_paths(config: &Config) -> Result<()> {
    let check_dir = |path: &PathBuf| -> Result<()> {
        if !path.is_absolute() {
            return Err(format!(
                "Only absolute paths allowed. Relative path: {}",
                path.display()
            )
            .into());
        }
        if !path.exists() {
            return Err(format!("Directory doesn't exist: {}", path.display()).into());
        }
        if !path.is_dir() {
            return Err(format!("Path is not a directory: {}", path.display()).into());
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
    pub html_logger: HtmlLogger,
}

impl<'a> ProcessorData<'a> {
    pub fn all_databases(&self) -> Vec<&Database> {
        once(&self.current_database as &_)
            .chain(self.dep_databases.iter())
            .collect()
    }
    pub fn all_items(&self) -> Vec<&DatabaseItem> {
        once(&self.current_database as &_)
            .chain(self.dep_databases.iter())
            .flat_map(|d| d.items.iter())
            .collect()
    }
}

#[derive(Debug)]
pub struct ProcessorMainCycleItem {
    pub item_name: String,
    pub run_after: Vec<String>,
}

pub struct ProcessingStep {
    pub name: String,
    pub is_const: bool,
    pub main_cycle_items: Vec<ProcessorMainCycleItem>,
    pub function: Box<Fn(ProcessorData) -> Result<()>>,
}

impl fmt::Debug for ProcessingStep {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ProcessingStep")
            .field("name", &self.name)
            .field("main_cycle_items", &self.main_cycle_items)
            .finish()
    }
}

impl ProcessingStep {
    pub fn new_custom<S: Into<String>, F: 'static + Fn(ProcessorData) -> Result<()>>(
        name: S,
        function: F,
    ) -> ProcessingStep {
        ProcessingStep {
            name: name.into(),
            is_const: false,
            function: Box::new(function),
            main_cycle_items: Vec::new(),
        }
    }
    pub fn new<S: Into<String>, F: 'static + Fn(ProcessorData) -> Result<()>>(
        name: S,
        run_after: Vec<String>,
        function: F,
    ) -> ProcessingStep {
        let name = name.into();
        ProcessingStep {
            name: name.clone(),
            is_const: false,
            function: Box::new(function),
            main_cycle_items: vec![ProcessorMainCycleItem {
                item_name: name,
                run_after,
            }],
        }
    }
}

mod steps {
    use common::string_utils::JoinWithSeparator;
    use database::CppCheckerInfo;
    use html_logger::escape_html;
    use processor::ProcessingStep;

    pub fn print_database() -> ProcessingStep {
        ProcessingStep {
            is_const: true,
            ..ProcessingStep::new_custom("print_database", |mut data| {
                data.html_logger.add_header(&["Item", "Environments"])?;

                for item in &data.current_database.items {
                    data.html_logger.add(
                        &[
                            escape_html(&item.cpp_data.to_string()),
                            format!("{:?}", item.source),
                        ],
                        "database_item",
                    )?;
                    if let Some(ref ffi_items) = item.ffi_items {
                        for ffi_item in ffi_items {
                            let item_text = format!("{:?}", ffi_item.cpp_item);
                            let item_texts = ffi_item.checks.items.iter().map(|item| {
                                format!(
                                    "<li>{}: {}</li>",
                                    item.env.short_text(),
                                    CppCheckerInfo::error_to_log(&item.error)
                                )
                            });
                            let env_text = format!("<ul>{}</ul>", item_texts.join(""));
                            data.html_logger
                                .add(&[escape_html(&item_text), env_text], "ffi_item")?;
                        }
                    }
                }
                Ok(())
            })
        }
    }
    pub fn clear() -> ProcessingStep {
        ProcessingStep::new_custom("clear", |data| {
            data.current_database.clear();
            Ok(())
        })
    }
    pub fn clear_ffi() -> ProcessingStep {
        ProcessingStep::new_custom("clear_ffi", |data| {
            for item in &mut data.current_database.items {
                item.ffi_items = None;
            }
            data.current_database.next_ffi_id = 0;
            Ok(())
        })
    }
}

pub fn process(workspace: &mut Workspace, config: &Config, operations: &[String]) -> Result<()> {
    log::status(format!(
        "Processing crate: {}",
        config.crate_properties().name()
    ));
    check_all_paths(&config)?;

    let standard_processing_steps = vec![
        cpp_parser_step(),
        add_explicit_destructors_step(),
        choose_allocation_places_step(),
        find_template_instantiations_step(),
        instantiate_templates_step(),
        cpp_ffi_generator_step(),
        // TODO: generate_slot_wrappers
        cpp_checker_step(),
        steps::print_database(),
        steps::clear_ffi(),
        steps::clear(),
    ];
    let all_processing_steps: Vec<_> = standard_processing_steps
        .iter()
        .chain(config.custom_processing_steps().iter())
        .collect();

    // TODO: allow to remove any prefix through `Config` (#25)
    #[allow(unused_variables)]
    let remove_qt_prefix = config.crate_properties().name().starts_with("qt_");

    log::status("Loading current crate data");
    let mut current_database = workspace
        .load_or_create_crate(config.crate_properties().name())
        .chain_err(|| "failed to load current crate data")?;

    if !config.dependent_cpp_crates().is_empty() {
        log::status("Loading dependencies");
    }
    let dependent_cpp_crates =
        config
            .dependent_cpp_crates()
            .iter()
            .map_if_ok(|name| -> Result<_> {
                workspace
                    .load_crate(name)
                    .chain_err(|| "failed to load dependency")
            })?;

    let mut current_database_saved = true;

    for operation in operations {
        if let Some(item) = all_processing_steps
            .iter()
            .find(|item| &item.name == operation)
        {
            log::status(format!("Running processor item: {}", &item.name));

            let html_logger = HtmlLogger::new(
                workspace
                    .log_path()?
                    .with_added(format!("{}_log.html", operation)),
                &format!("{} log", operation),
            )?;

            let data = ProcessorData {
                workspace,
                html_logger,
                current_database: &mut current_database,
                dep_databases: &dependent_cpp_crates,
                config,
            };
            (item.function)(data)?;

            if !item.is_const {
                current_database_saved = false;
            }
        } else {
            println!(
                "Unknown operation: {}. Supported operations: {}",
                operation,
                all_processing_steps
                    .iter()
                    .map(|item| &item.name)
                    .join(", ")
            );
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
        filter(&mut parser_cpp_data).chain_err(
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
        log::status("Saving data");
    }

    workspace.put_crate(current_database, current_database_saved);
    workspace.save_data()?;
    Ok(())
}
