pub extern crate cpp_to_rust_common as common;
use common::errors::{fancy_unwrap, Result};
use common::cpp_build_config::{CppBuildConfig, CppBuildPaths};

#[derive(Debug, Default)]
pub struct Config {
  cpp_build_paths: Option<CppBuildPaths>,
  cpp_build_config: Option<CppBuildConfig>,
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
      // TODO: read from json
      unimplemented!()
    };

    // TODO: get struct sizes
    // TODO: output build script variables for cargo
    unimplemented!()

  }
  pub fn run(self) -> ! {
    fancy_unwrap(self.run_and_return());
    std::process::exit(0);
  }
}


pub fn run_and_return() -> Result<()> {
  Config::default().run_and_return()
}

pub fn run() -> ! {
  Config::default().run()
}
