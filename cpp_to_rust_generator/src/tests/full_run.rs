use common::file_utils::{PathBufWithAdded, create_dir, create_file};
use common::utils::run_command;
use common::cpp_lib_builder::{CppLibBuilder, BuildType};
use common::errors::fancy_unwrap;
use config::{Config, CrateProperties};
use common::cpp_build_config::{CppBuildConfig, CppBuildConfigData};
use common::target;

use std::process::Command;
use std::path::PathBuf;
use tests::TempTestDir;

fn build_cpp_lib() -> TempTestDir {
  let cpp_lib_source_dir = {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test_assets");
    path.push("ctrt1");
    path.push("cpp");
    path
  };
  assert!(cpp_lib_source_dir.exists());
  let temp_dir = TempTestDir::new("test_full_run");
  let build_dir = temp_dir.path().with_added("build");
  let install_dir = temp_dir.path().with_added("install");
  if !build_dir.exists() {
    create_dir(&build_dir).unwrap();
  }
  if !install_dir.exists() {
    create_dir(&install_dir).unwrap();
  }
  fancy_unwrap(CppLibBuilder {
      cmake_source_dir: cpp_lib_source_dir,
      build_dir: build_dir,
      build_type: BuildType::Release,
      install_dir: install_dir,
      num_jobs: None,
      cmake_vars: Vec::new(),
      pipe_output: true,
    }
    .run());
  temp_dir
}

extern crate toml;

#[test]
fn full_run() {
  let temp_dir = build_cpp_lib();
  let crate_dir = temp_dir.path().with_added("crate");
  let cpp_install_lib_dir = temp_dir.path().with_added("install").with_added("lib");
  assert!(cpp_install_lib_dir.exists());
  let mut crate_properties = CrateProperties::new("rust_ctrt1", "0.0.0");
  crate_properties.set_links_attribute("ctrt1");

  let mut config = Config::new(&crate_dir,
                               temp_dir.path().with_added("cache"),
                               crate_properties);
  config.add_include_directive("ctrt1/all.h");
  let include_path = {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test_assets");
    path.push("ctrt1");
    path.push("cpp");
    path.push("include");
    path
  };
  let crate_template_path = {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test_assets");
    path.push("ctrt1");
    path.push("crate");
    path
  };
  assert!(include_path.exists());
  config.add_include_path(&include_path);
  config.add_target_include_path(&include_path);

  {
    let mut data = CppBuildConfigData::new();
    data.add_linked_lib("ctrt1");
    config.cpp_build_config_mut().add(target::Condition::True, data);
  }
  {
    let mut data = CppBuildConfigData::new();
    data.add_compiler_flag("-fPIC");
    config.cpp_build_config_mut().add(target::Condition::Env(target::Env::Msvc).negate(), data);
  }
  config.set_crate_template_path(&crate_template_path);
  fancy_unwrap(config.exec());
  assert!(crate_dir.exists());
  // we need to add root folder to cargo paths to force test crate
  // to use current version of cpp_to_rust
  let crate_cargo_dir = crate_dir.with_added(".cargo");
  if !crate_cargo_dir.exists() {
    create_dir(&crate_cargo_dir).unwrap();
  }
  let crate_cargo_config_path = crate_cargo_dir.with_added("config");
  if !crate_cargo_config_path.exists() {
    let manifest_parent_path =
      PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    assert!(manifest_parent_path.exists());
    let dep_paths = vec![manifest_parent_path.with_added("cpp_to_rust_build_tools"),
                         manifest_parent_path.with_added("cpp_to_rust_common")];

    let mut file = create_file(crate_cargo_config_path).unwrap();
    file.write({
          let mut table = toml::Table::new();
          table.insert("paths".to_string(),
                       toml::Value::Array(dep_paths.iter()
                         .map(|p| toml::Value::String(p.to_str().unwrap().to_string()))
                         .collect()));
          toml::Value::Table(table)
        }
        .to_string())
      .unwrap();
  }
  for cargo_cmd in &["update", "build", "test", "doc"] {
    let mut command = Command::new("cargo");
    command.arg(cargo_cmd);
    command.arg("-vv");
    if *cargo_cmd != "update" {
      command.arg("-j1");
    }
    command.current_dir(&crate_dir);
    command.env("CPP_TO_RUST_INCLUDE_PATHS", &include_path);
    command.env("CPP_TO_RUST_LIB_PATHS", &cpp_install_lib_dir);
    run_command(&mut command, false, false).unwrap();
  }
}
