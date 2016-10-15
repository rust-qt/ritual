use cpp_data::CppData;
use errors::Result;
use file_utils::{PathBufWithAdded, load_json};
use rust_info::RustExportInfo;

use std::path::PathBuf;

extern crate serde_json;

pub struct DependencyInfo {
  pub cpp_data: CppData,
  pub rust_export_info: RustExportInfo,
  pub path: PathBuf,
}

impl DependencyInfo {
  pub fn load(path: &PathBuf) -> Result<DependencyInfo> {
    let cpp_data_path = path.with_added("cpp_data.json");
    if !cpp_data_path.exists() {
      return Err(format!("file not found: {}", cpp_data_path.display()).into());
    }
    let cpp_data = try!(load_json(&cpp_data_path));

    let rust_export_info_path = path.with_added("rust_export_info.json");
    if !rust_export_info_path.exists() {
      return Err(format!("file not found: {}", rust_export_info_path.display()).into());
    }
    let rust_export_info = try!(load_json(&rust_export_info_path));
    Ok(DependencyInfo {
      cpp_data: cpp_data,
      rust_export_info: rust_export_info,
      path: path.clone(),
    })
  }
}
