use crate::Options;
use ritual_common::cpp_lib_builder::{BuildType, CppLibBuilder};
use ritual_common::errors::{FancyUnwrap, Result};
use ritual_common::file_utils::canonicalize;
use ritual_common::file_utils::create_dir_all;
use ritual_common::file_utils::repo_crate_local_path;
use ritual_common::utils::add_env_path_item;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum TempTestDir {
    System(::tempdir::TempDir),
    Custom(PathBuf),
}

impl TempTestDir {
    pub fn new(name: &str) -> TempTestDir {
        if let Ok(value) = ::std::env::var("RITUAL_TEMP_TEST_DIR") {
            let path = PathBuf::from(value);
            create_dir_all(&path).unwrap();
            let path = canonicalize(path).unwrap().join(name);
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

fn build_cpp_lib() -> Result<TempTestDir> {
    let cpp_lib_source_dir = repo_crate_local_path("qt_ritual/test_assets/moqt")?;

    let temp_dir = TempTestDir::new("test_full_run");
    let build_dir = temp_dir.path().join("build/moqt_core");
    let install_dir = temp_dir.path().join("install");
    create_dir_all(&build_dir)?;
    create_dir_all(&install_dir)?;
    CppLibBuilder {
        cmake_source_dir: cpp_lib_source_dir,
        build_dir,
        build_type: BuildType::Release,
        install_dir: Some(install_dir.clone()),
        num_jobs: None,
        cmake_vars: Vec::new(),
        capture_output: false,
        skip_cmake: false,
        skip_cmake_after_first_run: false,
    }
    .run()?;

    let add_env = |name, path: &Path| -> Result<()> {
        let value = add_env_path_item(name, vec![path.to_path_buf()])?;
        env::set_var(name, value);
        Ok(())
    };

    for lib in &["moqt_core", "moqt_gui"] {
        add_env("CPLUS_INCLUDE_PATH", &install_dir.join("include").join(lib))?;
    }
    let lib_path = install_dir.join("lib");
    add_env("LIBRARY_PATH", &lib_path)?;
    add_env("LD_LIBRARY_PATH", &lib_path)?;
    add_env("DYLD_LIBRARY_PATH", &lib_path)?;
    add_env("PATH", &lib_path)?;
    add_env("RITUAL_LIB_PATH", &lib_path)?;
    env::set_var("MOQT_INSTALL_DIR", &install_dir);
    Ok(temp_dir)
}

#[test]
fn test_moqt() {
    let temp_dir = build_cpp_lib().unwrap();
    let workspace = temp_dir.path().join("workspace");
    create_dir_all(&workspace).unwrap();

    crate::run(Options {
        workspace,
        local_paths: Some(true),
        delete_database: true,
        crates: vec!["moqt_core".into(), "moqt_gui".into()],
        operations: vec!["main".into()],
    })
    .fancy_unwrap();
}
