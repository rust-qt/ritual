mod cpp_ffi_generator;
mod cpp_code_generator;
mod caption_strategy;
mod cpp_data;
mod cpp_ffi_data;
mod cpp_method;
mod cpp_type;
mod cpp_operator;
mod log;
mod qt_specific;
mod rust_generator;
mod rust_code_generator;
mod rust_info;
mod rust_type;
mod utils;
mod cpp_parser;
mod serializable;
mod launcher;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

fn print_usage() {
  log::error("Usage:");
  log::error("\tcargo run lib_spec_file output_dir");
}

fn main() {
  let arguments: Vec<_> = std::env::args().collect();
  if arguments.len() != 3 {
    print_usage();
    return;
  }
  let lib_spec_path = PathBuf::from(&arguments[1]);
  let output_dir_path = PathBuf::from(&arguments[2]);
  launcher::run(lib_spec_path, output_dir_path);
}
