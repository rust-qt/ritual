use cpp_header_data::CppHeaderData;
use cpp_type_map::CppTypeMap;

#[derive(Debug, Clone)]
pub struct CppData {
  pub headers: Vec<CppHeaderData>,
  pub types: CppTypeMap,
  pub classes_blacklist: Vec<String>,
}
