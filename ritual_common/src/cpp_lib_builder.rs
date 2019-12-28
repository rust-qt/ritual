//! Implements building a CMake-based C++ library.

use crate::cpp_build_config::{CppBuildConfigData, CppBuildPaths, CppLibraryType};
use crate::errors::{err_msg, Result};
use crate::file_utils::{create_dir_all, file_to_string, path_to_str};
use crate::utils::{run_command, run_command_and_capture_output, CommandOutput, MapIfOk};
use crate::{env_var_names, target};
use itertools::Itertools;
use serde_derive::{Deserialize, Serialize};
use std::env;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A CMake variable with a name and a value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CMakeVar {
    pub name: String,
    pub value: String,
}
impl CMakeVar {
    /// Creates a new variable.
    pub fn new<S1: Into<String>, S2: Into<String>>(name: S1, value: S2) -> Self {
        CMakeVar {
            name: name.into(),
            value: value.into(),
        }
    }
    /// Creates a new variable containing a list of values.
    pub fn new_list<I, S, L>(name: S, values: L) -> Result<Self>
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
                .map_if_ok(|x| path_to_str(x.as_ref()).map(ToString::to_string))?,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CppLibBuilderOutput {
    Success,
    Fail(CommandOutput),
}

impl CppLibBuilderOutput {
    pub fn is_success(&self) -> bool {
        self == &CppLibBuilderOutput::Success
    }
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
                .arg("-Wno-dev")
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
            if let Ok(args) = env::var(env_var_names::CMAKE_ARGS) {
                cmake_command.args(shell_words::split(&args)?);
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

        if target::current_env() == target::Env::Msvc && self.capture_output {
            let path = self.build_dir.join("nmake_output.txt");
            run_command(
                Command::new("cmd")
                    .arg("/C")
                    .arg(format!(
                        "cmake --build . -- clean > {} 2>&1",
                        path_to_str(&path)?
                    ))
                    .current_dir(&self.build_dir),
            )?;
        } else {
            run_command(
                Command::new("cmake")
                    .arg("--build")
                    .arg(".")
                    .arg("--")
                    .arg("clean")
                    .current_dir(&self.build_dir),
            )?;
        }

        let mut make_args = vec!["--build".to_string(), ".".to_string(), "--".to_string()];
        let num_jobs = if let Some(x) = self.num_jobs {
            x
        } else {
            ::num_cpus::get()
        };

        if target::current_env() != target::Env::Msvc {
            make_args.push(format!("-j{}", num_jobs));
        }
        if self.install_dir.is_some() {
            make_args.push("install".to_string());
        }

        let mut capture_output_file = None;
        let mut make_command = if target::current_env() == target::Env::Msvc && self.capture_output
        {
            let path = self.build_dir.join("nmake_output.txt");
            let mut make_command = Command::new("cmd");
            make_command.arg("/C").arg(format!(
                "cmake {} > {} 2>&1",
                make_args.join(" "),
                path_to_str(&path)?
            ));
            capture_output_file = Some(path);
            make_command
        } else {
            let mut make_command = Command::new("cmake");
            make_command.args(&make_args);
            make_command
        };

        make_command.current_dir(&self.build_dir);
        if self.capture_output {
            if let Some(capture_output_file) = capture_output_file {
                if let Err(err) = run_command(&mut make_command) {
                    let output = CommandOutput {
                        status: 0,
                        stderr: format!(
                            "{}\n{}",
                            err.to_string(),
                            file_to_string(capture_output_file)?
                        ),
                        stdout: String::new(),
                    };
                    return Ok(CppLibBuilderOutput::Fail(output));
                }
            } else {
                let output = run_command_and_capture_output(&mut make_command)?;
                if !output.is_success() {
                    return Ok(CppLibBuilderOutput::Fail(output));
                }
            }
        } else {
            run_command(&mut make_command)?;
        }
        Ok(CppLibBuilderOutput::Success)
    }
}

pub struct CMakeConfigData<'a, 'b> {
    pub cpp_build_config_data: &'a CppBuildConfigData,
    pub cpp_build_paths: &'b CppBuildPaths,
    pub library_type: Option<CppLibraryType>,
    pub cpp_library_version: Option<String>,
}

pub fn version_to_number(version: &str) -> Result<u32> {
    const COEF: u64 = 100;
    let parsed = semver::Version::parse(version)?;
    let value = parsed.major * COEF * COEF + parsed.minor * COEF + parsed.patch;
    Ok(value as u32)
}

impl<'a, 'b> CMakeConfigData<'a, 'b> {
    pub fn cmake_vars(&self) -> Result<Vec<CMakeVar>> {
        let mut cmake_vars = Vec::new();
        if let Some(library_type) = self.library_type {
            cmake_vars.push(CMakeVar::new(
                "RITUAL_LIBRARY_TYPE",
                match library_type {
                    CppLibraryType::Shared => "SHARED",
                    CppLibraryType::Static => "STATIC",
                },
            ));
        }
        if let Some(version) = &self.cpp_library_version {
            cmake_vars.push(CMakeVar::new(
                "RITUAL_CPP_LIB_VERSION",
                version_to_number(version)?.to_string(),
            ));
        }
        cmake_vars.push(CMakeVar::new_path_list(
            "RITUAL_INCLUDE_PATH",
            self.cpp_build_paths.include_paths(),
        )?);
        cmake_vars.push(CMakeVar::new_path_list(
            "RITUAL_LIBRARY_PATH",
            self.cpp_build_paths.lib_paths(),
        )?);
        cmake_vars.push(CMakeVar::new_path_list(
            "RITUAL_FRAMEWORK_PATH",
            self.cpp_build_paths.framework_paths(),
        )?);
        cmake_vars.push(CMakeVar::new_list(
            "RITUAL_LINKED_LIBS",
            self.cpp_build_config_data.linked_libs(),
        )?);
        cmake_vars.push(CMakeVar::new_list(
            "RITUAL_LINKED_FRAMEWORKS",
            self.cpp_build_config_data.linked_frameworks(),
        )?);
        cmake_vars.push(CMakeVar::new(
            "RITUAL_COMPILER_FLAGS",
            self.cpp_build_config_data.compiler_flags().join(" "),
        ));
        cmake_vars.extend_from_slice(self.cpp_build_config_data.cmake_vars());
        Ok(cmake_vars)
    }
}
