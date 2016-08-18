extern crate serde_json;

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

#[cfg(test)]
mod tests;

use std::fs;
use std::fs::File;
use utils::PathBufPushTweak;


// mod doc_parser_support;

use std::path::PathBuf;
use std::env;
use std::process::Command;
use cpp_code_generator::CppCodeGenerator;

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


use serializable::{LibSpec, LocalOverrides};

fn run_command(command: &mut Command, fetch_stdout: bool) -> String {
  log::info(format!("Executing command: {:?}", command));
  match command.status() {
    Ok(status) => {
      if !status.success() {
        log::error(format!("Command failed: {:?} (status: {})", command, status));
        std::process::exit(1);
      }
      if fetch_stdout {
        match command.output() {
          Ok(output) => return String::from_utf8(output.stdout).unwrap(),
          Err(error) => {
            log::error(format!("Fetching output failed: {}", error));
            std::process::exit(1);
          }
        }
      } else {
        return String::new();
      }
    }
    Err(error) => {
      log::error(format!("Execution failed: {}", error));
      std::process::exit(1);
    }
  }
}


fn main() {
  let arguments: Vec<_> = env::args().collect();
  if arguments.len() != 3 {
    print_usage();
    return;
  }
  log::info("Reading lib spec...");
  let lib_spec_path = PathBuf::from(arguments[1].clone());
  let mut output_dir_path = PathBuf::from(arguments[2].clone());
  let current_dir = std::env::current_dir().unwrap();
  if output_dir_path.is_relative() {
    output_dir_path = current_dir.with_added(&output_dir_path);
  }
  if !output_dir_path.as_path().exists() {
    fs::create_dir(&output_dir_path).unwrap();
  }
  output_dir_path = fs::canonicalize(&output_dir_path).unwrap();
  let mut lib_spec_dir_path = lib_spec_path.clone();
  assert!(lib_spec_dir_path.pop());
  if lib_spec_dir_path.is_relative() {
    lib_spec_dir_path = current_dir.with_added(&lib_spec_dir_path);
  }
  lib_spec_dir_path = fs::canonicalize(&lib_spec_dir_path).unwrap();

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

  let is_qt_library = lib_spec.cpp.name.starts_with("Qt5");

  let mut include_dirs = Vec::new();
  let mut cpp_lib_path = None;
  let mut qt_this_lib_headers_dir = None;
  if is_qt_library {

    let qmake_path = match local_overrides.qmake_path {
      Some(path) => path.clone(),
      None => "qmake".to_string(),
    };
    log::info(format!("Using qmake path: {}", qmake_path));
    log::info("Detecting Qt directories...");
    let qt_install_headers_path = PathBuf::from(run_command(Command::new(&qmake_path)
                                                              .arg("-query")
                                                              .arg("QT_INSTALL_HEADERS"),
                                                            true)
      .trim());
    log::info(format!("QT_INSTALL_HEADERS = \"{}\"",
                      qt_install_headers_path.to_str().unwrap()));
    let qt_install_libs_path = PathBuf::from(run_command(Command::new(&qmake_path)
                                                           .arg("-query")
                                                           .arg("QT_INSTALL_LIBS"),
                                                         true)
      .trim());
    log::info(format!("QT_INSTALL_LIBS = \"{}\"",
                      qt_install_libs_path.to_str().unwrap()));
    cpp_lib_path = Some(qt_install_libs_path);
    include_dirs.push(qt_install_headers_path.clone());
    if &lib_spec.cpp.name == "Qt5Core" {
      let dir = qt_install_headers_path.with_added("QtCore");
      qt_this_lib_headers_dir = Some(dir.clone());
      include_dirs.push(dir);
    } else {
      log::warning("This library is not supported yet.");
    }
  }

  let parse_result_cache_file_path = {
    let mut p = output_dir_path.clone();
    p.push("cpp_data.json");
    p
  };
  let mut parse_result = if parse_result_cache_file_path.as_path().is_file() {
    log::info(format!("C++ data is loaded from file: {}",
                      parse_result_cache_file_path.to_str().unwrap()));
    let file = File::open(&parse_result_cache_file_path).unwrap();
    serde_json::from_reader(file).unwrap()
  } else {
    log::info("Parsing C++ headers.");
    let mut parser = cpp_parser::CppParser::new(include_dirs.clone(),
                                                lib_spec.cpp.include_file.clone(),
                                                output_dir_path.clone(),
                                                lib_spec.cpp.name_blacklist.clone());
    parser.run();
    let parse_result = parser.get_data();

    // let serialized_parse_result = serde_json::to_vec(&parse_result).unwrap();
    let mut file = File::create(&parse_result_cache_file_path).unwrap();
    // file.write(serialized_parse_result);
    serde_json::to_writer(&mut file, &parse_result).unwrap();
    log::info(format!("Header parse result is saved to file: {}",
                      parse_result_cache_file_path.to_str().unwrap()));
    parse_result
  };
  log::info("Post-processing parse result.");
  if is_qt_library {
    qt_specific::fix_header_names(&mut parse_result, &qt_this_lib_headers_dir.unwrap());
  }
  parse_result.post_process();

  let crate_path = output_dir_path.with_added(&lib_spec.rust.name);

  let c_lib_name = format!("{}_c", &lib_spec.rust.name);
  let c_lib_parent_path = crate_path.with_added("c_lib");
  let c_lib_path = c_lib_parent_path.with_added("source");
  let c_lib_tmp_path = c_lib_parent_path.with_added("source.new");
  if c_lib_tmp_path.as_path().exists() {
    fs::remove_dir_all(&c_lib_tmp_path).unwrap();
  }
  fs::create_dir_all(&c_lib_tmp_path).unwrap();
  log::info(format!("Generating C wrapper library ({}).", c_lib_name));
  let code_gen = CppCodeGenerator::new(c_lib_name.clone(), c_lib_tmp_path.clone());
  code_gen.generate_template_files(&lib_spec.cpp.include_file,
                                   &include_dirs.iter()
                                     .map(|x| x.to_str().unwrap().to_string())
                                     .collect());

  let c_gen =
    cpp_ffi_generator::CGenerator::new(parse_result, c_lib_name.clone(), c_lib_tmp_path.clone());
  let c_data = c_gen.generate_all();
  utils::move_files(&c_lib_tmp_path, &c_lib_path, Vec::new()).unwrap();

  log::info(format!("Building C wrapper library."));
  let c_lib_build_path = c_lib_parent_path.with_added("build");
  fs::create_dir_all(&c_lib_build_path).unwrap();
  let c_lib_install_path = c_lib_parent_path.with_added("install");
  fs::create_dir_all(&c_lib_install_path).unwrap();

  run_command(Command::new("cmake")
                .arg(&c_lib_path)
                .arg(format!("-DCMAKE_INSTALL_PREFIX={}",
                             c_lib_install_path.to_str().unwrap()))
                .current_dir(&c_lib_build_path),
              false);

  let make_command = match local_overrides.make_command {
    Some(cmd) => cmd,
    None => "make".to_string(),
  };
  let mut make_args = match local_overrides.make_arguments {
    Some(args) => args,
    None => Vec::new(),
  };
  make_args.push("install".to_string());
  run_command(Command::new(make_command)
                .args(&make_args)
                .current_dir(&c_lib_build_path),
              false);


  let crate_new_path = output_dir_path.with_added(format!("{}.new", &lib_spec.rust.name));
  if crate_new_path.as_path().exists() {
    fs::remove_dir_all(&crate_new_path).unwrap();
  }
  fs::create_dir_all(&crate_new_path).unwrap();
  let rustfmt_config_path = lib_spec_dir_path.with_added("rustfmt.toml");
  let rust_config = rust_code_generator::RustCodeGeneratorConfig {
    crate_name: lib_spec.rust.name.clone(),
    crate_authors: lib_spec.rust.authors.clone(),
    crate_version: lib_spec.rust.version.clone(),
    output_path: crate_new_path.clone(),
    template_path: lib_spec_dir_path.with_added("crate"),
    c_lib_name: c_lib_name,
    cpp_lib_name: lib_spec.cpp.name.clone(),
    rustfmt_config_path: if rustfmt_config_path.as_path().exists() {
      Some(rustfmt_config_path)
    } else {
      None
    },
    remove_qt_prefix: is_qt_library,
    module_blacklist: lib_spec.rust.module_blacklist,
  };
  let mut rust_gen = rust_generator::RustGenerator::new(c_data, rust_config);

  log::info(format!("Generating Rust crate ({}).", &lib_spec.rust.name));
  rust_gen.generate_all();
  utils::move_files(&crate_new_path,
                    &crate_path,
                    vec!["c_lib".to_string(), "target".to_string()])
    .unwrap();

  log::info(format!("Compiling Rust crate."));
  for cargo_cmd in vec!["test", "doc"] {
    let mut command = Command::new("cargo");
    command.arg(cargo_cmd)
      .current_dir(&crate_path);
    if let Some(ref cpp_lib_path) = cpp_lib_path {
      command.env("LIBRARY_PATH", cpp_lib_path.to_str().unwrap())
        .env("LD_LIBRARY_PATH", cpp_lib_path.to_str().unwrap());
    }
    run_command(&mut command, false);
  }
  log::info("Completed successfully.");
}
