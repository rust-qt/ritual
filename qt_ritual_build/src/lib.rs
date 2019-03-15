//! Implementation of build script for all Qt crates
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust)
//! for more information.

use qt_ritual_common::get_full_build_config;
use ritual_build::common::errors::{err_msg, FancyUnwrap, Result};
use ritual_build::Config;

/// Runs the build script.
pub fn run_and_return(crate_name: &str) -> Result<()> {
    let qt_config = get_full_build_config(crate_name)?;

    let mut config = Config::new()?;

    let original_qt_version = config
        .original_cpp_lib_version()
        .ok_or_else(|| err_msg("cpp_lib_version is expected in Config"))?;

    if original_qt_version != qt_config.installation_data.qt_version {
        println!(
            "cargo:warning=This crate was generated for Qt {}, but Qt {} is currently in use.",
            original_qt_version, qt_config.installation_data.qt_version
        );
    }

    config.set_cpp_build_config(qt_config.cpp_build_config);
    config.set_cpp_build_paths(qt_config.cpp_build_paths);
    config.run_and_return()
}

/// Runs the build script and exits the process with an appropriate exit code.
pub fn run(crate_name: &str) -> ! {
    run_and_return(crate_name).fancy_unwrap();
    std::process::exit(0);
}
