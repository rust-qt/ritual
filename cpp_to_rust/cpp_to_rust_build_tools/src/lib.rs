//! Implementation of `cpp_to_rust`'s build script.
//! Default generated build script uses this crate as a build dependency
//! and just calls `cpp_to_rust_build_tools::run()`.
//! If a custom build script is used, it should use this crate's API
//! to perform necessary build steps.


pub extern crate cpp_to_rust_common as common;
use common::errors::{fancy_unwrap, ChainErr, Result};
use common::cpp_build_config::{CppBuildConfig, CppBuildPaths, CppLibraryType};
use common::BuildScriptData;
use common::file_utils::{PathBufWithAdded, load_json, create_file, file_to_string, path_to_str};
use common::cpp_lib_builder::{CppLibBuilder, CMakeVar, BuildType};
use common::target::current_target;
use common::utils::{get_command_output, exe_suffix};
use common::log;

use std::path::PathBuf;
use std::process::Command;

/// Configuration of the build script.
#[derive(Debug)]
pub struct Config {
  cpp_build_paths: CppBuildPaths,
  build_script_data: BuildScriptData,
}

fn manifest_dir() -> Result<PathBuf> {
  let dir = std::env::var("CARGO_MANIFEST_DIR")
    .chain_err(|| "CARGO_MANIFEST_DIR env var is missing")?;
  Ok(PathBuf::from(dir))
}
fn out_dir() -> Result<PathBuf> {
  let dir = std::env::var("OUT_DIR")
    .chain_err(|| "OUT_DIR env var is missing")?;
  Ok(PathBuf::from(dir))
}

fn build_script_data() -> Result<BuildScriptData> {
  load_json(manifest_dir()?.with_added("build_script_data.json"))
}

impl Config {
  /// Constructs default configuration state based on
  /// information in the generated `build_script_data.json` file
  /// located at the crate root. The caller may change
  /// `CppBuildPaths` and `CppBuildConfig` values stored in this object
  /// and call `config.run()` to apply them.
  pub fn new() -> Result<Config> {
    Ok(Config {
         build_script_data: build_script_data()?,
         cpp_build_paths: CppBuildPaths::default(),
       })
  }

  /// Returns version of the native C++ library used for generating this crate.
  /// This is the value set with `Config::set_cpp_lib_version` during generation,
  /// or `None` if the version was not set.
  pub fn original_cpp_lib_version(&self) -> Option<&str> {
    self
      .build_script_data
      .cpp_lib_version
      .as_ref()
      .map(|x| x.as_str())
  }

  /// Returns current `CppBuildConfig` data.
  pub fn cpp_build_config(&self) -> &CppBuildConfig {
    &self.build_script_data.cpp_build_config
  }
  /// Returns mutable `CppBuildConfig` data.
  pub fn cpp_build_config_mut(&mut self) -> &mut CppBuildConfig {
    &mut self.build_script_data.cpp_build_config
  }
  /// Sets new `CppBuildConfig` data.
  pub fn set_cpp_build_config(&mut self, config: CppBuildConfig) {
    self.build_script_data.cpp_build_config = config;
  }
  /// Returns current `CppBuildPaths` data.
  pub fn cpp_build_paths(&self) -> &CppBuildPaths {
    &self.cpp_build_paths
  }
  /// Returns mutable `CppBuildPaths` data.
  pub fn cpp_build_paths_mut(&mut self) -> &mut CppBuildPaths {
    &mut self.cpp_build_paths
  }
  /// Sets new `CppBuildPaths` data.
  pub fn set_cpp_build_paths(&mut self, config: CppBuildPaths) {
    self.cpp_build_paths = config;
  }

  /// Same as `run()`, but result of the operation is returned to the caller.
  pub fn run_and_return(mut self) -> Result<()> {
    self.cpp_build_paths.apply_env();
    let cpp_build_config_data = self
      .build_script_data
      .cpp_build_config
      .eval(&current_target())?;
    let mut cmake_vars = Vec::new();
    cmake_vars.push(CMakeVar::new("C2R_LIBRARY_TYPE",
                                  match cpp_build_config_data.library_type() {
                                    Some(CppLibraryType::Shared) => "SHARED",
                                    Some(CppLibraryType::Static) |
                                    None => "STATIC",
                                  }));
    cmake_vars.push(CMakeVar::new_path_list("C2R_INCLUDE_PATHS",
                                            self.cpp_build_paths.include_paths())?);
    cmake_vars.push(CMakeVar::new_path_list("C2R_LIB_PATHS", self.cpp_build_paths.lib_paths())?);
    cmake_vars.push(CMakeVar::new_path_list("C2R_FRAMEWORK_PATHS",
                                            self.cpp_build_paths.framework_paths())?);
    cmake_vars.push(CMakeVar::new_list("C2R_LINKED_LIBS", cpp_build_config_data.linked_libs())?);
    cmake_vars.push(CMakeVar::new_list("C2R_LINKED_FRAMEWORKS",
                                       cpp_build_config_data.linked_frameworks())?);
    cmake_vars.push(CMakeVar::new("C2R_COMPILER_FLAGS",
                                  cpp_build_config_data.compiler_flags().join(" ")));
    let out_dir = out_dir()?;
    let c_lib_install_dir = out_dir.with_added("c_lib_install");
    let manifest_dir = manifest_dir()?;
    let profile = std::env::var("PROFILE")
      .chain_err(|| "PROFILE env var is missing")?;
    log::status("Building C++ wrapper library");
    CppLibBuilder {
        cmake_source_dir: manifest_dir.with_added("c_lib"),
        build_dir: out_dir.with_added("c_lib_build"),
        install_dir: c_lib_install_dir.clone(),
        num_jobs: std::env::var("NUM_JOBS")
          .ok()
          .and_then(|x| x.parse().ok()),
        cmake_vars: cmake_vars,
        build_type: match profile.as_str() {
          "debug" => BuildType::Debug,
          "release" => BuildType::Release,
          _ => return Err(format!("unknown value of PROFILE env var: {}", profile).into()),
        },
      }
      .run()?;
    {
      log::status("Generating ffi.rs file");
      let mut ffi_file = create_file(out_dir.with_added("ffi.rs"))?;
      if ::common::target::current_env() != ::common::target::Env::Msvc {
        // TODO: make it configurable
        ffi_file.write("#[link(name = \"stdc++\")]\n")?;
      }
      if cpp_build_config_data.library_type() == Some(CppLibraryType::Shared) {
        ffi_file
          .write(format!("#[link(name = \"{}\")]\n",
                         &self.build_script_data.cpp_wrapper_lib_name))?;
      } else {
        ffi_file
          .write(format!("#[link(name = \"{}\", kind = \"static\")]\n",
                         &self.build_script_data.cpp_wrapper_lib_name))?;
      }
      ffi_file
        .write(file_to_string(manifest_dir.with_added("src").with_added("ffi.in.rs"))?)?;
    }
    {
      log::status("Requesting type sizes");
      let mut command = Command::new(c_lib_install_dir
                                       .with_added("lib")
                                       .with_added(format!("type_sizes{}", exe_suffix())));
      let mut file = create_file(out_dir.with_added("type_sizes.rs"))?;
      file.write(get_command_output(&mut command)?)?;
    }

    for name in cpp_build_config_data.linked_libs() {
      println!("cargo:rustc-link-lib={}", name);
    }
    for name in cpp_build_config_data.linked_frameworks() {
      println!("cargo:rustc-link-lib=framework={}", name);
    }
    for path in self.cpp_build_paths.lib_paths() {
      println!("cargo:rustc-link-search=native={}", path_to_str(path)?);
    }
    for path in self.cpp_build_paths.framework_paths() {
      println!("cargo:rustc-link-search=framework={}", path_to_str(path)?);
    }
    println!("cargo:rustc-link-search=native={}",
             path_to_str(&c_lib_install_dir.with_added("lib"))?);
    log::status("cpp_to_rust build script finished.");
    Ok(())
  }

  /// Starts build script with current configuration.
  /// The build script performs the following operations:
  ///
  /// - Build the C++ wrapper library;
  /// - Generate `ffi.rs` file with actual link attributes;
  /// - Determine C++ type sizes on current platform and generate `type_sizes.rs`;
  /// - Report linking information to `cargo`.
  ///
  /// This function ends the process with the appropriate error code and never
  /// returns to the caller.
  pub fn run(self) -> ! {
    fancy_unwrap(self.run_and_return());
    std::process::exit(0)
  }
}


/// Same as `run()`, but result of the operation is returned to the caller.
pub fn run_and_return() -> Result<()> {
  Config::new()?.run_and_return()
}

/// Runs the build script with default configuration.
/// See `Config::run` for more information.
pub fn run() -> ! {
  let config = fancy_unwrap(Config::new());
  config.run()
}
