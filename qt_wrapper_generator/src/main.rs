
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
mod extractor_actions_generator;
mod read_extracted_info;
mod read_parse_result;
mod utils;

use std::path::{PathBuf};
use std::env;

extern crate find_folder;

fn print_usage() {
  println!("Usage:");
  println!("\tqt_wrapper_generator stage1 parse_result_path extractor_actions_path");
  println!("\tqt_wrapper_generator stage2 parse_result_path extracted_info_path qtcw_path rust_qt_path");
  std::process::exit(1);
}

fn main() {
  let arguments: Vec<_> = env::args().collect();
  if arguments.len() < 4 {
    print_usage();
    return;
  }
  let parse_result_path = PathBuf::from(arguments[2].clone());
  let mut parse_result = read_parse_result::do_it(&parse_result_path);
  for data in &mut parse_result.headers {
    data.ensure_explicit_destructor();
  }
  //TODO: unblock on Windows
  parse_result.classes_blacklist = vec!["QWinEventNotifier".to_string()];

  if arguments[1] == "stage1" {
    if arguments.len() != 4 {
      print_usage();
      return;
    }
    let extractor_actions_path = PathBuf::from(arguments[3].clone());
    extractor_actions_generator::do_it(parse_result, extractor_actions_path);
  } else if arguments[1] == "stage2" {
    if arguments.len() != 6 {
      print_usage();
      return;
    }
    let extracted_info_path = PathBuf::from(arguments[3].clone());
    let extracted_info = read_extracted_info::do_it(extracted_info_path);
    let qtcw_path = PathBuf::from(arguments[4].clone());
    let rust_qt_path = PathBuf::from(arguments[5].clone());
    let mut g = c_generator::CGenerator::new(parse_result, extracted_info, qtcw_path);
    g.generate_all();
  } else {
    print_usage();
    return;
  }
}
