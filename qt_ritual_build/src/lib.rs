//! Implementation of build script for all Qt crates
//!
//! See [README](https://github.com/rust-qt/ritual) of the repository root for more information.
//!
//! The build script uses `qmake` available in `PATH` to determine paths to the Qt installation and passes them to
//! `ritual_build`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use itertools::Itertools;
use qt_ritual_common::get_full_build_config;
use ritual_build::common::errors::{bail, FancyUnwrap, Result, ResultExt};
use ritual_build::common::file_utils::{create_file, path_to_str};
use ritual_build::common::target;
use ritual_build::common::utils::{run_command, MapIfOk};
use ritual_build::Config;
use semver::Version;
use sha1::{Digest, Sha1};
use std::env;
use std::fs::create_dir_all;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

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
        Ok(Some(version.to_string()))
    } else {
        Ok(None)
    }
}

/// Runs the build script.
pub fn try_run(crate_name: &str) -> Result<()> {
    env_logger::init();

    let qt_config = get_full_build_config(crate_name, None)?;

    let mut config = Config::new()?;

    let known_library_versions = config
        .known_targets()
        .iter()
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
                    "Current Qt version ({}) is unknown to {} crate. \
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
    config.try_run()
}

/// Runs the build script and exits the process with an appropriate exit code.
pub fn run(crate_name: &str) -> ! {
    try_run(crate_name).fancy_unwrap();
    std::process::exit(0);
}

/// Builds and links [Qt resource files](https://doc.qt.io/qt-5/resources.html).
///
/// Note that the resource file may not be rebuilt when the resource file or files referenced
/// for it are changed. You may have to run `cargo clean -p crate_name` to force a rebuild.
pub fn try_add_resources(paths: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<()> {
    let paths = paths
        .into_iter()
        .map(|path| path.as_ref().to_path_buf())
        .collect_vec();
    for path in &paths {
        if !path.is_file() {
            bail!("not a file: {}", path.display());
        }
    }
    let mut hasher = Sha1::new();
    hasher.input(format!("{:?}", paths));
    let project_name = format!("qt_resources_{:x}", hasher.result());

    let out_dir =
        PathBuf::from(env::var("OUT_DIR").with_context(|_| "OUT_DIR env var is missing")?);
    let dir = out_dir.join(&project_name);
    create_dir_all(&dir)?;

    let pro_file_path = dir.join(format!("{}.pro", project_name));
    let mut pro_file = create_file(&pro_file_path)?;
    writeln!(pro_file, "TEMPLATE = lib")?;
    writeln!(pro_file, "CONFIG += staticlib")?;
    writeln!(
        pro_file,
        "RESOURCES += {}",
        paths.iter().map_if_ok(|path| path_to_str(path))?.join(" ")
    )?;
    drop(pro_file);
    run_command(Command::new("qmake").arg(pro_file_path).current_dir(&dir))?;
    let make_command = if target::current_env() == target::Env::Msvc {
        "nmake"
    } else {
        "make"
    };
    run_command(Command::new(make_command).current_dir(&dir))?;
    println!("cargo:rustc-link-lib=static={}", project_name);
    println!("cargo:rustc-link-search={}", path_to_str(&dir)?);
    Ok(())
}

/// Calls `try_add_resources` and panic on an error.
pub fn add_resources(paths: impl IntoIterator<Item = impl AsRef<Path>>) {
    try_add_resources(paths).fancy_unwrap();
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
