//! Implements building a CMake-based C++ library.

use crate::cpp_build_config::CppBuildConfigData;
use crate::cpp_build_config::CppBuildPaths;
use crate::cpp_build_config::CppLibraryType;
use crate::errors::{err_msg, Result};
use crate::file_utils::{create_dir_all, path_to_str};
use crate::target;
use crate::utils::run_command;
use crate::utils::run_command_and_capture_output;
use crate::utils::CommandOutput;
use crate::utils::MapIfOk;
use itertools::Itertools;
use log::debug;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A CMake variable with a name and a value.
#[derive(Debug, Clone)]
pub struct CMakeVar {
    pub name: String,
    pub value: String,
}
impl CMakeVar {
    /// Creates a new variable.
    pub fn new<S1: Into<String>, S2: Into<String>>(name: S1, value: S2) -> CMakeVar {
        CMakeVar {
            name: name.into(),
            value: value.into(),
        }
    }
    /// Creates a new variable containing a list of values.
    pub fn new_list<I, S, L>(name: S, values: L) -> Result<CMakeVar>
    where
        S: Display + Into<String>,
        I: AsRef<str> + Display,
        L: IntoIterator<Item = I>,
    {
        let value = values
            .into_iter()
            .map_if_ok(|s| -> Result<_> {
                if s.as_ref().contains(';') {
                    Err(err_msg(format!(
                        "can't pass value to cmake because ';' symbol is reserved: {}",
                        s.as_ref()
                    )))
                } else {
                    Ok(s)
                }
            })?
            .into_iter()
            .join(";");
        Ok(CMakeVar::new(name, value))
    }

    /// Creates a new variable containing a list of paths.
    pub fn new_path_list<I, S, L>(name: S, paths: L) -> Result<CMakeVar>
    where
        S: Into<String> + Display,
        I: AsRef<Path>,
        L: IntoIterator<Item = I>,
    {
        CMakeVar::new_list(
            name,
            paths
                .into_iter()
                .map_if_ok(|x| path_to_str(x.as_ref()).map(|x| x.to_string()))?,
        )
    }
}

/// CMake build type (Debug or Release)
#[derive(Debug, Clone)]
pub enum BuildType {
    Debug,
    Release,
}

/// Implements building a CMake-based C++ library.
/// Construct a value and call `run()` to execute building.
#[derive(Debug, Clone)]
pub struct CppLibBuilder {
    /// Path to the source directory containing CMake config file
    pub cmake_source_dir: PathBuf,
    /// Path to the build directory (may not exist before building)
    pub build_dir: PathBuf,
    /// Path to the install directory (may not exist before building)
    pub install_dir: Option<PathBuf>,
    /// Number of threads used to build the library. If `None` is supplied,
    /// number of threads will be detected automatically.
    pub num_jobs: Option<usize>,
    /// CMake build type (Debug or Release)
    pub build_type: BuildType,
    /// Additional variables passed to CMake
    pub cmake_vars: Vec<CMakeVar>,
    pub capture_output: bool,
    pub skip_cmake: bool,
    pub skip_cmake_after_first_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CppLibBuilderOutput {
    Success,
    Fail(CommandOutput),
}

impl CppLibBuilder {
    /// Builds the library.
    pub fn run(&mut self) -> Result<CppLibBuilderOutput> {
        if !self.build_dir.exists() {
            create_dir_all(&self.build_dir)?;
        }
        if !self.skip_cmake {
            let mut cmake_command = Command::new("cmake");
            cmake_command
                .arg(&self.cmake_source_dir)
                .current_dir(&self.build_dir);
            let actual_build_type = if target::current_env() == target::Env::Msvc {
                // Rust always links to release version of MSVC runtime, so
                // link will fail if C library is built in debug mode
                BuildType::Release
            } else {
                self.build_type.clone()
            };
            if target::current_os() == target::OS::Windows {
                match target::current_env() {
                    target::Env::Msvc => {
                        cmake_command.arg("-G").arg("NMake Makefiles");
                    }
                    target::Env::Gnu => {
                        cmake_command.arg("-G").arg("MinGW Makefiles");
                    }
                    _ => {}
                }
            }
            let mut actual_cmake_vars = self.cmake_vars.clone();
            actual_cmake_vars.push(CMakeVar::new(
                "CMAKE_BUILD_TYPE",
                match actual_build_type {
                    BuildType::Release => "Release",
                    BuildType::Debug => "Debug",
                },
            ));
            if let Some(install_dir) = &self.install_dir {
                actual_cmake_vars.push(CMakeVar::new(
                    "CMAKE_INSTALL_PREFIX",
                    path_to_str(install_dir)?,
                ));
            }

            for var in actual_cmake_vars {
                cmake_command.arg(format!("-D{}={}", var.name, var.value));
            }
            if self.capture_output {
                let output = run_command_and_capture_output(&mut cmake_command)?;
                if !output.is_success() {
                    return Ok(CppLibBuilderOutput::Fail(output));
                }
            } else {
                run_command(&mut cmake_command)?;
            }
        }

        if self.skip_cmake_after_first_run {
            self.skip_cmake = true;
        }

        let mut make_command_name = if target::current_os() == target::OS::Windows {
            match target::current_env() {
                target::Env::Msvc => "nmake",
                target::Env::Gnu => "mingw32-make",
                _ => "make",
            }
        } else {
            "make"
        };

        let mut make_args = Vec::new();
        let num_jobs = if let Some(x) = self.num_jobs {
            x
        } else {
            ::num_cpus::get()
        };
        if target::current_env() == target::Env::Msvc && num_jobs > 1 {
            debug!("Checking for jom");
            if run_command(&mut Command::new("jom").arg("/version")).is_ok() {
                debug!("jom will be used instead of nmake.");
                make_command_name = "jom";
                make_args.push("/J".to_string());
                make_args.push(num_jobs.to_string());
            } else {
                debug!("jom not found in PATH. Using nmake.");
            }
        }
        if target::current_env() != target::Env::Msvc {
            make_args.push(format!("-j{}", num_jobs));
        }
        if self.install_dir.is_some() {
            make_args.push("install".to_string());
        }
        let mut make_command = Command::new(make_command_name);
        make_command.args(&make_args).current_dir(&self.build_dir);
        if self.capture_output {
            let output = run_command_and_capture_output(&mut make_command)?;
            if !output.is_success() {
                return Ok(CppLibBuilderOutput::Fail(output));
            }
        } else {
            run_command(&mut make_command)?;
        }
        Ok(CppLibBuilderOutput::Success)
    }
}

pub fn c2r_cmake_vars(
    cpp_build_config_data: &CppBuildConfigData,
    cpp_build_paths: &CppBuildPaths,
    library_type: Option<&CppLibraryType>,
) -> Result<Vec<CMakeVar>> {
    let mut cmake_vars = Vec::new();
    if let Some(library_type) = library_type {
        cmake_vars.push(CMakeVar::new(
            "C2R_LIBRARY_TYPE",
            match *library_type {
                CppLibraryType::Shared => "SHARED",
                CppLibraryType::Static => "STATIC",
            },
        ));
    }
    cmake_vars.push(CMakeVar::new_path_list(
        "C2R_INCLUDE_PATHS",
        cpp_build_paths.include_paths(),
    )?);
    cmake_vars.push(CMakeVar::new_path_list(
        "C2R_LIB_PATHS",
        cpp_build_paths.lib_paths(),
    )?);
    cmake_vars.push(CMakeVar::new_path_list(
        "C2R_FRAMEWORK_PATHS",
        cpp_build_paths.framework_paths(),
    )?);
    cmake_vars.push(CMakeVar::new_list(
        "C2R_LINKED_LIBS",
        cpp_build_config_data.linked_libs(),
    )?);
    cmake_vars.push(CMakeVar::new_list(
        "C2R_LINKED_FRAMEWORKS",
        cpp_build_config_data.linked_frameworks(),
    )?);
    cmake_vars.push(CMakeVar::new(
        "C2R_COMPILER_FLAGS",
        cpp_build_config_data.compiler_flags().join(" "),
    ));
    Ok(cmake_vars)
}
