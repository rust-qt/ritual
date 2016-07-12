
mod cpp_ffi_function_argument;
mod cpp_ffi_function_signature;
mod cpp_ffi_generator;
mod cpp_code_generator;
mod caption_strategy;
mod cpp_data;
mod cpp_and_ffi_method;
mod cpp_method;
mod cpp_type;
mod cpp_ffi_type;
mod cpp_operators;
mod log;
mod qt_specific;
mod rust_generator;
mod rust_code_generator;
mod rust_info;
mod rust_type;
mod utils;
mod cpp_parser;
// mod doc_parser_support;

use std::path::PathBuf;
use std::env;
use std::process::Command;

extern crate find_folder;

fn print_usage() {
  log::error("Usage:");
  log::error("\tqt_wrapper_generator check_parsers_consistency parse_result_path");
  log::error("\tqt_wrapper_generator stage0 qtcw_path rust_qt_path");
  std::process::exit(1);
}

fn main() {
  let arguments: Vec<_> = env::args().collect();
  //  if arguments.len() == 3 && arguments[1] == "check_parsers_consistency" {
  //    let headers_dir = ....;
  //    let mut parser1 = cpp_parser::CppParser::new();
  //    parser1.run();
  //    let stats = parser1.get_stats();
  //    let mut parse_result1 = parser1.get_data();
  //    let parse_result_path = PathBuf::from(arguments[2].clone());
  //    log::info("Reading parse result...");
  //    let parse_result2 = doc_parser_support::read_parse_result::do_it(&parse_result_path);
  //    qt_specific::fix_header_names(&mut parse_result1, &headers_dir);
  //    doc_parser_support::parsers_consistency_checker::check(&parse_result1, &stats, &parse_result2);
  //    return;
  //  }
  if arguments.len() == 4 && arguments[1] == "stage0" {
    let qtcw_path = PathBuf::from(arguments[2].clone());
    let rust_qt_path = PathBuf::from(arguments[3].clone());

    let qt_install_headers_path = PathBuf::from(String::from_utf8(Command::new("qmake")
        .arg("-query")
        .arg("QT_INSTALL_HEADERS")
        .output()
        .expect("Failed to execute qmake query.")
        .stdout)
      .unwrap()
      .trim());
    let qt_install_libs_path = PathBuf::from(String::from_utf8(Command::new("qmake")
        .arg("-query")
        .arg("QT_INSTALL_LIBS")
        .output()
        .expect("Failed to execute qmake query.")
        .stdout)
      .unwrap()
      .trim());

    let mut qt_core_headers_path = qt_install_headers_path.clone();
    qt_core_headers_path.push("QtCore");

    log::info("Stage 1. Parsing Qt headers.");
    let mut parser = cpp_parser::CppParser::new(vec![qt_install_headers_path.clone(),
                                                     qt_core_headers_path.clone()],
                                                "QtCore".to_string(),
                                                rust_qt_path.clone());
    parser.run();
    let mut parse_result = parser.get_data();
    qt_specific::fix_header_names(&mut parse_result, &qt_core_headers_path);

    parse_result.ensure_explicit_destructors();


    let c_gen = cpp_ffi_generator::CGenerator::new(parse_result, qtcw_path);
    log::info("Stage 2. Generating QTCW (Qt C wrapper) library.");
    let c_data = c_gen.generate_all();
    let mut rust_gen = rust_generator::RustGenerator::new(c_data, rust_qt_path);
    log::info("Stage 3. Generating Rust Qt crates.");
    rust_gen.generate_all();
    log::info("Source files for QTCW and Rust Qt crates have been generated.");

    return;
  }
  print_usage();
}
