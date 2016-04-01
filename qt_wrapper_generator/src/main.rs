
mod c_function_argument;
mod c_function_signature;
mod c_generator;
mod c_type;
mod caption_strategy;
mod cpp_and_c_method;
mod cpp_data;
mod cpp_header_data;
mod cpp_method;
mod cpp_type;
mod cpp_type_map;
mod enums;
mod read_parse_result;
mod utils;

use std::path::{PathBuf};
use std::env;

extern crate find_folder;

fn main() {
  let arguments: Vec<_> = env::args().collect();
  if arguments.len() != 4 {
    print!("Usage: qt_wrapper_generator parse_result_path qtcw_path rust_qt_path");
  }

  let parse_result_path = PathBuf::from(arguments[1].clone());
  let qtcw_path = PathBuf::from(arguments[2].clone());
  let rust_qt_path = PathBuf::from(arguments[3].clone());

  let mut parse_result = read_parse_result::do_it(&parse_result_path);
  for data in &mut parse_result.headers {
    data.ensure_explicit_destructor();
  }

  let mut g = c_generator::CGenerator::new(parse_result, qtcw_path);
  g.generate_all();

}
