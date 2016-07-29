#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate serde_json;
// extern crate toml;

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

use std::fs;
use std::fs::File;
use std::io::{Read, Write};


// mod doc_parser_support;

use std::path::PathBuf;
use std::env;
use std::process::Command;
use cpp_code_generator::CppCodeGenerator;

extern crate find_folder;

fn print_usage() {
  log::error("Usage:");
  log::error("\tcargo run lib_spec_file output_dir");
  std::process::exit(1);
}

// fn read_toml(path: &PathBuf) -> toml::Table {
//  match File::open(path) {
//    Ok(mut file) => {
//      let mut string = String::new();
//      file.read_to_string(&mut string).unwrap();
//      let mut parser = toml::Parser::new(&string);
//      match parser.parse() {
//        Some(value) => value,
//        None => {
//          panic!("Failed to parse {}: {:?}",
//                 path.to_str().unwrap(),
//                 parser.errors);
//        }
//      }
//    }
//    Err(err) => panic!("Failed to open file: {}", path.to_str().unwrap()),
//  }
// }


#[derive(Debug)]
#[derive(Serialize, Deserialize)]
struct CppLibSpec {
  name: String,
  include_file: String,
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
struct RustLibSpec {
  name: String,
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
struct LibSpec {
  cpp: CppLibSpec,
  rust: RustLibSpec,
}

#[derive(Debug, Default)]
#[derive(Serialize, Deserialize)]
struct LocalOverrides {
  qmake_path: Option<String>,
}

fn main() {
  let arguments: Vec<_> = env::args().collect();
  if arguments.len() != 3 {
    print_usage();
    return;
  }
  log::info("Reading lib spec...");
  let lib_spec_path = PathBuf::from(arguments[1].clone());
  let output_dir_path = PathBuf::from(arguments[2].clone());
  let file = File::open(&lib_spec_path).unwrap();
  let lib_spec: LibSpec = serde_json::from_reader(file).unwrap();
  log::info("Lib spec is valid.");
  log::info(format!("C++ library name: {}", lib_spec.cpp.name));

  let local_overrides_path = {
    let mut p = output_dir_path.clone();
    p.push("local_overrides.json");
    p
  };
  let local_overrides = if local_overrides_path.as_path().is_file() {
    log::info(format!("Loading local overrides file: {}",
                      local_overrides_path.to_str().unwrap()));
    let file = File::open(&local_overrides_path).unwrap();
    serde_json::from_reader(file).unwrap()
  } else {
    let r = LocalOverrides::default();
    let mut file = File::create(&local_overrides_path).unwrap();
    serde_json::to_writer(&mut file, &r).unwrap();
    log::info(format!("Local overrides file created: {}",
                      local_overrides_path.to_str().unwrap()));
    r
  };
  let qmake_path = match local_overrides.qmake_path {
    Some(path) => path.clone(),
    None => "qmake".to_string(),
  };
  log::info(format!("Using qmake path: {}", qmake_path));
  log::info("Detecting Qt directories...");
  let qt_install_headers_path = PathBuf::from(String::from_utf8(Command::new(&qmake_path)
                                                                  .arg("-query")
                                                                  .arg("QT_INSTALL_HEADERS")
                                                                  .output()
                                                                  .expect("Failed to execute \
                                                                           qmake query.")
                                                                  .stdout)
                                                .unwrap()
                                                .trim());
  log::info(format!("QT_INSTALL_HEADERS = \"{}\"",
                    qt_install_headers_path.to_str().unwrap()));
  let qt_install_libs_path = PathBuf::from(String::from_utf8(Command::new(&qmake_path)
                                                               .arg("-query")
                                                               .arg("QT_INSTALL_LIBS")
                                                               .output()
                                                               .expect("Failed to execute \
                                                                        qmake query.")
                                                               .stdout)
                                             .unwrap()
                                             .trim());
  log::info(format!("QT_INSTALL_LIBS = \"{}\"",
                    qt_install_libs_path.to_str().unwrap()));
  let qt_core_headers_path = {
    let mut p = qt_install_headers_path.clone();
    p.push("QtCore");
    p
  };
  let include_dirs = vec![qt_install_headers_path.clone(), qt_core_headers_path.clone()];

  let parse_result_cache_file_path = {
    let mut p = output_dir_path.clone();
    p.push("cpp_data.json");
    p
  };
  let parse_result = if parse_result_cache_file_path.as_path().is_file() {
    log::info(format!("Cpp data is loaded from file: {}",
                      parse_result_cache_file_path.to_str().unwrap()));
    let file = File::open(&parse_result_cache_file_path).unwrap();
    serde_json::from_reader(file).unwrap()
  } else {
    log::info("Parsing Qt headers.");
    let mut parser = cpp_parser::CppParser::new(include_dirs.clone(),
                                                lib_spec.cpp.include_file.clone(),
                                                output_dir_path.clone());
    parser.run();
    let mut parse_result = parser.get_data();
    qt_specific::fix_header_names(&mut parse_result, &qt_core_headers_path);

    parse_result.ensure_explicit_destructors();

    // let serialized_parse_result = serde_json::to_vec(&parse_result).unwrap();
    let mut file = File::create(&parse_result_cache_file_path).unwrap();
    // file.write(serialized_parse_result);
    serde_json::to_writer(&mut file, &parse_result).unwrap();
    log::info(format!("Header parse result is saved to file: {}",
                      parse_result_cache_file_path.to_str().unwrap()));
    parse_result
  };

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

  let c_lib_name = format!("{}_c_lib", &lib_spec.rust.name);
  let c_lib_path = {
    let mut p = output_dir_path.clone();
    p.push(&c_lib_name);
    p
  };
  if c_lib_path.as_path().is_dir() {
    log::info(format!("Skipping C library generation because directory already exists: {}",
                      c_lib_path.to_str().unwrap()));
  } else {
    fs::create_dir_all(&c_lib_path).unwrap();

    let code_gen = CppCodeGenerator::new(c_lib_name.clone(), c_lib_path.clone());
    code_gen.generate_template_files(&vec![lib_spec.cpp.name.clone()],
                                     &lib_spec.cpp.include_file,
                                     &include_dirs.iter()
                                                  .map(|x| x.to_str().unwrap().to_string())
                                                  .collect());

    let c_gen = cpp_ffi_generator::CGenerator::new(parse_result,
                                                   c_lib_name.clone(),
                                                   lib_spec.cpp.name.clone(),
                                                   c_lib_path);
    log::info(format!("Generating C wrapper library ({}).", c_lib_name));
    let c_data = c_gen.generate_all();
  }

  //  let crate_path = {
  //    let mut p = output_dir_path.clone(); p.push(&lib_spec.rust.name); p
  //  };
  //  let mut rust_gen = rust_generator::RustGenerator::new(c_data, crate_path);
  //  log::info(format!("Generating Rust crate ({}).", &lib_spec.rust.name));
  //  rust_gen.generate_all();
  //
  //  log::info(format!("Source files for C library and Rust crate have been generated."));

  return;
}
