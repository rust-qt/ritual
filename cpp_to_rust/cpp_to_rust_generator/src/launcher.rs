//! Main function of the generator

use config::{Config, DebugLoggingConfig};
use cpp_code_generator::{CppCodeGenerator, generate_cpp_type_size_requester, CppTypeSizeRequest};
use cpp_type::CppTypeClassBase;
use cpp_data::{CppData, CppDataWithDeps, ParserCppData};
use cpp_ffi_data::CppAndFfiData;
use cpp_ffi_generator;
use cpp_parser;
use cpp_post_processor::cpp_post_process;
use common::errors::{Result, ChainErr};
use common::string_utils::CaseOperations;
use common::file_utils::{PathBufWithAdded, move_files, create_dir_all, save_json, load_bincode,
                         save_bincode, canonicalize, remove_dir_all, remove_dir, read_dir,
                         create_file, path_to_str};
use common::BuildScriptData;
use common::log;
use rust_code_generator;
use rust_generator;
use rust_info::{RustTypeWrapperKind, RustExportInfo, DependencyInfo};

use std::path::{Path, PathBuf};
use std::collections::HashMap;


/// Returns path to the completion marker file
/// indicating that processing of the library in this `cache_dir`
/// was completed before.
/// This function can be used to skip heavy preparation steps
/// and avoid constructing a `Config` object.
/// Note that the marker is not created if
/// `write_cache` was set to false in `Config` during a previous run.
/// The marker should only be used if cache usage is set to `CacheUsage::Full`.
pub fn completed_marker_path<P: AsRef<Path>>(cache_dir: P) -> PathBuf {
  cache_dir.as_ref().with_added("cpp_to_rust_completed")
}

/// Returns true if a library was already processed in this `cache_dir`.
/// This function can be used to skip heavy preparation steps
/// and avoid constructing a `Config` object.
/// Note that the marker is not created if
/// `write_cache` was set to false in `Config` during a previous run.
/// The marker should only be used if cache usage is set to `CacheUsage::Full`.
pub fn is_completed<P: AsRef<Path>>(cache_dir: P) -> bool {
  completed_marker_path(cache_dir).exists()
}

/// Loads `RustExportInfo` and `CppData` or a dependency previously
/// processed in the cache directory `path`.
fn load_dependency(path: &PathBuf) -> Result<(RustExportInfo, CppData)> {
  let parser_cpp_data_path = path.with_added("parser_cpp_data.bin");
  if !parser_cpp_data_path.exists() {
    return Err(format!("file not found: {}", parser_cpp_data_path.display()).into());
  }
  let parser_cpp_data = load_bincode(&parser_cpp_data_path)?;



  let processed_cpp_data_path = path.with_added("processed_cpp_data.bin");
  if !processed_cpp_data_path.exists() {
    return Err(format!("file not found: {}", processed_cpp_data_path.display()).into());
  }
  let processed_cpp_data = load_bincode(&processed_cpp_data_path)?;
  let cpp_data = CppData {
    parser: parser_cpp_data,
    processed: processed_cpp_data,
  };
  let rust_export_info_path = path.with_added("rust_export_info.bin");
  if !rust_export_info_path.exists() {
    return Err(format!("file not found: {}", rust_export_info_path.display()).into());
  }
  let rust_export_info = load_bincode(&rust_export_info_path)?;
  Ok((rust_export_info, cpp_data))
}

/// Creates output and cache directories if they don't exist.
/// Returns `Err` if any path in `config` is invalid or relative.
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

/// Loads C++ data saved during a previous run of the generator
/// from the cache directory if it's available and permitted by `config.cache_usage()`.
/// Otherwise, performs necessary steps to parse and process C++ data.
fn load_or_create_cpp_data(config: &Config,
                           dependencies_cpp_data: Vec<CppData>)
                           -> Result<CppDataWithDeps> {
  let parser_cpp_data_file_path = config.cache_dir_path().with_added("parser_cpp_data.bin");

  let loaded_parser_cpp_data = if config.cache_usage().can_use_raw_cpp_data() &&
                                  parser_cpp_data_file_path.as_path().is_file() {
    match load_bincode(&parser_cpp_data_file_path) {
      Ok(r) => {
        log::status(format!("C++ parser data is loaded from file: {}",
                            parser_cpp_data_file_path.display()));
        Some(r)
      }
      Err(err) => {
        log::status(format!("Failed to load C++ parser data: {}", err));
        err.discard_expected();
        None
      }
    }
  } else {
    None
  };
  let parser_cpp_data = if let Some(x) = loaded_parser_cpp_data {
    x
  } else {
    log::status("Running C++ parser");
    let parser_config = cpp_parser::CppParserConfig {
      include_paths: Vec::from(config.include_paths()),
      framework_paths: Vec::from(config.framework_paths()),
      include_directives: Vec::from(config.include_directives()),
      target_include_paths: Vec::from(config.target_include_paths()),
      tmp_cpp_path: config.cache_dir_path().with_added("1.cpp"),
      name_blacklist: Vec::from(config.cpp_parser_blocked_names()),
      clang_arguments: Vec::from(config.cpp_parser_arguments()),
    };
    let mut parser_cpp_data: ParserCppData = cpp_parser::run(parser_config, &dependencies_cpp_data)
      .chain_err(|| "C++ parser failed")?;
    parser_cpp_data
      .detect_signals_and_slots(&dependencies_cpp_data)?;
    // TODO: rename `cpp_data_filters` to `parser_cpp_data_filters`
    if config.has_cpp_data_filters() {
      log::status("Running custom filters for C++ parser data");
      for filter in config.cpp_data_filters() {
        filter(&mut parser_cpp_data)
          .chain_err(|| "cpp_data_filter failed")?;
      }
    }
    if config.write_cache() {
      log::status("Saving C++ parser data");
      save_bincode(&parser_cpp_data_file_path, &parser_cpp_data)?;
      log::status(format!("C++ parser data is saved to file: {}",
                          parser_cpp_data_file_path.display()));
    }
    parser_cpp_data
  };

  let processed_cpp_data_file_path = config
    .cache_dir_path()
    .with_added("processed_cpp_data.bin");

  let loaded_processed_cpp_data = if config.cache_usage().can_use_cpp_data() &&
                                     processed_cpp_data_file_path.as_path().is_file() {
    match load_bincode(&processed_cpp_data_file_path) {
      Ok(r) => {
        log::status(format!("C++ processed data is loaded from file: {}",
                            processed_cpp_data_file_path.display()));
        Some(r)
      }
      Err(err) => {
        log::status(format!("Failed to load C++ processed data: {}", err));
        err.discard_expected();
        None
      }
    }
  } else {
    None
  };
  let full_cpp_data = if let Some(x) = loaded_processed_cpp_data {
    CppDataWithDeps {
      current: CppData {
        parser: parser_cpp_data,
        processed: x,
      },
      dependencies: dependencies_cpp_data,
    }
  } else {
    log::status("Post-processing parse result");
    let r = cpp_post_process(parser_cpp_data, dependencies_cpp_data, config.type_allocation_places())?;
    if config.write_cache() {
      log::status("Saving processed C++ data");
      save_bincode(&processed_cpp_data_file_path, &r.current.processed)?;
      log::status(format!("Processed C++ data is saved to file: {}",
                          processed_cpp_data_file_path.display()));
    }
    r
  };
  Ok(full_cpp_data)
}


/// Executes the generator.
#[cfg_attr(feature="clippy", allow(cyclomatic_complexity))]
pub fn run(config: Config) -> Result<()> {
  if config.cache_usage().can_skip_all() && is_completed(config.cache_dir_path()) {
    return Ok(());
  }
  check_all_paths(&config)?;
  {
    let mut logger = log::default_logger();
    logger.set_default_settings(log::LoggerSettings {
                                  file_path: None,
                                  write_to_stderr: false,
                                });
    let mut category_settings = HashMap::new();
    let mut debug_categories = vec![log::DebugGeneral,
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
                                    log::DebugQtHeaderNames];
    for category in &[log::Status, log::Error] {
      if config.quiet_mode() {
        debug_categories.push(*category);
      } else {
        category_settings.insert(*category,
                                 log::LoggerSettings {
                                   file_path: None,
                                   write_to_stderr: true,
                                 });
      }
    }
    let debug_logging_config = if config.debug_logging_config() == &DebugLoggingConfig::Print &&
                                  config.quiet_mode() {
      DebugLoggingConfig::SaveToFile
    } else {
      config.debug_logging_config().clone()
    };
    if debug_logging_config == DebugLoggingConfig::SaveToFile {
      let logs_dir = config.cache_dir_path().with_added("log");
      logger.log(log::Status,
                 format!("Debug log will be saved to {}", logs_dir.display()));
      if logs_dir.exists() {
        remove_dir_all(&logs_dir)?;
      }
      create_dir_all(&logs_dir)?;
      for category in debug_categories {
        let name = format!("{:?}", category).to_snake_case();
        let path = logs_dir.with_added(format!("{}.log", name));
        category_settings.insert(category,
                                 log::LoggerSettings {
                                   file_path: Some(path),
                                   write_to_stderr: false,
                                 });
      }
    } else if debug_logging_config == DebugLoggingConfig::Print {
      for category in debug_categories {
        category_settings.insert(category,
                                 log::LoggerSettings {
                                   file_path: None,
                                   write_to_stderr: true,
                                 });
      }
    }
    logger.set_all_category_settings(category_settings);
  }

  // TODO: allow to remove any prefix through `Config` (#25)
  let remove_qt_prefix = config.crate_properties().name().starts_with("qt_");

  if !config.dependency_cache_paths().is_empty() {
    log::status("Loading dependencies");
  }
  let mut dependencies = Vec::new();
  let mut dependencies_cpp_data = Vec::new();
  for cache_path in config.dependency_cache_paths() {
    let (info, cpp_data) = load_dependency(&canonicalize(cache_path)?)
      .chain_err(|| "failed to load dependency")?;
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


  let cpp_ffi_lib_name = format!("{}_c", &config.crate_properties().name());
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
  log::status(format!("Generating C++ wrapper library ({})", cpp_ffi_lib_name));

  let cpp_ffi_headers = cpp_ffi_generator::run(&cpp_data,
                                               cpp_ffi_lib_name.clone(),
                                               config.cpp_ffi_generator_filters())
      .chain_err(|| "FFI generator failed")?;

  log::status(format!("Generating C++ wrapper code"));
  let code_gen = CppCodeGenerator::new(cpp_ffi_lib_name.clone(), c_lib_tmp_path.clone());
  code_gen
    .generate_template_files(config.include_directives())?;
  code_gen.generate_files(&cpp_ffi_headers)?;

  let crate_new_path = if output_path_existed {
    let path = config
      .cache_dir_path()
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
    cpp_ffi_lib_name: cpp_ffi_lib_name.clone(),
    generator_dependencies: &dependencies,
    write_dependencies_local_paths: config.write_dependencies_local_paths(),
    cpp_lib_version: config.cpp_lib_version().map(|s| s.into()),
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
                                      })
      .chain_err(|| "Rust data generator failed")?;
  log::status(format!("Generating Rust crate code ({})",
                      &config.crate_properties().name()));
  rust_code_generator::run(rust_config, &rust_data)
    .chain_err(|| "Rust code generator failed")?;
  let mut cpp_type_size_requests = Vec::new();
  for type1 in &rust_data.processed_types {
    if let RustTypeWrapperKind::Struct { ref size_const_name, .. } = type1.kind {
      if let Some(ref size_const_name) = *size_const_name {
        cpp_type_size_requests.push(CppTypeSizeRequest {
                                      cpp_code: CppTypeClassBase {
                                          name: type1.cpp_name.clone(),
                                          template_arguments: type1.cpp_template_arguments.clone(),
                                        }
                                        .to_cpp_code()?,
                                      size_const_name: size_const_name.clone(),
                                    });
      }
    }
  }
  {
    let mut file = create_file(c_lib_tmp_path.with_added("type_sizes.cpp"))?;
    file
      .write(generate_cpp_type_size_requester(&cpp_type_size_requests,
                                              config.include_directives())?)?;
  }
  if c_lib_path_existed {
    move_files(&c_lib_tmp_path, &c_lib_path)?;
  }
  if config.write_cache() {
    let rust_export_path = config
      .cache_dir_path()
      .with_added("rust_export_info.bin");
    log::status("Saving Rust export info");
    save_bincode(&rust_export_path,
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
  save_json(config
              .output_dir_path()
              .with_added("build_script_data.json"),
            &BuildScriptData {
               cpp_build_config: config.cpp_build_config().clone(),
               cpp_wrapper_lib_name: cpp_ffi_lib_name,
               cpp_lib_version: config.cpp_lib_version().map(|s| s.to_string()),
             })?;
  if config.write_cache() {
    create_file(completed_marker_path(config.cache_dir_path()))?;
  }
  log::status("cpp_to_rust generator finished");
  Ok(())
}
