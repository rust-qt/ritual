extern crate num_cpus;

use config::Config;
use cpp_code_generator::CppCodeGenerator;
use cpp_ffi_generator;
use cpp_lib_builder::CppLibBuilder;
use cpp_parser;
use dependency_info::DependencyInfo;
use errors::{Result, ChainErr};
use file_utils::{PathBufWithAdded, move_files, create_dir_all, load_json, save_json, canonicalize,
                 remove_dir_all, remove_dir, read_dir, path_to_str, create_file};
use log;
use rust_code_generator::{RustCodeGeneratorDependency, RustLinkItem, RustLinkKind};
use rust_code_generator;
use rust_generator;
use rust_info::{InputCargoTomlData, RustExportInfo};
use string_utils::JoinWithString;
use utils::{is_msvc, MapIfOk};

use std::path::PathBuf;
use std;

pub enum BuildProfile {
  Debug,
  Release,
}

pub struct BuildEnvironment {
  pub config: Config,
  pub output_dir_path: PathBuf,
  pub source_dir_path: PathBuf,
  pub dependency_paths: Vec<PathBuf>,
  pub num_jobs: Option<i32>,
  pub build_profile: BuildProfile,
  pub pipe_output: bool,
  pub aggressive_caching: bool,
}

pub fn run_from_build_script(config: Config) -> Result<()> {
  let mut dependency_paths = Vec::new();
  for (name, value) in std::env::vars_os() {
    if let Ok(name) = name.into_string() {
      if name.starts_with("DEP_") && name.ends_with("_CPP_TO_RUST_DATA_PATH") {
        log::info(format!("Found dependency: {}", value.to_string_lossy()));
        dependency_paths.push(PathBuf::from(value));
      }
    }
  }
  let source_dir_path = PathBuf::from(try!(std::env::var("CARGO_MANIFEST_DIR")
    .chain_err(|| "failed to read required env var: CARGO_MANIFEST_DIR")));
  let out_dir = PathBuf::from(try!(std::env::var("OUT_DIR")
    .chain_err(|| "failed to read required env var: OUT_DIR")));
  let mut aggressive_caching = false;
  let output_dir_path = if let Ok(cache) = std::env::var("CPP_TO_RUST_CACHE") {
    let input_cargo_toml_path = source_dir_path.with_added("Cargo.toml");
    if !input_cargo_toml_path.exists() {
      return Err(format!("Input Cargo.toml does not exist: {}",
                         input_cargo_toml_path.display())
        .into());
    }
    let input_cargo_toml_data = try!(InputCargoTomlData::from_file(&input_cargo_toml_path));
    let cache_path = PathBuf::from(cache).with_added(&input_cargo_toml_data.name);
    let lib_in_file_path = out_dir.with_added("lib.in.rs");
    let mut lib_in_file = try!(create_file(lib_in_file_path));
    try!(lib_in_file.write(format!("include!(\"{}\");\n",
                                   cache_path.with_added("lib.in.rs").display())));
    log::info(format!("Using cache directory: {}", cache_path.display()));
    aggressive_caching = true;
    cache_path
  } else {
    out_dir
  };
  run(BuildEnvironment {
    config: config,
    source_dir_path: source_dir_path,
    output_dir_path: output_dir_path,
    num_jobs: try!(std::env::var("NUM_JOBS")
        .chain_err(|| "failed to read required env var: NUM_JOBS"))
      .parse()
      .ok(),
    build_profile: match try!(std::env::var("PROFILE")
        .chain_err(|| "failed to read required env var: PROFILE"))
      .as_ref() {
      "debug" | "test" | "doc" => BuildProfile::Debug,
      "release" | "bench" => BuildProfile::Release,
      a => return Err(format!("unsupported profile: {}", a).into()),
    },
    dependency_paths: dependency_paths,
    pipe_output: false,
    aggressive_caching: aggressive_caching,
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

  log::info("Reading input Cargo.toml");
  let input_cargo_toml_path = source_dir_path.with_added("Cargo.toml");
  if !input_cargo_toml_path.exists() {
    return Err(format!("Input Cargo.toml does not exist: {}",
                       input_cargo_toml_path.display())
      .into());
  }
  let input_cargo_toml_data = try!(InputCargoTomlData::from_file(&input_cargo_toml_path));
  if env.config.linked_libs().iter().any(|x| x == &input_cargo_toml_data.name) {
    return Err(format!("Rust crate name ({}) must not be the same as linked library name \
            because it can cause library name conflict and linker failure.",
                       input_cargo_toml_data.name)
      .into());
  }

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
  let cpp_lib_dirs = Vec::from(env.config.lib_paths());
  let framework_dirs = Vec::from(env.config.framework_paths());
  let include_dirs = Vec::from(env.config.include_paths());
  let target_include_dirs = Vec::from(env.config.target_include_paths());
  let mut link_items = Vec::new();
  for item in env.config.linked_libs() {
    link_items.push(RustLinkItem {
      name: item.to_string(),
      kind: RustLinkKind::SharedLibrary,
    });
  }
  for item in env.config.linked_frameworks() {
    link_items.push(RustLinkItem {
      name: item.to_string(),
      kind: RustLinkKind::Framework,
    });
  }
  if let Some(ref links) = input_cargo_toml_data.links {
    if !link_items.iter().any(|x| &x.name == links) {
      log::warning(format!("Value of 'links' field in Cargo.toml ({}) should be one of \
        linked libraries or frameworks ({}).",
                           links,
                           link_items.iter().map(|x| &x.name).join(", ")));
    }
  } else {
    log::warning("It's recommended to add 'links' field to Cargo.toml.");
  }

  // TODO: move other effects of this var to qt_build_tools
  let is_qt_library = link_items.iter().any(|x| x.name.starts_with("Qt"));

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
  if env.aggressive_caching &&
     output_dir_path.with_added("cpp_to_rust_completed").as_path().exists() {
    log::info("No processing! cpp_to_rust uses previous results.");
  } else {
    let parse_result_cache_file_path = output_dir_path.with_added("cpp_data.json");
    let loaded_parse_result = if env.aggressive_caching &&
                                 parse_result_cache_file_path.as_path().is_file() {
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
                               include_paths: include_dirs.clone(),
                               framework_paths: framework_dirs.clone(),
                               include_directives: Vec::from(env.config.include_directives()),
                               target_include_paths: target_include_dirs,
                               tmp_cpp_path: output_dir_path.with_added("1.cpp"),
                               name_blacklist: Vec::from(env.config.cpp_parser_blocked_names()),
                               flags: Vec::from(env.config.cpp_parser_flags()),
                             },
                             &dependencies.iter().map(|x| &x.cpp_data).collect::<Vec<_>>())
          .chain_err(|| "C++ parser failed"));
      for filter in env.config.cpp_data_filters() {
        try!(filter(&mut parse_result).chain_err(|| "cpp_data_filter failed"));
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
                                                      c_lib_name.clone(),
                                                      env.config.cpp_ffi_generator_filters())
      .chain_err(|| "FFI generator failed"));

    let mut cpp_libs_for_shared_c_lib = Vec::new();
    if c_lib_is_shared {
      for lib in env.config.linked_libs() {
        cpp_libs_for_shared_c_lib.push(lib.clone());
      }
      for dep in &dependencies {
        for lib in &dep.rust_export_info.linked_libs {
          cpp_libs_for_shared_c_lib.push(lib.clone());
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
                                          &framework_dirs_str,
                                          env.config.cpp_compiler_flags()));
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
      final_output_path: output_dir_path.clone(),
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
                       linked_libs: Vec::from(env.config.linked_libs()),
                       linked_frameworks: Vec::from(env.config.linked_frameworks()),
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
  try!(create_file(output_dir_path.with_added("cpp_to_rust_completed")));
  Ok(())
}
