use doc_parser_support::cpp_header_data::CppHeaderData;
use doc_parser_support::cpp_type_map::CppTypeMap;

#[derive(Debug, Clone)]
pub struct DocCppData {
  pub headers: Vec<CppHeaderData>,
  pub types: CppTypeMap,
  pub classes_blacklist: Vec<String>,
}
