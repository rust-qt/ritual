use cpp_data::CppData;
use rust_info::RustExportInfo;
use std::path::PathBuf;
use std::fs::File;

use utils::PathBufPushTweak;

extern crate serde_json;

pub struct DependencyInfo {
  pub cpp_data: CppData,
  pub rust_export_info: RustExportInfo,
  pub path: PathBuf,
}

impl DependencyInfo {
  pub fn load(path: &PathBuf) -> DependencyInfo {
    let cpp_data_path = path.with_added("cpp_data.json");
    if !cpp_data_path.exists() {
      panic!("Invalid dependency: file not found: {}",
             cpp_data_path.display());
    }
    let file = match File::open(&cpp_data_path) {
      Ok(r) => r,
      Err(_) => {
        panic!("Invalid dependency: failed to open file: {}",
               cpp_data_path.display())
      }
    };
    let cpp_data = match serde_json::from_reader(file) {
      Ok(r) => r,
      Err(err) => {
        panic!("Invalid dependency: failed to parse file: {}: {}",
               cpp_data_path.display(),
               err)
      }
    };

    let rust_export_info_path = path.with_added("rust_export_info.json");
    if !rust_export_info_path.exists() {
      panic!("Invalid dependency: file not found: {}",
             rust_export_info_path.display());
    }
    let file2 = match File::open(&rust_export_info_path) {
      Ok(r) => r,
      Err(_) => {
        panic!("Invalid dependency: failed to open file: {}",
               rust_export_info_path.display())
      }
    };
    let rust_export_info = match serde_json::from_reader(file2) {
      Ok(r) => r,
      Err(err) => {
        panic!("Invalid dependency: failed to parse file: {}: {}",
               rust_export_info_path.display(),
               err)
      }
    };
    DependencyInfo {
      cpp_data: cpp_data,
      rust_export_info: rust_export_info,
      path: path.clone(),
    }

  }
}
