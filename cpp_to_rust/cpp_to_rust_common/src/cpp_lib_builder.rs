use errors::Result;
use file_utils::{create_dir_all, path_to_str};
use utils::{is_msvc, run_command};
use utils::MapIfOk;
use string_utils::JoinWithString;
use std::process::Command;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CMakeVar {
  pub name: String,
  pub value: String,
}
impl CMakeVar {
  pub fn new<S1: Into<String>, S2: Into<String>>(name: S1, value: S2) -> CMakeVar {
    CMakeVar {
      name: name.into(),
      value: value.into(),
    }
  }
  pub fn new_list<I, S, L>(name: S, paths: L) -> Result<CMakeVar>
    where S: Into<String>,
          I: AsRef<str>,
          L: IntoIterator<Item = I>
  {
    let value = paths.into_iter()
      .map_if_ok(|s| -> Result<_> {
        if s.as_ref().contains(';') {
          Err(format!("can't pass value to cmake because ';' symbol is reserved: {}",
                      s.as_ref())
            .into())
        } else {
          Ok(s)
        }
      })?
      .into_iter()
      .join(";");
    Ok(CMakeVar::new(name, value))
  }

  pub fn new_path_list<I, S, L>(name: S, paths: L) -> Result<CMakeVar>
    where S: Into<String>,
          I: AsRef<Path>,
          L: IntoIterator<Item = I>
  {
    CMakeVar::new_list(name,
                       paths.into_iter()
                         .map_if_ok(|x| path_to_str(x.as_ref()).map(|x| x.to_string()))?)
  }
}

#[derive(Debug, Clone)]
pub enum BuildType {
  Debug,
  Release,
}

#[derive(Debug, Clone)]
pub struct CppLibBuilder {
  pub cmake_source_dir: PathBuf,
  pub build_dir: PathBuf,
  pub install_dir: PathBuf,
  pub num_jobs: Option<i32>,
  pub build_type: BuildType,
  pub pipe_output: bool,
  pub cmake_vars: Vec<CMakeVar>,
}

impl CppLibBuilder {
  pub fn run(self) -> Result<()> {
    if !self.build_dir.exists() {
      create_dir_all(&self.build_dir)?;
    }
    let mut cmake_command = Command::new("cmake");
    cmake_command.arg(self.cmake_source_dir)
      .current_dir(&self.build_dir);
    let actual_build_type = if is_msvc() {
      // Rust always links to release version of MSVC runtime, so
      // link will fail if C library is built in debug mode
      BuildType::Release
    } else {
      self.build_type
    };
    if is_msvc() {
      cmake_command.arg("-G").arg("NMake Makefiles");
    }
    let mut actual_cmake_vars = self.cmake_vars.clone();
    actual_cmake_vars.push(CMakeVar::new("CMAKE_BUILD_TYPE",
                                         match actual_build_type {
                                           BuildType::Release => "Release",
                                           BuildType::Debug => "Debug",
                                         }));
    actual_cmake_vars.push(CMakeVar::new("CMAKE_INSTALL_PREFIX", path_to_str(&self.install_dir)?));

    for var in actual_cmake_vars {
      cmake_command.arg(format!("-D{}={}", var.name, var.value));
    }
    run_command(&mut cmake_command)?;

    let make_command_name = if is_msvc() { "nmake" } else { "make" }.to_string();
    let mut make_args = Vec::new();
    let num_jobs = if let Some(x) = self.num_jobs {
      x
    } else {
      ::num_cpus::get() as i32
    };
    if !is_msvc() {
      // nmake doesn't support multiple jobs
      // TODO: allow to use jom
      make_args.push(format!("-j{}", num_jobs));
    }
    make_args.push("install".to_string());
    let mut make_command = Command::new(make_command_name);
    make_command.args(&make_args)
      .current_dir(self.build_dir);
    run_command(&mut make_command)?;
    Ok(())
  }
}
