use new_impl::workspace::Workspace;
use config::Config;
use common::log;
use common::utils::MapIfOk;
use common::file_utils::PathBufWithAdded;
use cpp_parser;
use common::errors::{ChainErr, Result};
use std::path::PathBuf;
use new_impl::cpp_checker;
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

pub fn process(workspace: &mut Workspace, config: &Config, operations: &[String]) -> Result<()> {
  log::status(format!(
    "Processing crate: {}",
    config.crate_properties().name()
  ));
  check_all_paths(&config)?;

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

  let mut current_database_saved = true;

  for operation in operations {
    match operation.as_str() {
      "run_cpp_parser" => {
        log::status("Running C++ parser");
        current_database_saved = false;
        let parser_config = cpp_parser::CppParserConfig {
          include_paths: Vec::from(config.cpp_build_paths().include_paths()),
          framework_paths: Vec::from(config.cpp_build_paths().framework_paths()),
          include_directives: Vec::from(config.include_directives()),
          target_include_paths: Vec::from(config.target_include_paths()),
          tmp_cpp_path: workspace.tmp_path()?.with_added("1.cpp"),
          html_log_path: workspace.log_path()?.with_added("cpp_parser_log.html"),
          name_blacklist: Vec::from(config.cpp_parser_blocked_names()),
          clang_arguments: Vec::from(config.cpp_parser_arguments()),
          cpp_library_version: config.cpp_lib_version().map(|s| s.to_string()),
        };

        cpp_parser::run(parser_config, &mut current_database, &dependent_cpp_crates)
          .chain_err(|| "C++ parser failed")?;
      }
      "run_cpp_checker" => {
        log::status("Running C++ checker");
        // TODO: enable this!
        //current_database_saved = false;
        cpp_checker::run(workspace, config)?;
      }
      //...
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
