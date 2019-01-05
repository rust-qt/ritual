/*use common::cpp_build_config::CppBuildConfigData;
use common::cpp_lib_builder::{BuildType, CppLibBuilder};
use common::errors::fancy_unwrap;
use common::target;
use common::utils::{add_env_path_item, run_command};
use config::{Config, CrateProperties};
use std::path::PathBuf;
use std::process::Command;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum TempTestDir {
    System(::tempdir::TempDir),
    Custom(PathBuf),
}

impl TempTestDir {
    pub fn new(name: &str) -> TempTestDir {
        if let Ok(value) = ::std::env::var("CPP_TO_RUST_TEMP_TEST_DIR") {
            let path = canonicalize(PathBuf::from(value)).unwrap().join(name);
            create_dir_all(&path).unwrap();
            TempTestDir::Custom(path)
        } else {
            TempTestDir::System(::tempdir::TempDir::new(name).unwrap())
        }
    }

    pub fn path(&self) -> &Path {
        match *self {
            TempTestDir::System(ref dir) => dir.path(),
            TempTestDir::Custom(ref path) => path,
        }
    }
}


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
    let build_dir = temp_dir.path().join("build");
    let install_dir = temp_dir.path().join("install");
    if !build_dir.exists() {
        create_dir(&build_dir).unwrap();
    }
    if !install_dir.exists() {
        create_dir(&install_dir).unwrap();
    }
    fancy_unwrap(
        CppLibBuilder {
            cmake_source_dir: cpp_lib_source_dir,
            build_dir: build_dir,
            build_type: BuildType::Release,
            install_dir: Some(install_dir),
            num_jobs: None,
            cmake_vars: Vec::new(),
        }
        .run(),
    );
    temp_dir
}

#[test]
fn full_run() {
    let temp_dir = build_cpp_lib();
    let crate_dir = temp_dir.path().join("crate");
    let cpp_install_lib_dir = temp_dir.path().join("install").join("lib");
    assert!(cpp_install_lib_dir.exists());
    let crate_properties = CrateProperties::new("rust_ctrt1", "0.0.0");

    let mut config = Config::new(
        &crate_dir,
        temp_dir.path().join("cache"),
        crate_properties,
    );
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
        config
            .cpp_build_config_mut()
            .add(target::Condition::True, data);
    }
    {
        let mut data = CppBuildConfigData::new();
        data.add_compiler_flag("-fPIC");
        data.add_compiler_flag("-std=gnu++11");
        config
            .cpp_build_config_mut()
            .add(target::Condition::Env(target::Env::Msvc).negate(), data);
    }
    if target::current_env() == target::Env::Msvc {
        config.add_cpp_parser_argument("-std=c++14");
    } else {
        config.add_cpp_parser_argument("-std=gnu++11");
    }
    config.set_crate_template_path(&crate_template_path);
    fancy_unwrap(config.exec());
    assert!(crate_dir.exists());

    for cargo_cmd in &["update", "build", "test", "doc"] {
        let mut command = Command::new("cargo");
        command.arg(cargo_cmd);
        command.arg("-v");
        if *cargo_cmd != "update" {
            command.arg("-j1");
        }
        //    if *cargo_cmd == "test" {
        //      command.arg("--");
        //      command.arg("--nocapture");
        //    }
        command.current_dir(&crate_dir);
        command.env("CPP_TO_RUST_INCLUDE_PATHS", &include_path);
        command.env("CPP_TO_RUST_LIB_PATHS", &cpp_install_lib_dir);
        command.env(
            "PATH",
            add_env_path_item("PATH", vec![cpp_install_lib_dir.clone()]).unwrap(),
        );
        command.env(
            "LD_LIBRARY_PATH",
            add_env_path_item("LD_LIBRARY_PATH", vec![cpp_install_lib_dir.clone()]).unwrap(),
        );
        run_command(&mut command).unwrap();
    }
}
*/
