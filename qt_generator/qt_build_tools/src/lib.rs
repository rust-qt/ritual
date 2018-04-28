//! Implementation of build script for all Qt crates
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust)
//! for more information.

extern crate cpp_to_rust_build_tools;
extern crate qt_generator_common;

use cpp_to_rust_build_tools::Config;
use cpp_to_rust_build_tools::common::errors::{fancy_unwrap, ChainErr, Result};
use cpp_to_rust_build_tools::common::target;
use cpp_to_rust_build_tools::common::cpp_build_config::CppBuildConfigData;
use qt_generator_common::{framework_name, get_full_build_config, get_installation_data,
                          lib_dependencies, real_lib_name, InstallationData};

/// Runs the build script.
pub fn run_and_return(sublib_name: &str) -> Result<()> {
  let qt_config = get_full_build_config()?;

  let mut config = Config::new()?;
  {
    let original_qt_version = config
      .original_cpp_lib_version()
      .chain_err(|| "cpp_lib_version is expected in Config")?;

    if original_qt_version != qt_config.installation_data.qt_version {
      println!(
        "cargo:warning=This crate was generated for Qt {}, but Qt {} is currently in use.",
        original_qt_version, installation_data.qt_version
      );
    }
  }
  config.set_cpp_build_config(qt_config.cpp_build_config);
  config.set_cpp_build_paths(qt_config.cpp_build_paths);
  config.run_and_return()
}

/// Runs the build script and exits the process with an appropriate exit code.
pub fn run(sublib_name: &str) -> ! {
  fancy_unwrap(run_and_return(sublib_name));
  std::process::exit(0)
}
