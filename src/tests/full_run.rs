use std;
use file_utils::PathBufWithAdded;
use utils::manifest_dir;
use cpp_lib_builder::CppLibBuilder;
use launcher::{BuildEnvironment, InvokationMethod, BuildProfile};
use launcher;
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
  std::fs::create_dir(&build_dir).unwrap();
  std::fs::create_dir(&install_dir).unwrap();
  CppLibBuilder {
      cmake_source_dir: &cpp_lib_source_dir,
      build_dir: &build_dir,
      install_dir: &install_dir,
      num_jobs: 1,
      linker_env_library_dirs: None,
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

#[test]
fn full_run() {
  let temp_dir = build_cpp_lib();
  let output_dir = temp_dir.path().with_added("rust");
  let cpp_install_lib_dir = temp_dir.path().with_added("install").with_added("lib");
  assert!(cpp_install_lib_dir.exists());
  temp_dir.into_path(); //DEBUG
  let lib_spec_dir = {
    let mut path = manifest_dir();
    path.push("test_assets");
    path.push("ctrt1");
    path.push("spec");
    path
  };
  assert!(lib_spec_dir.exists());
  launcher::run(BuildEnvironment {
      invokation_method: InvokationMethod::CommandLine,
      output_dir_path: output_dir,
      source_dir_path: lib_spec_dir,
      dependency_paths: Vec::new(),
      num_jobs: Some(1),
      build_profile: BuildProfile::Debug,
      extra_lib_paths: vec![cpp_install_lib_dir],
    })
    .unwrap_or_else(|e| {
      e.display_report();
      panic!("{}", e);
    });
}
