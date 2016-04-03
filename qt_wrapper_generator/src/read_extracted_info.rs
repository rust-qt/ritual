use std::path::PathBuf;

pub struct CppExtractedInfo {
  nothing: String,
}

pub fn do_it(file_name: PathBuf) -> CppExtractedInfo {
  CppExtractedInfo { nothing: "...".to_string() }
}
