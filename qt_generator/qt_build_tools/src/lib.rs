//! Implementation of build script for all Qt crates
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust)
//! for more information.

extern crate cpp_to_rust_build_tools;
extern crate qt_generator_common;

use cpp_to_rust_build_tools::Config;
use cpp_to_rust_build_tools::common::errors::{fancy_unwrap, Result, ChainErr};
use cpp_to_rust_build_tools::common::target;
use cpp_to_rust_build_tools::common::cpp_build_config::CppBuildConfigData;
use qt_generator_common::{InstallationData, get_installation_data, real_lib_name, framework_name,
                          lib_dependencies};

/// Runs the build script.
pub fn run_and_return(sublib_name: &str) -> Result<()> {
  let installation_data = get_installation_data(sublib_name)?;

  let mut config = Config::new()?;
  {
    let original_qt_version = config.original_cpp_lib_version().chain_err(
      || "cpp_lib_version is expected in Config",
    )?;

    if original_qt_version != installation_data.qt_version {
      println!(
        "cargo:warning=This crate was generated for Qt {}, but Qt {} is currently in use.",
        original_qt_version,
        installation_data.qt_version
      );
    }
  }

  let mut cpp_build_config_data = CppBuildConfigData::new();
  {
    let mut apply_installation_data = |name: &str, data: &InstallationData| {
      config.cpp_build_paths_mut().add_include_path(
        &data.root_include_path,
      );
      config.cpp_build_paths_mut().add_include_path(
        &data.lib_include_path,
      );
      if data.is_framework {
        config.cpp_build_paths_mut().add_framework_path(
          &data.lib_path,
        );
        cpp_build_config_data.add_linked_framework(framework_name(name));
      } else {
        config.cpp_build_paths_mut().add_lib_path(&data.lib_path);
        cpp_build_config_data.add_linked_lib(real_lib_name(name));
      }
    };


    apply_installation_data(sublib_name, &installation_data);
    for dep in lib_dependencies(sublib_name)? {
      let dep_data = get_installation_data(dep)?;
      apply_installation_data(dep, &dep_data);
    }

  }

  config.cpp_build_config_mut().add(
    target::Condition::True,
    cpp_build_config_data,
  );
  config.run_and_return()
}

/// Runs the build script and exits the process with an appropriate exit code.
pub fn run(sublib_name: &str) -> ! {
  fancy_unwrap(run_and_return(sublib_name));
  std::process::exit(0)
}
