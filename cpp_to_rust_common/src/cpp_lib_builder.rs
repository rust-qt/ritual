use errors::Result;
use file_utils::{create_dir_all, path_to_str};
use utils::{is_msvc, run_command};
use utils::MapIfOk;
use string_utils::JoinWithString;
use std::process::Command;
use std::path::{Path, PathBuf};

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
  pub fn new_list<I, S, L>(name: S, paths: L) -> CMakeVar
    where S: Into<String>, I: AsRef<str>, L: IntoIterator<Item=I>
  {
    let value = paths.into_iter().map(|s| {
      if s.as_ref().contains(' ') {
        format!("\"{}\"", s.as_ref())
      } else {
        s.as_ref().to_string()
      }
    }).join(" ");
    CMakeVar::new(name, value)
  }

  pub fn new_path_list<I, S, L>(name: S, paths: L) -> Result<CMakeVar>
    where S: Into<String>, I: AsRef<Path>, L: IntoIterator<Item=I>
  {
    Ok(CMakeVar::new_list(name, paths.into_iter().map_if_ok(|x| {
      path_to_str(x.as_ref()).map(|x| x.to_string())
    })?))
  }
}

pub struct CppLibBuilder {
  pub cmake_source_dir: PathBuf,
  pub build_dir: PathBuf,
  pub install_dir: PathBuf,
  pub num_jobs: Option<i32>,
//  pub linker_env_library_dirs: Option<&'a [PathBuf]>,
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
      .arg(format!("-DCMAKE_INSTALL_PREFIX={}",
                   try!(path_to_str(&self.install_dir))))
      .current_dir(&self.build_dir);
    if is_msvc() {
      cmake_command.arg("-G").arg("NMake Makefiles");
      // Rust always links to release version of MSVC runtime, so
      // link will fail if C library is built in debug mode
      cmake_command.arg("-DCMAKE_BUILD_TYPE=Release");
    }
    for var in self.cmake_vars {
      cmake_command.arg(format!("-D{}={}", var.name, var.value));
    }
    // TODO: enable release mode on other platforms if cargo is in release mode
    // (maybe build C library in both debug and release in separate folders)
    try!(run_command(&mut cmake_command, false, self.pipe_output));

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
    // TODO: use link_directories() cmake directive

//    if let Some(linker_env_library_dirs) = self.linker_env_library_dirs {
//      if !linker_env_library_dirs.is_empty() {
//        for name in &["LIBRARY_PATH", "LD_LIBRARY_PATH", "LIB"] {
//          let value = try!(add_env_path_item(name, Vec::from(linker_env_library_dirs)));
//          make_command.env(name, value);
//        }
//      }
//    }
    try!(run_command(&mut make_command, false, self.pipe_output));
    Ok(())
  }
}
