//! Implementation of build script for all Qt crates
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust)
//! for more information.

use qt_ritual_common::get_full_build_config;
use ritual_build::common::cpp_lib_builder::CMakeVar;
use ritual_build::common::errors::{FancyUnwrap, Result};
use ritual_build::Config;

/// Runs the build script.
pub fn run_and_return(crate_name: &str) -> Result<()> {
    let qt_config = get_full_build_config(crate_name)?;

    let mut config = Config::new()?;
    config.add_cmake_var(CMakeVar::new("RITUAL_QT", "1"));
    config.set_current_cpp_library_version(Some(qt_config.installation_data.qt_version));
    config.set_cpp_build_config(qt_config.cpp_build_config);
    config.set_cpp_build_paths(qt_config.cpp_build_paths);
    config.run_and_return()
}

/// Runs the build script and exits the process with an appropriate exit code.
pub fn run(crate_name: &str) -> ! {
    run_and_return(crate_name).fancy_unwrap();
    std::process::exit(0);
}
