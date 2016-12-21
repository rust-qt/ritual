pub extern crate cpp_to_rust_common as common;
use common::errors::{fancy_unwrap, ChainErr, Result};
use common::cpp_build_config::{CppBuildConfig, CppBuildPaths};
use common::file_utils::{PathBufWithAdded, load_json};
use common::cpp_lib_builder::CppLibBuilder;

use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Config {
  cpp_build_paths: Option<CppBuildPaths>,
  cpp_build_config: Option<CppBuildConfig>,
}

fn manifest_dir() -> Result<PathBuf> {
  let dir = std::env::var("CARGO_MANIFEST_DIR").chain_err(|| "CARGO_MANIFEST_DIR env var is missing")?;
  Ok(PathBuf::from(dir))
}
fn out_dir() -> Result<PathBuf> {
  let dir = std::env::var("OUT_DIR").chain_err(|| "OUT_DIR env var is missing")?;
  Ok(PathBuf::from(dir))
}


pub fn default_cpp_build_config() -> Result<CppBuildConfig> {
  load_json(manifest_dir()?.with_added("rust_export_info.json"))
}

impl Config {
  pub fn new() -> Config {
    Config::default()
  }
  pub fn override_cpp_build_config(&mut self, config: CppBuildConfig) {
    self.cpp_build_config = Some(config);
  }
  pub fn set_cpp_build_paths(&mut self, config: CppBuildPaths) {
    self.cpp_build_paths = Some(config);
  }

  pub fn run_and_return(self) -> Result<()> {
    let cpp_build_paths = self.cpp_build_paths.unwrap_or_default();
    let cpp_build_config = if let Some(x) = self.cpp_build_config {
      x
    } else {
      default_cpp_build_config()?
    };
    CppLibBuilder {
      cmake_source_dir: manifest_dir()?.with_added("c_lib"),
      build_dir: out_dir()?.with_added("c_lib_build"),
      install_dir: out_dir()?.with_added("c_lib_install"),
      num_jobs: std::env::var("NUM_JOBS").ok().and_then(|x| x.parse().ok()),
      pipe_output: false,
      cmake_vars: Vec::new(),
    }.run()?;

    // TODO: get struct sizes
    // TODO: output build script variables for cargo
    Ok(())
  }
  pub fn run(self) -> ! {
    fancy_unwrap(self.run_and_return());
    std::process::exit(0)
  }
}


pub fn run_and_return() -> Result<()> {
  Config::default().run_and_return()
}

pub fn run() -> ! {
  Config::default().run()
}
