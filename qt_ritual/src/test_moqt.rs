use crate::lib_configs::{
    global_config, MOQT_INSTALL_DIR_ENV_VAR_NAME, MOQT_TEMPLATE_DIR_ENV_VAR_NAME,
};
use ritual::cli::{self, Command, Options};
use ritual_common::cpp_lib_builder::{BuildType, CppLibBuilder};
use ritual_common::env_var_names;
use ritual_common::errors::{FancyUnwrap, Result};
use ritual_common::file_utils::{
    canonicalize, copy_recursively, create_dir, create_dir_all, create_file, file_to_string,
    read_dir, remove_dir_all, repo_dir_path,
};
use ritual_common::utils::add_env_path_item;
use std::env;
use std::io::Write;
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
        match self {
            TempTestDir::System(dir) => dir.path(),
            TempTestDir::Custom(path) => path,
        }
    }
}

fn build_moqt() -> Result<TempTestDir> {
    let cpp_lib_source_dir = repo_dir_path("qt_ritual/test_assets/moqt")?;

    let temp_dir = TempTestDir::new("test_full_run");
    let build_dir = temp_dir.path().join("build");
    let install_dir = temp_dir.path().join("install");
    let template_dir = temp_dir.path().join("template");
    create_dir_all(&build_dir)?;
    create_dir_all(&install_dir)?;
    if template_dir.exists() {
        remove_dir_all(&template_dir)?;
    }
    create_dir_all(&template_dir)?;
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

    let repo_template_dir = repo_dir_path("qt_ritual/crate_templates")?;
    let add_env = |name, path: &Path| -> Result<()> {
        let value = add_env_path_item(name, vec![path.to_path_buf()])?;
        env::set_var(name, value);
        Ok(())
    };

    for lib in &["moqt_core", "moqt_gui"] {
        let path = install_dir.join("include").join(lib);
        add_env("CPLUS_INCLUDE_PATH", &path)?;
        add_env("INCLUDE", &path)?; // for Windows

        copy_recursively(&repo_template_dir.join(lib), &template_dir.join(lib))?;
        let real_lib = lib.replace("mo", "");
        let real_template_src_dir = repo_template_dir.join(&real_lib).join("src");
        let template_src_dir = template_dir.join(lib).join("src");
        create_dir(&template_src_dir)?;
        for item in read_dir(&real_template_src_dir)? {
            let item = item?;
            let content = file_to_string(item.path())?;
            let new_content = content
                .replace("qt_core", "moqt_core")
                .replace("qt_gui", "moqt_gui");
            let mut file = create_file(template_src_dir.join(item.file_name()))?;
            write!(file, "{}", new_content)?;
        }

        if repo_template_dir.join(&real_lib).join("c_lib").exists() {
            copy_recursively(
                &repo_template_dir.join(&real_lib).join("c_lib"),
                &template_dir.join(lib).join("c_lib"),
            )?;
        }
    }
    let lib_path = install_dir.join("lib");
    add_env("LIBRARY_PATH", &lib_path)?;
    add_env("LD_LIBRARY_PATH", &lib_path)?;
    add_env("DYLD_LIBRARY_PATH", &lib_path)?;
    add_env("PATH", &lib_path)?;
    add_env(env_var_names::LIBRARY_PATH, &lib_path)?;
    env::set_var(MOQT_INSTALL_DIR_ENV_VAR_NAME, &install_dir);
    env::set_var(MOQT_TEMPLATE_DIR_ENV_VAR_NAME, &template_dir);
    Ok(temp_dir)
}

#[test]
fn test_moqt() {
    let temp_dir = build_moqt().fancy_unwrap();
    let workspace = temp_dir.path().join("workspace");
    create_dir_all(&workspace).unwrap();

    cli::run(
        Options {
            workspace,
            local_paths: Some(true),
            crates: "moqt_core,moqt_gui".into(),
            output_crates_version: None,
            command: Command::Parse,
        },
        global_config(),
    )
    .fancy_unwrap();
}
