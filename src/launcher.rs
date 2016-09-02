extern crate serde_json;
extern crate num_cpus;

use std;
use std::fs;
use std::fs::File;
use utils::PathBufPushTweak;

use std::path::PathBuf;
use std::process::Command;
use cpp_code_generator::CppCodeGenerator;
use log;
use cpp_parser;
use qt_specific;
use utils;
use cpp_ffi_generator;
use rust_code_generator;
use rust_generator;
use serializable::LibSpec;
use cpp_ffi_generator::CppAndFfiData;

/// Runs a command, checks that it is successful, and
/// returns its output if requested
fn run_command(command: &mut Command, fetch_stdout: bool) -> String {
  log::info(format!("Executing command: {:?}", command));
  if fetch_stdout {
    match command.output() {
      Ok(output) => {
        match command.status() {
          Ok(status) => {
            if !status.success() {
              panic!("Command failed: {:?} (status: {})", command, status);
            }
          }
          Err(error) => {
            panic!("Execution failed: {}", error);
          }
        }
        String::from_utf8(output.stdout).unwrap()
      }
      Err(error) => {
        panic!("Execution failed: {}", error);
      }
    }
  } else {
    match command.status() {
      Ok(status) => {
        if !status.success() {
          panic!("Command failed: {:?} (status: {})", command, status);
        }
      }
      Err(error) => {
        panic!("Execution failed: {}", error);
      }
    }
    String::new()
  }
}

pub enum BuildProfile {
  Debug,
  Release,
}

pub use rust_code_generator::InvokationMethod;

pub struct BuildEnvironment {
  pub invokation_method: InvokationMethod,
  pub output_dir_path: PathBuf,
  pub source_dir_path: PathBuf,
  pub num_jobs: Option<i32>,
  pub build_profile: BuildProfile,
}

pub fn run(env: BuildEnvironment) {
  // canonicalize paths
  let current_dir = std::env::current_dir().unwrap();
  let mut output_dir_path = if env.output_dir_path.is_relative() {
    current_dir.with_added(&env.output_dir_path)
  } else {
    env.output_dir_path
  };
  if !output_dir_path.as_path().exists() {
    fs::create_dir(&output_dir_path).unwrap();
  }
  output_dir_path = fs::canonicalize(&output_dir_path).unwrap();
  let mut source_dir_path = if env.source_dir_path.is_relative() {
    current_dir.with_added(&env.source_dir_path)
  } else {
    env.source_dir_path
  };
  source_dir_path = fs::canonicalize(&source_dir_path).unwrap();

  let lib_spec_path = source_dir_path.with_added("spec.json");

  log::info("Reading lib spec...");
  let file = File::open(&lib_spec_path).unwrap();
  let lib_spec: LibSpec = serde_json::from_reader(file).unwrap();
  log::info("Lib spec is valid.");
  log::info(format!("C++ library name: {}", lib_spec.cpp.name));

  let is_qt_library = lib_spec.cpp.name.starts_with("Qt5");

  let mut include_dirs = Vec::new();
  let mut cpp_lib_path = None;
  let mut qt_this_lib_headers_dir = None;
  if is_qt_library {

    let qmake_path = "qmake".to_string();
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
  let c_lib_parent_path = output_dir_path.with_added("c_lib");
  let c_lib_install_path = c_lib_parent_path.with_added("install");
  let num_jobs = env.num_jobs.unwrap_or_else(|| num_cpus::get() as i32);
  if output_dir_path.with_added("skip_processing").as_path().exists() {
    log::info("Processing skipped!");
  } else {
    let parse_result_cache_file_path = output_dir_path.with_added("cpp_data.json");
    let mut parse_result = if parse_result_cache_file_path.as_path().is_file() {
      log::info(format!("C++ data is loaded from file: {}",
                        parse_result_cache_file_path.to_str().unwrap()));
      let file = File::open(&parse_result_cache_file_path).unwrap();
      serde_json::from_reader(file).unwrap()
    } else {
      log::info("Parsing C++ headers.");
      let parse_result = cpp_parser::run(cpp_parser::CppParserConfig {
        include_dirs: include_dirs.clone(),
        header_name: lib_spec.cpp.include_file.clone(),
        tmp_cpp_path: output_dir_path.with_added("1.cpp"),
        name_blacklist: lib_spec.cpp.name_blacklist.clone(),
      });

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

    let c_lib_name = format!("{}_c", &lib_spec.rust.name);
    let c_lib_path = c_lib_parent_path.with_added("source");
    let c_lib_tmp_path = c_lib_parent_path.with_added("source.new");
    if c_lib_tmp_path.as_path().exists() {
      fs::remove_dir_all(&c_lib_tmp_path).unwrap();
    }
    fs::create_dir_all(&c_lib_tmp_path).unwrap();
    log::info(format!("Generating C wrapper library ({}).", c_lib_name));

    let cpp_ffi_headers = cpp_ffi_generator::run(&parse_result, lib_spec.cpp.clone());

    let code_gen = CppCodeGenerator::new(c_lib_name.clone(), c_lib_tmp_path.clone());
    code_gen.generate_template_files(&lib_spec.cpp.include_file,
                                     &include_dirs.iter()
                                       .map(|x| x.to_str().unwrap().to_string())
                                       .collect());
    code_gen.generate_files(&cpp_ffi_headers);

    utils::move_files(&c_lib_tmp_path, &c_lib_path).unwrap();

    log::info(format!("Building C wrapper library."));
    let c_lib_build_path = c_lib_parent_path.with_added("build");
    fs::create_dir_all(&c_lib_build_path).unwrap();
    fs::create_dir_all(&c_lib_install_path).unwrap();

    run_command(Command::new("cmake")
                  .arg(&c_lib_path)
                  .arg(format!("-DCMAKE_INSTALL_PREFIX={}",
                               c_lib_install_path.to_str().unwrap()))
                  .current_dir(&c_lib_build_path),
                false);

    let make_command = "make".to_string();
    let mut make_args = Vec::new();
    make_args.push(format!("-j{}", num_jobs));
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
    let rustfmt_config_path = source_dir_path.with_added("rustfmt.toml");
    let rust_config = rust_code_generator::RustCodeGeneratorConfig {
      invokation_method: env.invokation_method.clone(),
      crate_name: lib_spec.rust.name.clone(),
      crate_authors: lib_spec.rust.authors.clone(),
      crate_version: lib_spec.rust.version.clone(),
      output_path: crate_new_path.clone(),
      template_path: source_dir_path.clone(),
      c_lib_name: c_lib_name,
      cpp_lib_name: lib_spec.cpp.name.clone(),
      rustfmt_config_path: if rustfmt_config_path.as_path().exists() {
        Some(rustfmt_config_path)
      } else {
        None
      },
    };
    log::info(format!("Generating Rust crate ({}).", &lib_spec.rust.name));
    let rust_data = rust_generator::run(CppAndFfiData {
                                          cpp_data: parse_result,
                                          cpp_ffi_headers: cpp_ffi_headers,
                                        },
                                        rust_generator::RustGeneratorConfig {
                                          crate_name: lib_spec.rust.name.clone(),
                                          remove_qt_prefix: is_qt_library,
                                          module_blacklist: lib_spec.rust.module_blacklist,
                                        });
    rust_code_generator::run(rust_config, &rust_data);

    for item in fs::read_dir(&crate_new_path).unwrap() {
      let item = item.unwrap();
      utils::move_files(&crate_new_path.with_added(item.file_name()),
                        &output_dir_path.with_added(item.file_name()))
        .unwrap();
    }
    fs::remove_dir(&crate_new_path).unwrap();
  }


  match env.invokation_method {
    InvokationMethod::CommandLine => {
      log::info(format!("Compiling Rust crate."));
      for cargo_cmd in vec!["test", "doc"] {
        let mut command = Command::new("cargo");
        command.arg(cargo_cmd);
        command.arg(format!("-j{}", num_jobs));
        command.current_dir(&output_dir_path);
        if let Some(ref cpp_lib_path) = cpp_lib_path {
          let lib_path = cpp_lib_path.to_str().unwrap();
          command.env("LIBRARY_PATH", lib_path)
            .env("LD_LIBRARY_PATH", lib_path);
        }
        run_command(&mut command, false);
      }
      log::info("Completed successfully.");
    }
    InvokationMethod::BuildScript => {
      println!("cargo:rustc-link-search={}",
               c_lib_install_path.with_added("lib").to_str().unwrap());
      if let Some(ref cpp_lib_path) = cpp_lib_path {
        let lib_path = cpp_lib_path.to_str().unwrap();
        println!("cargo:rustc-link-search=native={}", lib_path);
      }
    }
  }
}
