use config::Config;
use cpp_code_generator::CppCodeGenerator;
use cpp_data::CppData;
use cpp_ffi_data::CppAndFfiData;
use cpp_ffi_generator;
use cpp_parser;
use common::errors::{Result, ChainErr};
use common::file_utils::{PathBufWithAdded, move_files, create_dir_all, load_json, save_json,
                         canonicalize, remove_dir_all, remove_dir, read_dir, create_file};
use common::log;
use rust_code_generator::RustCodeGeneratorDependency;
use rust_code_generator;
use rust_generator;
use rust_info::RustExportInfo;

use std::path::{Path, PathBuf};

const COMPLETED_MARKER_FILE_NAME: &'static str = "cpp_to_rust_completed";

/// Returns true if a library was already processed in this `cache_dir`.
/// cpp_to_rust won't process this project again until the completed marker file
/// is removed. You can use this function to skip heavy preparation steps
/// and avoid constructing a Config object.
pub fn is_completed<P: AsRef<Path>>(cache_dir: P) -> bool {
  let path = cache_dir.as_ref().with_added(COMPLETED_MARKER_FILE_NAME);
  let result = path.exists();
  if result {
    log::info("No processing! cpp_to_rust uses previous results.");
    log::info(format!("Remove \"{}\" file to force processing.", path.display()));
  }
  result
}

struct DependencyInfo {
  pub rust_export_info: RustExportInfo,
  pub path: PathBuf,
}

fn load_dependency(path: &PathBuf) -> Result<(RustExportInfo, CppData)> {
  let cpp_data_path = path.with_added("cpp_data.json");
  if !cpp_data_path.exists() {
    return Err(format!("file not found: {}", cpp_data_path.display()).into());
  }
  let cpp_data = try!(load_json(&cpp_data_path));

  let rust_export_info_path = path.with_added("rust_export_info.json");
  if !rust_export_info_path.exists() {
    return Err(format!("file not found: {}", rust_export_info_path.display()).into());
  }
  let rust_export_info = try!(load_json(&rust_export_info_path));
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
  for path in config.dependency_paths() {
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
        log::info(format!("C++ data is loaded from file: {}",
                          cpp_data_cache_file_path.display()));
        cpp_data_processed = true;
        Some(r)
      }
      Err(err) => {
        log::warning(format!("Failed to load C++ data: {}", err));
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
        log::info(format!("Raw C++ data is loaded from file: {}",
                          cpp_data_cache_file_path.display()));
        Some(r)
      }
      Err(err) => {
        log::warning(format!("Failed to load raw C++ data: {}", err));
        err.discard_expected();
        None
      }
    }
  }
  let mut cpp_data = if let Some(r) = loaded_cpp_data {
    r
  } else {
    log::info("Running C++ parser.");
    let parser_config = cpp_parser::CppParserConfig {
      include_paths: Vec::from(config.include_paths()),
      framework_paths: Vec::from(config.framework_paths()),
      include_directives: Vec::from(config.include_directives()),
      target_include_paths: Vec::from(config.target_include_paths()),
      tmp_cpp_path: config.cache_dir_path().with_added("1.cpp"),
      name_blacklist: Vec::from(config.cpp_parser_blocked_names()),
      flags: Vec::from(config.cpp_parser_flags()),
    };
    let cpp_data = try!(cpp_parser::run(parser_config, dependencies_cpp_data)
      .chain_err(|| "C++ parser failed"));
    try!(save_json(&raw_cpp_data_cache_file_path, &cpp_data));
    log::info(format!("Raw C++ data is saved to file: {}",
                      raw_cpp_data_cache_file_path.display()));
    cpp_data
  };
  if !cpp_data_processed {
    log::info("Post-processing parse result.");
    for filter in config.cpp_data_filters() {
      try!(filter(&mut cpp_data).chain_err(|| "cpp_data_filter failed"));
    }
    try!(cpp_data.post_process());

    try!(save_json(&cpp_data_cache_file_path, &cpp_data));
    log::info(format!("C++ data is saved to file: {}",
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
  // TODO: allow to remove any prefix through `Config`
  let remove_qt_prefix = config.crate_properties().name.starts_with("qt_");

  if !config.dependency_paths().is_empty() {
    log::info("Loading dependencies");
  }
  let mut dependencies = Vec::new();
  let mut dependencies_cpp_data = Vec::new();
  for path in config.dependency_paths() {
    let (info, cpp_data) = try!(load_dependency(&try!(canonicalize(path)))
      .chain_err(|| "failed to load dependency"));
    dependencies.push(DependencyInfo {
      path: path.clone(),
      rust_export_info: info,
    });
    dependencies_cpp_data.push(cpp_data);
  }
  let cpp_data = load_or_create_cpp_data(&config, dependencies_cpp_data)?;
  let output_path_existed = config.output_dir_path().with_added("src").exists();

  let c_lib_path = config.output_dir_path().with_added("c_lib");
  let c_lib_path_existed = c_lib_path.exists();


  let c_lib_name = format!("{}_c", &config.crate_properties().name);
  let c_lib_tmp_path = if c_lib_path_existed {
    let path = config.cache_dir_path().with_added("c_lib.new");
    if path.exists() {
      try!(remove_dir_all(&path));
    }
    path
  } else {
    c_lib_path.clone()
  };
  try!(create_dir_all(&c_lib_tmp_path));
  log::info(format!("Generating C wrapper library ({}).", c_lib_name));

  let cpp_ffi_headers = try!(cpp_ffi_generator::run(&cpp_data,
                                                    c_lib_name.clone(),
                                                    config.cpp_ffi_generator_filters())
    .chain_err(|| "FFI generator failed"));

  log::info(format!("Generating C wrapper code."));
  let code_gen = CppCodeGenerator::new(c_lib_name.clone(), c_lib_tmp_path.clone());
  try!(code_gen.generate_template_files(config.include_directives()));
  try!(code_gen.generate_files(&cpp_ffi_headers));
  if c_lib_path_existed {
    try!(move_files(&c_lib_tmp_path, &c_lib_path));
  }

  let crate_new_path = if output_path_existed {
    let path = config.cache_dir_path()
      .with_added(format!("{}.new", &config.crate_properties().name));
    if path.as_path().exists() {
      try!(remove_dir_all(&path));
    }
    path
  } else {
    config.output_dir_path().clone()
  };
  try!(create_dir_all(&crate_new_path));
  let rust_config = rust_code_generator::RustCodeGeneratorConfig {
    crate_properties: config.crate_properties().clone(),
    output_path: crate_new_path.clone(),
    crate_template_path: config.crate_template_path().cloned(),
    c_lib_name: c_lib_name,
    dependencies: dependencies.iter()
      .map(|x| {
        RustCodeGeneratorDependency {
          crate_name: x.rust_export_info.crate_name.clone(),
          crate_path: x.path.clone(),
        }
      })
      .collect(),
  };
  let mut dependency_rust_types = Vec::new();
  for dep in &dependencies {
    dependency_rust_types.extend_from_slice(&dep.rust_export_info.rust_types);
  }
  log::info("Preparing Rust functions");
  let rust_data = try!(rust_generator::run(CppAndFfiData {
                                             cpp_data: cpp_data,
                                             cpp_ffi_headers: cpp_ffi_headers,
                                           },
                                           dependency_rust_types,
                                           rust_generator::RustGeneratorConfig {
                                             crate_name: config.crate_properties().name.clone(),
                                             // TODO: more universal prefix removal
                                             remove_qt_prefix: remove_qt_prefix,
                                           })
    .chain_err(|| "Rust data generator failed"));
  log::info(format!("Generating Rust crate code ({}).",
                    &config.crate_properties().name));
  try!(rust_code_generator::run(rust_config, &rust_data)
    .chain_err(|| "Rust code generator failed"));
  {
    let rust_export_path = config.cache_dir_path().with_added("rust_export_info.json");
    try!(save_json(&rust_export_path,
                   &RustExportInfo {
                     crate_name: config.crate_properties().name.clone(),
                     rust_types: rust_data.processed_types,
                   }));
    log::info(format!("Rust export info is saved to file: {}",
                      rust_export_path.display()));
  }

  if output_path_existed {
    for item in try!(read_dir(&crate_new_path)) {
      let item = try!(item);
      try!(move_files(&crate_new_path.with_added(item.file_name()),
                      &config.output_dir_path().with_added(item.file_name())));
    }
    try!(remove_dir(&crate_new_path));
  }
  try!(save_json(config.output_dir_path().with_added("cpp_build_config.json"),
                 config.cpp_build_config()));
  try!(create_file(config.cache_dir_path().with_added(COMPLETED_MARKER_FILE_NAME)));
  Ok(())
}
