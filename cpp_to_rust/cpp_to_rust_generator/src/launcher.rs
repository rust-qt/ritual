use config::Config;
use cpp_code_generator::{CppCodeGenerator, generate_cpp_type_size_requester, CppTypeSizeRequest};
use cpp_type::CppTypeClassBase;
use cpp_data::CppData;
use cpp_ffi_data::CppAndFfiData;
use cpp_ffi_generator;
use cpp_parser;
use common::errors::{Result, ChainErr};
use common::string_utils::CaseOperations;
use common::file_utils::{PathBufWithAdded, move_files, create_dir_all, load_json, save_json,
                         canonicalize, remove_dir_all, remove_dir, read_dir, create_file,
                         path_to_str};
use common::build_script_data::BuildScriptData;
use common::log;
use rust_code_generator;
use rust_generator;
use rust_info::{RustTypeWrapperKind, RustExportInfo, DependencyInfo};

use std::path::{Path, PathBuf};

pub fn completed_marker_path<P: AsRef<Path>>(cache_dir: P) -> PathBuf {
  cache_dir.as_ref().with_added("cpp_to_rust_completed")
}

/// Returns true if a library was already processed in this `cache_dir`.
/// cpp_to_rust won't process this project again until the completed marker file
/// is removed. You can use this function to skip heavy preparation steps
/// and avoid constructing a Config object.
pub fn is_completed<P: AsRef<Path>>(cache_dir: P) -> bool {
  completed_marker_path(cache_dir).exists()
}

fn load_dependency(path: &PathBuf) -> Result<(RustExportInfo, CppData)> {
  let cpp_data_path = path.with_added("cpp_data.json");
  if !cpp_data_path.exists() {
    return Err(format!("file not found: {}", cpp_data_path.display()).into());
  }
  let cpp_data = load_json(&cpp_data_path)?;

  let rust_export_info_path = path.with_added("rust_export_info.json");
  if !rust_export_info_path.exists() {
    return Err(format!("file not found: {}", rust_export_info_path.display()).into());
  }
  let rust_export_info = load_json(&rust_export_info_path)?;
  Ok((rust_export_info, cpp_data))
}

fn check_all_paths(config: &Config) -> Result<()> {
  let check_dir = |path: &PathBuf| -> Result<()> {
    if !path.is_absolute() {
      return Err(format!("Only absolute paths allowed. Relative path: {}",
                         path.display())
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
  create_dir_all(config.output_dir_path())?;
  check_dir(config.output_dir_path())?;
  create_dir_all(config.cache_dir_path())?;
  check_dir(config.cache_dir_path())?;
  if let Some(path) = config.crate_template_path() {
    check_dir(path)?;
  }
  for path in config.dependency_cache_paths() {
    check_dir(path)?;
  }
  for path in config.include_paths() {
    check_dir(path)?;
  }
  for path in config.target_include_paths() {
    check_dir(path)?;
  }
  Ok(())
}

fn load_or_create_cpp_data(config: &Config,
                           dependencies_cpp_data: Vec<CppData>)
                           -> Result<CppData> {
  let cpp_data_cache_file_path = config.cache_dir_path().with_added("cpp_data.json");
  let mut cpp_data_processed = false;
  let mut loaded_cpp_data = if cpp_data_cache_file_path.as_path().is_file() {
    match load_json(&cpp_data_cache_file_path) {
      Ok(r) => {
        log::status(format!("C++ data is loaded from file: {}",
                            cpp_data_cache_file_path.display()));
        cpp_data_processed = true;
        Some(r)
      }
      Err(err) => {
        log::status(format!("Failed to load C++ data: {}", err));
        err.discard_expected();
        None
      }
    }
  } else {
    None
  };
  let raw_cpp_data_cache_file_path = config.cache_dir_path().with_added("raw_cpp_data.json");
  if loaded_cpp_data.is_none() {
    loaded_cpp_data = match load_json(&raw_cpp_data_cache_file_path) {
      Ok(r) => {
        log::status(format!("Raw C++ data is loaded from file: {}",
                            cpp_data_cache_file_path.display()));
        Some(r)
      }
      Err(err) => {
        log::status(format!("Failed to load raw C++ data: {}", err));
        err.discard_expected();
        None
      }
    }
  }
  let mut cpp_data = if let Some(r) = loaded_cpp_data {
    r
  } else {
    log::status("Running C++ parser");
    let parser_config = cpp_parser::CppParserConfig {
      include_paths: Vec::from(config.include_paths()),
      framework_paths: Vec::from(config.framework_paths()),
      include_directives: Vec::from(config.include_directives()),
      target_include_paths: Vec::from(config.target_include_paths()),
      tmp_cpp_path: config.cache_dir_path().with_added("1.cpp"),
      name_blacklist: Vec::from(config.cpp_parser_blocked_names()),
      flags: Vec::from(config.cpp_parser_flags()),
    };
    let cpp_data =
      cpp_parser::run(parser_config, dependencies_cpp_data).chain_err(|| "C++ parser failed")?;
    log::status("Saving raw C++ data");
    save_json(&raw_cpp_data_cache_file_path, &cpp_data)?;
    log::status(format!("Raw C++ data is saved to file: {}",
                        raw_cpp_data_cache_file_path.display()));
    cpp_data
  };
  if !cpp_data_processed {
    log::status("Post-processing parse result");
    for filter in config.cpp_data_filters() {
      filter(&mut cpp_data).chain_err(|| "cpp_data_filter failed")?;
    }
    cpp_data.choose_allocation_places(config.type_allocation_places())?;

    cpp_data.post_process()?;

    log::status("Saving C++ data");
    save_json(&cpp_data_cache_file_path, &cpp_data)?;
    log::status(format!("C++ data is saved to file: {}",
                        cpp_data_cache_file_path.display()));
  };
  Ok(cpp_data)
}


// TODO: simplify this function
#[cfg_attr(feature="clippy", allow(cyclomatic_complexity))]
pub fn run(config: Config) -> Result<()> {
  if is_completed(config.cache_dir_path()) {
    return Ok(());
  }
  check_all_paths(&config)?;
  {
    let mut logger = log::default_logger();
    logger.default_settings.write_to_stderr = false;
    logger.category_settings.clear();
    for category in &[log::Status, log::Error] {
      logger.category_settings.insert(*category,
                                      log::LoggerSettings {
                                        file_path: None,
                                        write_to_stderr: true,
                                      });
    }
    const NO_LOG_VAR_NAME: &'static str = "CPP_TO_RUST_NO_LOG";
    if ::std::env::var(NO_LOG_VAR_NAME).is_ok() {
      logger.log(log::Status,
                 format!("Debug log is disabled with {} env var.", NO_LOG_VAR_NAME));
    } else {
      let logs_dir = config.cache_dir_path().with_added("log");
      logger.log(log::Status,
                 format!("Debug log will be saved to {}", logs_dir.display()));
      logger.log(log::Status,
                 format!("Set {} env var to disable debug log.", NO_LOG_VAR_NAME));
      if logs_dir.exists() {
        remove_dir_all(&logs_dir)?;
      }
      create_dir_all(&logs_dir)?;
      for category in &[log::DebugGeneral,
                        log::DebugMoveFiles,
                        log::DebugTemplateInstantiation,
                        log::DebugInheritance,
                        log::DebugParserSkips,
                        log::DebugParser,
                        log::DebugFfiSkips,
                        log::DebugSignals,
                        log::DebugAllocationPlace,
                        log::DebugRustSkips,
                        log::DebugQtDoc,
                        log::DebugQtHeaderNames] {
        let name = format!("{:?}", *category).to_snake_case();
        let path = logs_dir.with_added(format!("{}.log", name));
        logger.category_settings.insert(*category,
                                        log::LoggerSettings {
                                          file_path: Some(path),
                                          write_to_stderr: false,
                                        });
      }
    }
  }

  // TODO: allow to remove any prefix through `Config` (#25)
  let remove_qt_prefix = config.crate_properties().name().starts_with("qt_");

  if !config.dependency_cache_paths().is_empty() {
    log::status("Loading dependencies");
  }
  let mut dependencies = Vec::new();
  let mut dependencies_cpp_data = Vec::new();
  for cache_path in config.dependency_cache_paths() {
    let (info, cpp_data) =
      load_dependency(&canonicalize(cache_path)?).chain_err(|| "failed to load dependency")?;
    dependencies.push(DependencyInfo {
      cache_path: cache_path.clone(),
      rust_export_info: info,
    });
    dependencies_cpp_data.push(cpp_data);
  }
  let cpp_data = load_or_create_cpp_data(&config, dependencies_cpp_data)?;
  let output_path_existed = config.output_dir_path().with_added("src").exists();

  let c_lib_path = config.output_dir_path().with_added("c_lib");
  let c_lib_path_existed = c_lib_path.exists();


  let c_lib_name = format!("{}_c", &config.crate_properties().name());
  let c_lib_tmp_path = if c_lib_path_existed {
    let path = config.cache_dir_path().with_added("c_lib.new");
    if path.exists() {
      remove_dir_all(&path)?;
    }
    path
  } else {
    c_lib_path.clone()
  };
  create_dir_all(&c_lib_tmp_path)?;
  log::status(format!("Generating C++ wrapper library ({})", c_lib_name));

  let cpp_ffi_headers = cpp_ffi_generator::run(&cpp_data,
                                               c_lib_name.clone(),
                                               config.cpp_ffi_generator_filters())
      .chain_err(|| "FFI generator failed")?;

  log::status(format!("Generating C++ wrapper code"));
  let code_gen = CppCodeGenerator::new(c_lib_name.clone(), c_lib_tmp_path.clone());
  code_gen.generate_template_files(config.include_directives())?;
  code_gen.generate_files(&cpp_ffi_headers)?;

  let crate_new_path = if output_path_existed {
    let path = config.cache_dir_path()
      .with_added(format!("{}.new", &config.crate_properties().name()));
    if path.as_path().exists() {
      remove_dir_all(&path)?;
    }
    path
  } else {
    config.output_dir_path().clone()
  };
  create_dir_all(&crate_new_path)?;
  let rust_config = rust_code_generator::RustCodeGeneratorConfig {
    crate_properties: config.crate_properties().clone(),
    output_path: crate_new_path.clone(),
    crate_template_path: config.crate_template_path().cloned(),
    c_lib_name: c_lib_name.clone(),
    generator_dependencies: &dependencies,
    write_dependencies_local_paths: config.write_dependencies_local_paths(),
  };
  let mut dependency_rust_types = Vec::new();
  for dep in &dependencies {
    dependency_rust_types.extend_from_slice(&dep.rust_export_info.rust_types);
  }
  log::status("Preparing Rust functions");
  let rust_data = rust_generator::run(CppAndFfiData {
                                        cpp_data: cpp_data,
                                        cpp_ffi_headers: cpp_ffi_headers,
                                      },
                                      dependency_rust_types,
                                      rust_generator::RustGeneratorConfig {
                                        crate_name: config.crate_properties().name().clone(),
                                        // TODO: more universal prefix removal (#25)
                                        remove_qt_prefix: remove_qt_prefix,
                                      }).chain_err(|| "Rust data generator failed")?;
  log::status(format!("Generating Rust crate code ({})",
                      &config.crate_properties().name()));
  rust_code_generator::run(rust_config, &rust_data).chain_err(|| "Rust code generator failed")?;
  let mut cpp_type_size_requests = Vec::new();
  for type1 in &rust_data.processed_types {
    if let RustTypeWrapperKind::Struct { ref size_const_name, .. } = type1.kind {
      if let Some(ref size_const_name) = *size_const_name {
        cpp_type_size_requests.push(CppTypeSizeRequest {
          cpp_code: CppTypeClassBase {
              name: type1.cpp_name.clone(),
              template_arguments: type1.cpp_template_arguments.clone(),
            }.to_cpp_code()?,
          size_const_name: size_const_name.clone(),
        });
      }
    }
  }
  {
    let mut file = create_file(c_lib_tmp_path.with_added("type_sizes.cpp"))?;
    file.write(generate_cpp_type_size_requester(&cpp_type_size_requests,
                                              config.include_directives())?)?;
  }
  if c_lib_path_existed {
    move_files(&c_lib_tmp_path, &c_lib_path)?;
  }
  {
    let rust_export_path = config.cache_dir_path().with_added("rust_export_info.json");
    log::status("Saving Rust export info");
    save_json(&rust_export_path,
              &RustExportInfo {
                crate_name: config.crate_properties().name().clone(),
                crate_version: config.crate_properties().version().clone(),
                rust_types: rust_data.processed_types,
                output_path: path_to_str(config.output_dir_path())?.to_string(),
              })?;
    log::status(format!("Rust export info is saved to file: {}",
                        rust_export_path.display()));
  }

  if output_path_existed {
    // move all generated top level files and folders (and delete corresponding old folders)
    // but keep existing unknown top level files and folders, such as "target" or ".cargo"
    for item in read_dir(&crate_new_path)? {
      let item = item?;
      move_files(&crate_new_path.with_added(item.file_name()),
                 &config.output_dir_path().with_added(item.file_name()))?;
    }
    remove_dir(&crate_new_path)?;
  }
  save_json(config.output_dir_path().with_added("build_script_data.json"),
            &BuildScriptData {
              cpp_build_config: config.cpp_build_config().clone(),
              cpp_wrapper_lib_name: c_lib_name,
            })?;
  create_file(completed_marker_path(config.cache_dir_path()))?;
  log::status("cpp_to_rust generator finished");
  Ok(())
}
