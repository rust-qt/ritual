use common::errors::{ChainErr, Result};
use common::file_utils::PathBufWithAdded;
use common::log;

use common::utils::MapIfOk;
use config::Config;
use cpp_ffi_generator::cpp_ffi_generator;
use cpp_parser::cpp_parser;
use new_impl::cpp_checker::cpp_checker;

use common::string_utils::JoinWithSeparator;
use new_impl::database::{Database, DatabaseItem};
use new_impl::html_logger::HtmlLogger;
use new_impl::workspace::Workspace;
use std::iter::once;
use std::path::PathBuf;
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

pub struct ProcessorMainCycleItem {
  pub item_name: String,
  pub run_after: Vec<String>,
}

pub type ProcessorItemFn = fn(data: ProcessorData) -> Result<()>;

pub struct ProcessorItem {
  pub name: String,
  pub main_cycle_items: Vec<ProcessorMainCycleItem>,
  pub function: ProcessorItemFn,
}

impl ProcessorItem {
  pub fn new_custom<S: Into<String>>(name: S, function: ProcessorItemFn) -> ProcessorItem {
    ProcessorItem {
      name: name.into(),
      function,
      main_cycle_items: Vec::new(),
    }
  }
  pub fn new<S: Into<String>>(
    name: S,
    run_after: Vec<String>,
    function: ProcessorItemFn,
  ) -> ProcessorItem {
    let name = name.into();
    ProcessorItem {
      name: name.clone(),
      function,
      main_cycle_items: vec![ProcessorMainCycleItem {
        item_name: name,
        run_after,
      }],
    }
  }
}

mod items {
  use common::string_utils::JoinWithSeparator;
  use new_impl::database::CppCheckerInfo;
  use new_impl::html_logger::escape_html;
  use new_impl::processor::ProcessorItem;

  pub fn print_database() -> ProcessorItem {
    ProcessorItem::new_custom("print_database", |mut data| {
      data.html_logger.add_header(&["Item", "Environments"])?;

      for item in &data.current_database.items {
        data.html_logger.add(
          &[
            escape_html(&item.cpp_data.to_string()),
            format!("{:?}", item.source),
          ],
          "database_item",
        )?;
        if let Some(ref cpp_ffi_methods) = item.cpp_ffi_methods {
          for ffi_method in cpp_ffi_methods {
            let item_text = ffi_method.short_text();
            let item_texts = ffi_method.checks.items.iter().map(|item| {
              format!(
                "<li>{}: {}</li>",
                item.env.short_text(),
                CppCheckerInfo::error_to_log(&item.error)
              )
            });
            let env_text = format!("<ul>{}</ul>", item_texts.join(""));
            data
              .html_logger
              .add(&[escape_html(&item_text), env_text], "ffi_method")?;
          }
        }
      }
      Ok(())
    })
  }
  pub fn clear() -> ProcessorItem {
    ProcessorItem::new_custom("clear", |data| {
      data.current_database.clear();
      Ok(())
    })
  }
  pub fn clear_cpp_ffi() -> ProcessorItem {
    ProcessorItem::new_custom("clear_cpp_ffi", |data| {
      for item in &mut data.current_database.items {
        item.cpp_ffi_methods = None;
      }
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

  let processor_items = vec![
    cpp_parser(),
    // TODO: instantiate_templates
    // TODO: generate_field_accessors
    // TODO: generate_casts
    cpp_ffi_generator(),
    // TODO: generate_slot_wrappers
    cpp_checker(),
    items::print_database(),
    items::clear_cpp_ffi(),
    items::clear(),
  ];

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
    } else {
      println!(
        "Unknown operation: {}. Supported operations: {}",
        operation,
        processor_items.iter().map(|item| &item.name).join(", ")
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
  let current_database_saved = !operations.iter().any(|op| op != "print_database");
  if !current_database_saved {
    log::status("Saving data");
  }
  workspace.put_crate(current_database, current_database_saved);
  workspace.save_data()?;
  Ok(())
}
