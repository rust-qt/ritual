use new_impl::workspace::Workspace;
use config::Config;
use common::log;
use common::utils::MapIfOk;
use common::file_utils::PathBufWithAdded;
use cpp_parser::cpp_parser;
use common::errors::{ChainErr, Result};
use std::path::PathBuf;
use new_impl::cpp_checker::cpp_checker;
use new_impl::database::{Database, DatabaseItem};
use new_impl::html_logger::HtmlLogger;
use new_impl::database::DataEnv;
use new_impl::database::DataSource;
use common::target::current_target;
use new_impl::database::CppItemData;
use new_impl::database::DataEnvInfo;
use new_impl::database::DatabaseUpdateResultType;
use new_impl::database::DatabaseUpdateResult;
use std::iter::once;
use cpp_ffi_generator::cpp_ffi_generator;
//use cpp_post_processor::cpp_post_process;

/// Creates output and cache directories if they don't exist.
/// Returns `Err` if any path in `config` is invalid or relative.
fn check_all_paths(config: &Config) -> Result<()> {
  let check_dir = |path: &PathBuf| -> Result<()> {
    if !path.is_absolute() {
      return Err(
        format!(
          "Only absolute paths allowed. Relative path: {}",
          path.display()
        ).into(),
      );
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
  pub env: DataEnv,
}

impl<'a> ProcessorData<'a> {
  pub fn add_cpp_data(&mut self, data: &CppItemData, info: DataEnvInfo) {
    let data_text = data.to_string();
    let result = self.current_database.add_cpp_data(&self.env, data, info);
    self.html_logger.log_database_update_result(&result);
  }

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

pub struct ProcessorItem {
  pub name: String,
  pub is_main: bool,
  pub run_after: Vec<String>,
  pub function: fn(data: ProcessorData) -> Result<()>,
}

pub fn process(workspace: &mut Workspace, config: &Config, operations: &[String]) -> Result<()> {
  log::status(format!(
    "Processing crate: {}",
    config.crate_properties().name()
  ));
  check_all_paths(&config)?;

  let processor_items = vec![
    cpp_parser(),
    // TODO: instantiate_templates
    // TODO: generate_field_accessors
    // TODO: generate_casts
    cpp_ffi_generator(),
    // TODO: generate_slot_wrappers
    cpp_checker(),
  ];

  // TODO: allow to remove any prefix through `Config` (#25)
  let remove_qt_prefix = config.crate_properties().name().starts_with("qt_");

  log::status("Loading current crate data");
  let mut current_database = workspace
    .load_or_create_crate(config.crate_properties().name())
    .chain_err(|| "failed to load current crate data")?;

  if !config.dependent_cpp_crates().is_empty() {
    log::status("Loading dependencies");
  }
  let dependent_cpp_crates = config
    .dependent_cpp_crates()
    .iter()
    .map_if_ok(|name| -> Result<_> {
      workspace
        .load_crate(name)
        .chain_err(|| "failed to load dependency")
    })?;

  for operation in operations {
    if let Some(item) = processor_items.iter().find(|item| &item.name == operation) {
      log::status(format!("Running processor item: {}", &item.name));

      let html_logger = HtmlLogger::new(
        workspace
          .log_path()?
          .with_added(format!("{}_log.html", operation)),
        "C++ parser log",
      )?;

      let data = ProcessorData {
        workspace,
        html_logger,
        env: DataEnv {
          target: current_target(),
          data_source: match operation.as_str() {
            "cpp_parser" => DataSource::CppParser,
            "cpp_checker" => DataSource::CppChecker,
            _ => unreachable!(),
          },
          cpp_library_version: config.cpp_lib_version().map(|s| s.to_string()),
        },
        current_database: &mut current_database,
        dep_databases: &dependent_cpp_crates,
        config,
      };
      if !data
        .current_database
        .environments
        .iter()
        .any(|e| e == &data.env)
      {
        data.current_database.environments.push(data.env.clone());
      }
      data.current_database.invalidate_env(&data.env);
      (item.function)(data)?;
    } else {
      // TODO: all other operations are also processor items
      match operation.as_str() {
        "print_database" => {
          let path = workspace
            .log_path()?
            .with_added(format!("database_{}.html", current_database.crate_name));
          log::status("Printing database");
          current_database.print_as_html(&path)?;
        }
        "generate_crate" => {
          unimplemented!()

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
        }
        "clear" => unimplemented!(),
        _ => return Err(format!("unknown operation: {}", operation).into()),
      }
    }
  }

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
  let current_database_saved = !operations.iter().any(|op| op != "print_database");
  if !current_database_saved {
    log::status("Saving data");
  }
  workspace.put_crate(current_database, current_database_saved);
  workspace.save_data()?;
  Ok(())
}
