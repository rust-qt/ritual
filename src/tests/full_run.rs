use file_utils::{PathBufWithAdded, create_dir, create_file};
use utils::{manifest_dir, run_command, add_env_path_item};
use cpp_lib_builder::CppLibBuilder;

use std::process::Command;

extern crate tempdir;

fn build_cpp_lib() -> tempdir::TempDir {
  let cpp_lib_source_dir = {
    let mut path = manifest_dir();
    path.push("test_assets");
    path.push("ctrt1");
    path.push("cpp");
    path
  };
  assert!(cpp_lib_source_dir.exists());
  let temp_dir = tempdir::TempDir::new("test_full_run").unwrap();
  let build_dir = temp_dir.path().with_added("build");
  let install_dir = temp_dir.path().with_added("install");
  create_dir(&build_dir).unwrap();
  create_dir(&install_dir).unwrap();
  CppLibBuilder {
      cmake_source_dir: &cpp_lib_source_dir,
      build_dir: &build_dir,
      install_dir: &install_dir,
      num_jobs: 1,
      linker_env_library_dirs: None,
      pipe_output: true,
    }
    .run()
    .unwrap_or_else(|e| {
      e.display_report();
      panic!("{}", e);
    });
  temp_dir
}

#[test]
fn only_cpp_lib() {
  build_cpp_lib();
}

extern crate toml;

#[test]
fn full_run() {
  let temp_dir = build_cpp_lib();
  // let output_dir = temp_dir.path().with_added("rust");
  // TODO: maybe override output dir to avoid conflicts
  let cpp_install_lib_dir = temp_dir.path().with_added("install").with_added("lib");
  assert!(cpp_install_lib_dir.exists());
  temp_dir.into_path(); //DEBUG
  let crate_dir = {
    let mut path = manifest_dir();
    path.push("test_assets");
    path.push("ctrt1");
    path.push("crate");
    path
  };
  assert!(crate_dir.exists());
  // we need to add root folder to cargo paths to force test crate
  // to use current version of cpp_to_rust
  let crate_cargo_dir = crate_dir.with_added(".cargo");
  if !crate_cargo_dir.exists() {
    create_dir(&crate_cargo_dir).unwrap();
  }
  let crate_cargo_config_path = crate_cargo_dir.with_added("config");
  if !crate_cargo_config_path.exists() {
    let mut file = create_file(crate_cargo_config_path).unwrap();
    file.write({
          let mut table = toml::Table::new();
          table.insert("paths".to_string(),
                       toml::Value::Array(vec![toml::Value::String("../../..".into())]));
          toml::Value::Table(table)
        }
        .to_string())
      .unwrap();
  }
  for cargo_cmd in &["build", "test", "doc"] {
    let mut command = Command::new("cargo");
    command.arg(cargo_cmd);
    command.arg("--verbose");
    command.arg("-j1");
    command.current_dir(&crate_dir);

    for name in &["LIBRARY_PATH", "LD_LIBRARY_PATH", "LIB", "PATH"] {
      let value = add_env_path_item(name, vec![cpp_install_lib_dir.clone()]).unwrap();
      command.env(name, value);
    }
    run_command(&mut command, false, false).unwrap();
  }
}
