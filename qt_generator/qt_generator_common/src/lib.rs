//! Common utilities for the generator and the build script for Qt crates.
//!
//! Qt modules are identified within this crate using snake case names without
//! a prefix, e.g. `core` for QtCore and `ui_tools` for QtUiTools.
//! `sublib_name` argument should be in this form.
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust)
//! for more information.

extern crate cpp_to_rust_common;

use cpp_to_rust_common::utils::get_command_output;
use cpp_to_rust_common::file_utils::PathBufWithAdded;
use cpp_to_rust_common::string_utils::CaseOperations;
use cpp_to_rust_common::errors::Result;
use cpp_to_rust_common::log;

use std::path::PathBuf;
use std::process::Command;

/// Makes a query to `qmake`.
fn run_qmake_string_query(property: &str) -> Result<String> {
  let result = get_command_output(Command::new("qmake").arg("-query").arg(property))?;
  Ok(result.trim().to_string())
}


/// Makes a query to `qmake` and interprets its output as a path.
fn run_qmake_query(property: &str) -> Result<PathBuf> {
  Ok(PathBuf::from(run_qmake_string_query(property)?))
}

/// Properties of a Qt installation
pub struct InstallationData {
  /// Qt version.
  pub qt_version: String,
  /// Path to the parent include directory of the installation.
  pub root_include_path: PathBuf,
  /// Path to the include directory of the library that is being processed.
  /// This is a direct subdirectory of `root_include_path`.
  pub lib_include_path: PathBuf,
  /// Path to the directory containing library files for the linker.
  pub lib_path: PathBuf,
  /// Path to the directory containing Qt documentation files.
  pub docs_path: PathBuf,
  /// If true, this Qt library was built as a MacOS framework.
  pub is_framework: bool,
}

/// Detects properties of current Qt installation using `qmake` command line utility.
pub fn get_installation_data(sublib_name: &str) -> Result<InstallationData> {
  let qt_version = run_qmake_string_query("QT_VERSION")?;
  log::status(format!("QT_VERSION = \"{}\"", qt_version));
  log::status("Detecting Qt directories");

  let root_include_path = run_qmake_query("QT_INSTALL_HEADERS")?;
  log::status(format!("QT_INSTALL_HEADERS = \"{}\"", root_include_path.display()));
  let lib_path = run_qmake_query("QT_INSTALL_LIBS")?;
  log::status(format!("QT_INSTALL_LIBS = \"{}\"", lib_path.display()));
  let docs_path = run_qmake_query("QT_INSTALL_DOCS")?;
  log::status(format!("QT_INSTALL_DOCS = \"{}\"", docs_path.display()));
  let folder_name = lib_folder_name(sublib_name);
  let dir = root_include_path.with_added(&folder_name);
  if dir.exists() {
    Ok(InstallationData {
         root_include_path: root_include_path,
         lib_path: lib_path,
         docs_path: docs_path,
         lib_include_path: dir,
         is_framework: false,
         qt_version: qt_version,
       })
  } else {
    let dir2 = lib_path.with_added(format!("{}.framework/Headers", folder_name));
    if dir2.exists() {
      Ok(InstallationData {
           root_include_path: root_include_path,
           lib_path: lib_path,
           docs_path: docs_path,
           lib_include_path: dir2,
           is_framework: true,
           qt_version: qt_version,
         })
    } else {
      Err(format!("extra header dir not found (tried: {}, {})",
                  dir.display(),
                  dir2.display())
              .into())
    }
  }
}

/// Returns library name of the specified module as
/// should be passed to the linker, e.g. `"Qt5Core"`.
pub fn real_lib_name(sublib_name: &str) -> String {
  let sublib_name_capitalized = sublib_name.to_class_case();
  format!("Qt5{}", sublib_name_capitalized)
}

/// Returns name of the module's include directory, e.g. `"QtCore"`.
pub fn lib_folder_name(sublib_name: &str) -> String {
  let sublib_name_capitalized = sublib_name.to_class_case();
  format!("Qt{}", sublib_name_capitalized)
}

/// Returns MacOS framework name of the specified module as
/// should be passed to the linker, e.g. `"QtCore"`.
pub fn framework_name(sublib_name: &str) -> String {
  let sublib_name_capitalized = sublib_name.to_class_case();
  format!("Qt{}", sublib_name_capitalized)
}

/// Returns list of modules this module depends on.
pub fn lib_dependencies(sublib_name: &str) -> Result<&'static [&'static str]> {
  const CORE: &'static [&'static str] = &[];
  const GUI: &'static [&'static str] = &["core"];
  const WIDGETS: &'static [&'static str] = &["core", "gui"];
  const UI_TOOLS: &'static [&'static str] = &["core", "gui", "widgets"];
  Ok(match sublib_name {
       "core" => CORE,
       "gui" => GUI,
       "widgets" => WIDGETS,
       "ui_tools" => UI_TOOLS,
       _ => return Err(format!("Unknown lib name: {}", sublib_name).into()),
     })
}
