
mod c_function_argument;
mod c_function_signature;
mod c_generator;
mod c_type;
mod caption_strategy;
mod clang_cpp_data;
mod cpp_and_c_method;
mod cpp_data;
mod cpp_header_data;
mod cpp_method;
mod cpp_type;
mod cpp_type_map;
mod enums;
mod extractor_actions_generator;
mod log;
mod qt_specific;
mod parsers_consistency_checker;
mod read_extracted_info;
mod read_parse_result;
mod rust_generator;
mod rust_type;
mod utils;
mod cpp_parser;

use std::path::PathBuf;
use std::env;

extern crate find_folder;

fn print_usage() {
  log::error("Usage:");
  log::error("\tqt_wrapper_generator stage1 parse_result_path extractor_actions_path");
  log::error("\tqt_wrapper_generator stage2 parse_result_path extracted_info_path qtcw_path \
              rust_qt_path");
  std::process::exit(1);
}

fn main() {
  let arguments: Vec<_> = env::args().collect();
  if arguments.len() == 3 && arguments[1] == "cpp_parser" {
    let headers_dir = PathBuf::from("/home/ri/bin/Qt/5.5/gcc_64/include/QtCore");
    // qt_specific::fix_header_names(&headers_dir);
    let mut parser1 = cpp_parser::CppParser::new();
    parser1.run();
    let mut parse_result1 = parser1.get_data();
    let parse_result_path = PathBuf::from(arguments[2].clone());
    log::info("Reading parse result...");
    let parse_result2 = read_parse_result::do_it(&parse_result_path);
    qt_specific::fix_header_names(&mut parse_result1, &headers_dir);
    parsers_consistency_checker::check(&parse_result1, &parse_result2);
    return;
  }
  if arguments.len() < 4 {
    print_usage();
    return;
  }
  let parse_result_path = PathBuf::from(arguments[2].clone());
  log::info("Reading parse result...");
  let mut parse_result = read_parse_result::do_it(&parse_result_path);
  for data in &mut parse_result.headers {
    data.ensure_explicit_destructor();
  }
  // TODO: unblock on Windows
  parse_result.classes_blacklist = vec!["QWinEventNotifier".to_string()];

  if arguments[1] == "stage1" {
    if arguments.len() != 4 {
      print_usage();
      return;
    }
    let extractor_actions_path = PathBuf::from(arguments[3].clone());
    log::info("Stage 1. Generating C++ data extraction request.");
    extractor_actions_generator::do_it(parse_result, extractor_actions_path);
    log::info("Done. C++ data extractor will be executed shortly.");
  } else if arguments[1] == "stage2" {
    if arguments.len() != 6 {
      print_usage();
      return;
    }
    let extracted_info_path = PathBuf::from(arguments[3].clone());
    log::info("Reading C++ extraction result...");
    let extracted_info = read_extracted_info::do_it(extracted_info_path);
    let qtcw_path = PathBuf::from(arguments[4].clone());
    let rust_qt_path = PathBuf::from(arguments[5].clone());
    let c_gen = c_generator::CGenerator::new(parse_result, extracted_info, qtcw_path);
    log::info("Stage 2. Generating QTCW (Qt C wrapper) library.");
    let c_data = c_gen.generate_all();
    let mut rust_gen = rust_generator::RustGenerator::new(c_data, rust_qt_path);
    log::info("Stage 3. Generating Rust Qt crates.");
    rust_gen.generate_all();
    log::info("Source files for QTCW and Rust Qt crates have been generated.");
  } else {
    print_usage();
    return;
  }
}
