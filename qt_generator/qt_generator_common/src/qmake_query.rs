use cpp_to_rust_common::utils::get_command_output;
use cpp_to_rust_common::file_utils::PathBufWithAdded;
use cpp_to_rust_common::string_utils::CaseOperations;
use cpp_to_rust_common::errors::Result;
use cpp_to_rust_common::log;

use std::path::PathBuf;
use std::process::Command;

pub fn run_qmake_query(arg: &str) -> Result<PathBuf> {
  let result = get_command_output(Command::new("qmake").arg("-query").arg(arg))?;
  Ok(PathBuf::from(result.trim()))
}


pub struct InstallationData {
  pub root_include_path: PathBuf,
  pub lib_include_path: PathBuf,
  pub lib_path: PathBuf,
  pub is_framework: bool,
}

pub fn get_installation_data(sublib_name: &str) -> Result<InstallationData> {
  log::info("Detecting Qt directories...");

  let root_include_path = run_qmake_query("QT_INSTALL_HEADERS")?;
  log::info(format!("QT_INSTALL_HEADERS = \"{}\"", root_include_path.display()));
  let lib_path = run_qmake_query("QT_INSTALL_LIBS")?;
  log::info(format!("QT_INSTALL_LIBS = \"{}\"", lib_path.display()));
  let folder_name = lib_folder_name(sublib_name);
  let dir = root_include_path.with_added(&folder_name);
  if dir.exists() {
    Ok(InstallationData {
      root_include_path: root_include_path,
      lib_path: lib_path,
      lib_include_path: dir,
      is_framework: false,
    })
  } else {
    let dir2 = lib_path.with_added(format!("{}.framework/Headers", folder_name));
    if dir2.exists() {
      Ok(InstallationData {
        root_include_path: root_include_path,
        lib_path: lib_path,
        lib_include_path: dir2,
        is_framework: true,
      })
    } else {
      Err(format!("extra header dir not found (tried: {}, {})",
                  dir.display(),
                  dir2.display())
        .into())
    }
  }
}

pub fn real_lib_name(sublib_name: &str) -> String {
  let sublib_name_capitalized = sublib_name.to_class_case();
  format!("Qt5{}", sublib_name_capitalized)
}

pub fn lib_folder_name(sublib_name: &str) -> String {
  let sublib_name_capitalized = sublib_name.to_class_case();
  format!("Qt{}", sublib_name_capitalized)
}

pub fn framework_name(sublib_name: &str) -> String {
  let sublib_name_capitalized = sublib_name.to_class_case();
  format!("Qt{}", sublib_name_capitalized)
}
