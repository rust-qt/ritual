use c_generator::CppAndCData;
use std::path::PathBuf;

pub struct RustGenerator {
  input_data: CppAndCData,
  output_path: PathBuf,
}

impl RustGenerator {
  pub fn new(input_data: CppAndCData, output_path: PathBuf) -> Self {
    RustGenerator {
      input_data: input_data,
      output_path: output_path,
    }

  }
}
