//! Implementation of build script for all Qt crates
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust)
//! for more information.

use itertools::Itertools;
use qt_ritual_common::get_full_build_config;
use ritual_build::common::errors::{FancyUnwrap, Result};
use ritual_build::common::target::current_target;
use ritual_build::common::utils::MapIfOk;
use ritual_build::Config;
use semver::Version;

#[allow(clippy::op_ref)] // false positive
fn detect_closest_version(known: &[&str], current: &str) -> Result<Option<String>> {
    let known = known.map_if_ok(|i| Version::parse(i))?;
    let current = Version::parse(current)?;

    if known.contains(&current) {
        return Ok(Some(current.to_string()));
    }

    let same_patch = known
        .iter()
        .filter(|v| v.major == current.major && v.minor == current.minor)
        .collect_vec();

    if !same_patch.is_empty() {
        if let Some(version) = same_patch.iter().filter(|&&v| v < &current).max() {
            return Ok(Some(version.to_string()));
        }
        return Ok(Some(same_patch.iter().min().unwrap().to_string()));
    }

    if let Some(version) = known.iter().filter(|&v| v < &current).max() {
        return Ok(Some(version.to_string()));
    } else {
        return Ok(None);
    }
}

/// Runs the build script.
pub fn run_and_return(crate_name: &str) -> Result<()> {
    let qt_config = get_full_build_config(crate_name, None)?;

    let mut config = Config::new()?;

    let current_target = current_target();
    let known_library_versions = config
        .known_targets()
        .iter()
        .filter(|item| item.target == current_target)
        .map(|item| {
            item.cpp_library_version
                .as_ref()
                .expect("qt crates should always have reported library version")
                .as_str()
        })
        .collect_vec();

    if known_library_versions.contains(&qt_config.installation_data.qt_version.as_str()) {
        config.set_current_cpp_library_version(Some(qt_config.installation_data.qt_version));
    } else {
        match detect_closest_version(
            &known_library_versions,
            &qt_config.installation_data.qt_version,
        ) {
            Ok(Some(version)) => {
                println!(
                    "cargo:warning=Current Qt version ({}) is unknown to {} crate. \
                     Using closest known version ({})",
                    qt_config.installation_data.qt_version, crate_name, version
                );
                config.set_current_cpp_library_version(Some(version));
            }
            Ok(None) => {
                println!("This crate supports the following targets:");
                for target in config.known_targets() {
                    println!("* {}", target.short_text());
                }
                panic!(
                    "Unsupported Qt version: {}",
                    qt_config.installation_data.qt_version
                );
            }
            Err(error) => {
                println!(
                    "cargo:warning=Error while choosing known version: {}",
                    error
                );
            }
        }
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

#[test]
fn versions() {
    assert_eq!(
        detect_closest_version(&["5.11.0", "5.12.2"], "5.13.1").unwrap(),
        Some("5.12.2".to_string())
    );
    assert_eq!(
        detect_closest_version(&["5.11.0", "5.9.7", "5.12.2"], "5.9.1").unwrap(),
        Some("5.9.7".to_string())
    );
    assert_eq!(
        detect_closest_version(&["5.11.0", "5.10.7", "5.12.2"], "5.9.1").unwrap(),
        None
    );
    assert_eq!(
        detect_closest_version(&["5.11.0", "5.9.7", "5.12.2"], "5.10.1").unwrap(),
        Some("5.9.7".to_string())
    );
    assert_eq!(
        detect_closest_version(&["5.11.0", "5.9.7", "5.12.2"], "5.11.2").unwrap(),
        Some("5.11.0".to_string())
    );
    assert_eq!(
        detect_closest_version(&["5.11.2", "5.9.7", "5.12.2"], "5.11.0").unwrap(),
        Some("5.11.2".to_string())
    );
}
