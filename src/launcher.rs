extern crate num_cpus;

use config::Config;
use cpp_code_generator::CppCodeGenerator;
use cpp_ffi_generator;
use cpp_lib_builder::CppLibBuilder;
use cpp_parser;
use dependency_info::DependencyInfo;
use errors::{Result, ChainErr};
use file_utils::{PathBufWithAdded, move_files, create_dir_all, load_json, save_json, canonicalize,
                 remove_dir_all, remove_dir, read_dir, path_to_str};
use log;
use qt_doc_parser::QtDocData;
use qt_specific;
use rust_code_generator::{RustCodeGeneratorDependency, RustLinkItem, RustLinkKind};
use rust_code_generator;
use rust_generator;
use rust_info::{InputCargoTomlData, RustExportInfo};
use serializable::LibSpec;
use utils::{is_msvc, run_command, MapIfOk};

use std::path::PathBuf;
use std::process::Command;
use std::env;

pub enum BuildProfile {
  Debug,
  Release,
}

pub use rust_code_generator::InvokationMethod;

pub struct BuildEnvironment {
  pub config: Config,
  pub output_dir_path: PathBuf,
  pub source_dir_path: PathBuf,
  pub dependency_paths: Vec<PathBuf>,
  pub num_jobs: Option<i32>,
  pub build_profile: BuildProfile,
  pub pipe_output: bool,
}

pub fn run_from_build_script(config: Config) -> Result<()> {
  let mut dependency_paths = Vec::new();
  for (name, value) in env::vars_os() {
    if let Ok(name) = name.into_string() {
      if name.starts_with("DEP_") && name.ends_with("_CPP_TO_RUST_DATA_PATH") {
        log::info(format!("Found dependency: {}", value.to_string_lossy()));
        dependency_paths.push(PathBuf::from(value));
      }
    }
  }
  run(BuildEnvironment {
    config: config,
    source_dir_path: PathBuf::from(try!(env::var("CARGO_MANIFEST_DIR")
      .chain_err(|| "failed to read required env var: CARGO_MANIFEST_DIR"))),
    output_dir_path: PathBuf::from(try!(env::var("OUT_DIR")
      .chain_err(|| "failed to read required env var: OUT_DIR"))),
    num_jobs: try!(env::var("NUM_JOBS").chain_err(|| "failed to read required env var: NUM_JOBS"))
      .parse()
      .ok(),
    build_profile: match try!(env::var("PROFILE")
        .chain_err(|| "failed to read required env var: PROFILE"))
      .as_ref() {
      "debug" | "test" | "doc" => BuildProfile::Debug,
      "release" | "bench" => BuildProfile::Release,
      a => return Err(format!("unsupported profile: {}", a).into()),
    },
    dependency_paths: dependency_paths,
    pipe_output: false,
  })
}



// TODO: simplify this function
#[cfg_attr(feature="clippy", allow(cyclomatic_complexity))]
pub fn run(env: BuildEnvironment) -> Result<()> {
  // canonicalize paths
  if !env.source_dir_path.as_path().exists() {
    return Err(format!("source dir doesn't exist: {}",
                       env.source_dir_path.display())
      .into());
  }
  if !env.output_dir_path.as_path().exists() {
    try!(create_dir_all(&env.output_dir_path));
  }
  let output_dir_path = try!(canonicalize(&env.output_dir_path));
  let source_dir_path = try!(canonicalize(&env.source_dir_path));

  let lib_spec_path = source_dir_path.with_added("spec.json");

  log::info("Reading lib spec");
  if !lib_spec_path.exists() {
    return Err(format!("Lib spec file does not exist: {}", lib_spec_path.display()).into());
  }
  let lib_spec: LibSpec = try!(load_json(&lib_spec_path));

  log::info("Reading input Cargo.toml");
  let input_cargo_toml_path = source_dir_path.with_added("Cargo.toml");
  if !input_cargo_toml_path.exists() {
    return Err(format!("Input Cargo.toml does not exist: {}",
                       input_cargo_toml_path.display())
      .into());
  }
  let input_cargo_toml_data = try!(InputCargoTomlData::from_file(&input_cargo_toml_path));
  if input_cargo_toml_data.cpp_lib_name == input_cargo_toml_data.name {
    return Err(format!("Rust crate name ({}) must not be the same as C++ library name ({}) \
            because it can cause library name conflict.",
                       input_cargo_toml_data.name,
                       input_cargo_toml_data.cpp_lib_name)
      .into());
  }
  log::info(format!("C++ library name: {}", input_cargo_toml_data.cpp_lib_name));

  let is_qt_library = input_cargo_toml_data.cpp_lib_name.starts_with("Qt5");

  let mut qt_this_lib_headers_dir = None;
  for &(caption, paths) in &[("Include path", env.config.include_paths()),
                             ("Lib path", env.config.lib_paths()),
                             ("Target include path", env.config.target_include_paths())] {
    for path in paths {
      if !path.is_absolute() {
        return Err(format!("{} is not absolute: {}", caption, path.display()).into());
      }
      if !path.exists() {
        return Err(format!("{} does not exist: {}", caption, path.display()).into());
      }
      if !path.is_dir() {
        return Err(format!("{} is not a directory: {}", caption, path.display()).into());
      }
    }
  }
  let mut cpp_lib_dirs = Vec::from(env.config.lib_paths());
  let mut include_dirs = Vec::from(env.config.include_paths());
  let mut target_include_dirs = Vec::from(env.config.target_include_paths());
  let mut framework_dirs = Vec::new();
  let mut link_items = Vec::new();
  if is_qt_library {

    let qmake_path = "qmake".to_string();
    log::info("Detecting Qt directories...");
    let result1 = try!(run_command(Command::new(&qmake_path)
                                     .arg("-query")
                                     .arg("QT_INSTALL_HEADERS"),
                                   true,
                                   true));
    let qt_install_headers_path = PathBuf::from(result1.trim());
    log::info(format!("QT_INSTALL_HEADERS = \"{}\"",
                      qt_install_headers_path.display()));
    let result2 = try!(run_command(Command::new(&qmake_path)
                                     .arg("-query")
                                     .arg("QT_INSTALL_LIBS"),
                                   true,
                                   true));
    let qt_install_libs_path = PathBuf::from(result2.trim());
    log::info(format!("QT_INSTALL_LIBS = \"{}\"", qt_install_libs_path.display()));
    cpp_lib_dirs.push(qt_install_libs_path.clone());
    include_dirs.push(qt_install_headers_path.clone());

    if input_cargo_toml_data.cpp_lib_name.starts_with("Qt5") {
      let dir = qt_install_headers_path.with_added(format!("Qt{}", &input_cargo_toml_data.cpp_lib_name[3..]));
      if dir.exists() {
        qt_this_lib_headers_dir = Some(dir.clone());
        include_dirs.push(dir.clone());
        target_include_dirs.push(dir);
      } else {
        let dir2 = qt_install_libs_path.with_added(format!("Qt{}.framework/Headers",
                              &input_cargo_toml_data.cpp_lib_name[3..]));
        if dir2.exists() {
          qt_this_lib_headers_dir = Some(dir2.clone());
          include_dirs.push(dir2.clone());
          target_include_dirs.push(dir2);
          framework_dirs.push(qt_install_libs_path.clone());
          link_items.push(RustLinkItem {
            name: format!("Qt{}", &input_cargo_toml_data.cpp_lib_name[3..]),
            kind: RustLinkKind::Framework,
          });
        } else {
          log::warning(format!("extra header dir not found (tried: {}, {})",
                               dir.display(),
                               dir2.display()));
        }
      }
    }
  }
  if framework_dirs.is_empty() {
    link_items.push(RustLinkItem {
      name: input_cargo_toml_data.cpp_lib_name.clone(),
      kind: RustLinkKind::SharedLibrary,
    });
    for name in env.config.extra_libs() {
      if is_msvc() && name == "GL" {
        // msvc doesn't need to link to GL
        // TODO: allow platform-specific link items in manifest (#14)
        continue;
      }
      link_items.push(RustLinkItem {
        name: name.clone(),
        kind: RustLinkKind::SharedLibrary,
      });
    }
  }
  let qt_doc_data = if is_qt_library {
    // TODO: find a better way to specify doc source (#35)
    let env_var_name = format!("{}_DOC_DATA",
                               input_cargo_toml_data.cpp_lib_name.to_uppercase());
    if let Ok(env_var_value) = env::var(&env_var_name) {
      log::info(format!("Loading Qt doc data from {}", &env_var_value));
      match QtDocData::new(&PathBuf::from(&env_var_value)) {
        Ok(r) => Some(r),
        Err(msg) => {
          log::warning(format!("Failed to load Qt doc data: {}", msg));
          None
        }
      }
    } else {
      log::warning(format!("Building without Qt doc data (no env var: {})",
                           env_var_name));
      None
    }
  } else {
    None
  };
  if !env.dependency_paths.is_empty() {
    log::info("Loading dependencies");
  }
  let dependencies: Vec<_> = try!(env.dependency_paths
    .iter()
    .map_if_ok(|path| DependencyInfo::load(&try!(canonicalize(path))))
    .chain_err(|| "failed to load dependency"));

  let c_lib_parent_path = output_dir_path.with_added("c_lib");
  let c_lib_install_path = c_lib_parent_path.with_added("install");
  let c_lib_lib_path = c_lib_install_path.with_added("lib");
  let num_jobs = env.num_jobs.unwrap_or_else(|| num_cpus::get() as i32);
  let mut dependency_cpp_types = Vec::new();
  for dep in &dependencies {
    dependency_cpp_types.extend_from_slice(&dep.cpp_data.types);
  }
  let c_lib_is_shared = is_msvc();
  if output_dir_path.with_added("skip_processing").as_path().exists() {
    log::info("Processing skipped!");
  } else {
    let parse_result_cache_file_path = output_dir_path.with_added("cpp_data.json");
    let loaded_parse_result = if parse_result_cache_file_path.as_path().is_file() {
      match load_json(&parse_result_cache_file_path) {
        Ok(r) => {
          log::info(format!("C++ data is loaded from file: {}",
                            parse_result_cache_file_path.display()));
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

    let parse_result = if let Some(r) = loaded_parse_result {
      r
    } else {
      log::info("Parsing C++ headers.");
      let mut parse_result =
        try!(cpp_parser::run(cpp_parser::CppParserConfig {
                               include_dirs: include_dirs.clone(),
                               framework_dirs: framework_dirs.clone(),
                               include_directives: Vec::from(env.config.include_directives()),
                               target_include_dirs: target_include_dirs,
                               tmp_cpp_path: output_dir_path.with_added("1.cpp"),
                               name_blacklist: Vec::from(env.config.cpp_parser_blocked_names()),
                             },
                             &dependencies.iter().map(|x| &x.cpp_data).collect::<Vec<_>>())
          .chain_err(|| "C++ parser failed"));
      if is_qt_library {
        if let Some(ref qt_this_lib_headers_dir) = qt_this_lib_headers_dir {
          try!(qt_specific::fix_header_names(&mut parse_result, qt_this_lib_headers_dir)
            .chain_err(|| "qt fix_header_names failed"));
        }
      }
      log::info("Post-processing parse result.");
      try!(parse_result.post_process(&dependencies.iter().map(|x| &x.cpp_data).collect::<Vec<_>>()));

      try!(save_json(&parse_result_cache_file_path, &parse_result));
      log::info(format!("Header parse result is saved to file: {}",
                        parse_result_cache_file_path.display()));
      parse_result
    };

    let c_lib_name = format!("{}_c", &input_cargo_toml_data.name);
    let c_lib_path = c_lib_parent_path.with_added("source");
    let c_lib_tmp_path = c_lib_parent_path.with_added("source.new");
    if c_lib_tmp_path.as_path().exists() {
      try!(remove_dir_all(&c_lib_tmp_path));
    }
    try!(create_dir_all(&c_lib_tmp_path));
    log::info(format!("Generating C wrapper library ({}).", c_lib_name));

    let cpp_ffi_headers = try!(cpp_ffi_generator::run(&parse_result,
                                                      lib_spec.cpp.clone(),
                                                      input_cargo_toml_data.cpp_lib_name.clone(),
                                                      env.config.cpp_ffi_generator_filter())
      .chain_err(|| "FFI generator failed"));

    let mut cpp_libs_for_shared_c_lib = Vec::new();
    if c_lib_is_shared {
      let mut add_lib = |c: &String| {
        if !(is_msvc() && c == "GL") {
          cpp_libs_for_shared_c_lib.push(c.clone());
        }
      };
      for extra_lib in env.config.extra_libs() {
        add_lib(extra_lib);
      }
      for dep in &dependencies {
        for extra_lib in &dep.rust_export_info.extra_libs {
          add_lib(extra_lib);
        }
      }
    }
    let code_gen = CppCodeGenerator::new(c_lib_name.clone(),
                                         c_lib_tmp_path.clone(),
                                         c_lib_is_shared,
                                         cpp_libs_for_shared_c_lib);
    let include_dirs_str = try!(include_dirs.iter()
      .map_if_ok(|x| -> Result<_> { Ok(try!(path_to_str(x)).to_string()) }));
    let framework_dirs_str = try!(framework_dirs.iter()
      .map_if_ok(|x| -> Result<_> { Ok(try!(path_to_str(x)).to_string()) }));
    try!(code_gen.generate_template_files(env.config.include_directives(),
                                          &include_dirs_str,
                                          &framework_dirs_str));
    try!(code_gen.generate_files(&cpp_ffi_headers));

    try!(move_files(&c_lib_tmp_path, &c_lib_path));

    log::info("Building C wrapper library.");
    let c_lib_build_path = c_lib_parent_path.with_added("build");
    try!(create_dir_all(&c_lib_build_path));
    try!(create_dir_all(&c_lib_install_path));

    try!(CppLibBuilder {
        cmake_source_dir: &c_lib_path,
        build_dir: &c_lib_build_path,
        install_dir: &c_lib_install_path,
        num_jobs: num_jobs,
        linker_env_library_dirs: if c_lib_is_shared {
          Some(&cpp_lib_dirs)
        } else {
          None
        },
        pipe_output: env.pipe_output,
      }
      .run()
      .chain_err(|| "C wrapper build failed")
      .into());

    let crate_new_path = output_dir_path.with_added(format!("{}.new", &input_cargo_toml_data.name));
    if crate_new_path.as_path().exists() {
      try!(remove_dir_all(&crate_new_path));
    }
    try!(create_dir_all(&crate_new_path));
    let rustfmt_config_path = source_dir_path.with_added("rustfmt.toml");
    let rust_config = rust_code_generator::RustCodeGeneratorConfig {
      crate_name: input_cargo_toml_data.name.clone(),
      crate_authors: input_cargo_toml_data.authors.clone(),
      crate_version: input_cargo_toml_data.version.clone(),
      output_path: crate_new_path.clone(),
      template_path: source_dir_path.clone(),
      c_lib_name: c_lib_name,
      c_lib_is_shared: c_lib_is_shared,
      link_items: link_items,
      framework_dirs: framework_dirs_str,
      rustfmt_config_path: if rustfmt_config_path.as_path().exists() {
        Some(rustfmt_config_path)
      } else {
        None
      },
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
    let rust_data = try!(rust_generator::run(cpp_ffi_generator::CppAndFfiData {
                                               cpp_data: parse_result,
                                               cpp_ffi_headers: cpp_ffi_headers,
                                             },
                                             dependency_rust_types,
                                             rust_generator::RustGeneratorConfig {
                                               crate_name: input_cargo_toml_data.name.clone(),
                                               remove_qt_prefix: is_qt_library,
                                               qt_doc_data: qt_doc_data,
                                             })
      .chain_err(|| "Rust data generator failed"));
    log::info(format!("Generating Rust crate ({}).", &input_cargo_toml_data.name));
    try!(rust_code_generator::run(rust_config, &rust_data)
      .chain_err(|| "Rust code generator failed"));
    {
      let rust_export_path = output_dir_path.with_added("rust_export_info.json");
      try!(save_json(&rust_export_path,
                     &RustExportInfo {
                       crate_name: input_cargo_toml_data.name.clone(),
                       rust_types: rust_data.processed_types,
                       extra_libs: Vec::from(env.config.extra_libs()),
                     }));
      log::info(format!("Rust export info is saved to file: {}",
                        rust_export_path.display()));
    }

    for item in try!(read_dir(&crate_new_path)) {
      let item = try!(item);
      try!(move_files(&crate_new_path.with_added(item.file_name()),
                      &output_dir_path.with_added(item.file_name())));
    }
    try!(remove_dir(&crate_new_path));
  }


  // match env.invokation_method {
  // InvokationMethod::CommandLine => {
  // log::info("Compiling Rust crate.");
  // let mut all_cpp_lib_dirs = cpp_lib_dirs.clone();
  // if c_lib_is_shared {
  // all_cpp_lib_dirs.push(c_lib_lib_path.clone());
  // }
  // if output_dir_path.with_added("Cargo.lock").exists() {
  // try!(remove_file(output_dir_path.with_added("Cargo.lock")));
  // }
  // for cargo_cmd in &["build", "test", "doc"] {
  // let mut command = Command::new("cargo");
  // command.arg(cargo_cmd);
  // command.arg("--verbose");
  // command.arg(format!("-j{}", num_jobs));
  // command.current_dir(&output_dir_path);
  // if !all_cpp_lib_dirs.is_empty() {
  // for name in &["LIBRARY_PATH", "LD_LIBRARY_PATH", "LIB", "PATH"] {
  // let value = try!(add_env_path_item(name, all_cpp_lib_dirs.clone()));
  // command.env(name, value);
  // }
  // }
  // if !framework_dirs.is_empty() {
  // command.env("DYLD_FRAMEWORK_PATH",
  // try!(add_env_path_item("DYLD_FRAMEWORK_PATH", framework_dirs.clone())));
  // }
  // if is_msvc() && *cargo_cmd == "test" {
  // cargo doesn't pass this flag to rustc when it compiles qt_core,
  // so it's compiled with static std and the tests fail with
  // "cannot satisfy dependencies so `std` only shows up once" error.
  // command.env("RUSTFLAGS", "-C prefer-dynamic");
  // }
  // try!(run_command(&mut command, false, env.pipe_output)
  // .chain_err(|| "failed to build generated crate"));
  // }
  // log::info("Completed successfully.");
  // }
  // InvokationMethod::BuildScript => {
  println!("cargo:rustc-link-search={}",
           try!(path_to_str(&c_lib_lib_path)));
  for dir in &cpp_lib_dirs {
    println!("cargo:rustc-link-search=native={}", try!(path_to_str(dir)));
  }
  println!("cargo:cpp_to_rust_data_path={}",
           try!(path_to_str(&output_dir_path)));
  for dir in &framework_dirs {
    println!("cargo:rustc-link-search=framework={}",
             try!(path_to_str(dir)));
  }
  Ok(())
}
