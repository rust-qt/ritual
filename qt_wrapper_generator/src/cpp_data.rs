use cpp_header_data::CppHeaderData;
use cpp_type_map::CppTypeMap;

#[derive(Debug)]
pub struct CppData {
  pub headers: Vec<CppHeaderData>,
  pub types: CppTypeMap,
}