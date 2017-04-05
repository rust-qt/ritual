pub extern crate cpp_to_rust_common as common;
use common::errors::{fancy_unwrap, ChainErr, Result};
use common::cpp_build_config::{CppBuildConfig, CppBuildPaths, CppLibraryType};
use common::build_script_data::BuildScriptData;
use common::file_utils::{PathBufWithAdded, load_json, create_file, file_to_string, path_to_str};
use common::cpp_lib_builder::{CppLibBuilder, CMakeVar, BuildType};
use common::target::current_target;
use common::utils::{get_command_output, exe_suffix};
use common::log;

use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub struct Config {
  cpp_build_paths: CppBuildPaths,
  cpp_build_config: CppBuildConfig,
}

fn manifest_dir() -> Result<PathBuf> {
  let dir =
    std::env::var("CARGO_MANIFEST_DIR").chain_err(|| "CARGO_MANIFEST_DIR env var is missing")?;
  Ok(PathBuf::from(dir))
}
fn out_dir() -> Result<PathBuf> {
  let dir = std::env::var("OUT_DIR").chain_err(|| "OUT_DIR env var is missing")?;
  Ok(PathBuf::from(dir))
}

fn build_script_data() -> Result<BuildScriptData> {
  load_json(manifest_dir()?.with_added("build_script_data.json"))
}

impl Config {
  pub fn new() -> Result<Config> {
    Ok(Config {
         cpp_build_config: build_script_data()?.cpp_build_config,
         cpp_build_paths: CppBuildPaths::default(),
       })
  }
  pub fn cpp_build_config(&self) -> &CppBuildConfig {
    &self.cpp_build_config
  }
  pub fn cpp_build_config_mut(&mut self) -> &mut CppBuildConfig {
    &mut self.cpp_build_config
  }
  pub fn set_cpp_build_config(&mut self, config: CppBuildConfig) {
    self.cpp_build_config = config;
  }
  pub fn cpp_build_paths(&self) -> &CppBuildPaths {
    &self.cpp_build_paths
  }
  pub fn cpp_build_paths_mut(&mut self) -> &mut CppBuildPaths {
    &mut self.cpp_build_paths
  }
  pub fn set_cpp_build_paths(&mut self, config: CppBuildPaths) {
    self.cpp_build_paths = config;
  }


  pub fn run_and_return(mut self) -> Result<()> {
    self.cpp_build_paths.apply_env();
    let build_script_data = build_script_data()?;
    let cpp_build_config_data = self.cpp_build_config.eval(&current_target())?;
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
    let profile = std::env::var("PROFILE").chain_err(|| "PROFILE env var is missing")?;
    log::status("Building C++ wrapper library");
    CppLibBuilder {
        cmake_source_dir: manifest_dir.with_added("c_lib"),
        build_dir: out_dir.with_added("c_lib_build"),
        install_dir: c_lib_install_dir.clone(),
        num_jobs: std::env::var("NUM_JOBS")
          .ok()
          .and_then(|x| x.parse().ok()),
        pipe_output: false,
        cmake_vars: cmake_vars,
        build_type: match profile.as_str() {
          "debug" => BuildType::Debug,
          "release" => BuildType::Release,
          _ => return Err(format!("unknown value of PROFILE env var: {}", profile).into()),
        },
      }.run()?;
    {
      log::status("Generating ffi.rs file");
      let mut ffi_file = create_file(out_dir.with_added("ffi.rs"))?;
      for name in cpp_build_config_data.linked_libs() {
        ffi_file.write(format!("#[link(name = \"{}\")]\n", name))?;
      }
      for name in cpp_build_config_data.linked_frameworks() {
        ffi_file.write(format!("#[link(name = \"{}\", kind = \"framework\")]\n", name))?;
      }
      if !::common::utils::is_msvc() {
        // TODO: make it configurable
        ffi_file.write("#[link(name = \"stdc++\")]\n")?;
      }
      if cpp_build_config_data.library_type() == Some(CppLibraryType::Shared) {
        ffi_file.write(format!("#[link(name = \"{}\")]\n",
                         &build_script_data.cpp_wrapper_lib_name))?;
      } else {
        ffi_file.write(format!("#[link(name = \"{}\", kind = \"static\")]\n",
                         &build_script_data.cpp_wrapper_lib_name))?;
      }
      ffi_file.write(file_to_string(manifest_dir.with_added("src").with_added("ffi.in.rs"))?)?;
    }
    {
      log::status("Requesting type sizes");
      let mut command = Command::new(c_lib_install_dir
                                       .with_added("lib")
                                       .with_added(format!("type_sizes{}", exe_suffix())));
      let mut file = create_file(out_dir.with_added("type_sizes.rs"))?;
      file.write(get_command_output(&mut command)?)?;
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
  pub fn run(self) -> ! {
    fancy_unwrap(self.run_and_return());
    std::process::exit(0)
  }
}


pub fn run_and_return() -> Result<()> {
  Config::new()?.run_and_return()
}

pub fn run() -> ! {
  let config = fancy_unwrap(Config::new());
  config.run()
}
