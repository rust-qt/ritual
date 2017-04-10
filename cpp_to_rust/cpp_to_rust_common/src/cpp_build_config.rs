//! Types for configuring build script behavior.

pub use serializable::{CppBuildConfig, CppBuildConfigData, CppLibraryType};

use std::path::PathBuf;

/// Machine-specific information required to build the C++ wrapper library.
/// This type holds configuration properties that cannot be determined
/// at the time of crate generation because they are always platform-dependent.
///
/// By default, all path lists are empty, and the build script doesn't add
/// any extra directories to paths while compiling and linking the crate.
/// If `CPP_TO_RUST_LIB_PATHS`, `CPP_TO_RUST_FRAMEWORK_PATHS` or
/// `CPP_TO_RUST_INCLUDE_PATHS` environment variables are present during
/// execution of the build script, their values are used. A custom
/// build script can get an object of this type using `Config::cpp_build_paths_mut`
/// and use its API to set extra search paths.
///
/// This type is currently only used in `cpp_to_rust_build_tools`, but
/// `cpp_to_rust_generator` may start to use it in the future if needed.
#[derive(Debug, Default, Clone)]
pub struct CppBuildPaths {
  lib_paths: Vec<PathBuf>,
  framework_paths: Vec<PathBuf>,
  include_paths: Vec<PathBuf>,
}

impl CppBuildPaths {
  /// Constructs an empty configuration object.
  pub fn new() -> CppBuildPaths {
    CppBuildPaths::default()
  }

  /// Adds `path` to a lib directory.
  /// It's supplied to the linker via `-L` option or environment variables.
  pub fn add_lib_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.lib_paths.push(path.into());
  }

  /// Adds `path` to a framework directory (OS X specific).
  /// It's supplied to the linker via `-F` option or environment variables.
  pub fn add_framework_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.framework_paths.push(path.into());
  }

  /// Adds `path` to an include directory.
  /// It's supplied to the C++ parser
  /// and the C++ compiler via `-I` option.
  pub fn add_include_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.include_paths.push(path.into());
  }

  /// If `CPP_TO_RUST_LIB_PATHS`, `CPP_TO_RUST_FRAMEWORK_PATHS` or
  /// `CPP_TO_RUST_INCLUDE_PATHS` environment variables are present,
  /// their values override current values of the object.
  pub fn apply_env(&mut self) {
    use std::env;
    if let Ok(paths) = env::var("CPP_TO_RUST_LIB_PATHS") {
      self.lib_paths = env::split_paths(&paths).collect();
    }
    if let Ok(paths) = env::var("CPP_TO_RUST_FRAMEWORK_PATHS") {
      self.framework_paths = env::split_paths(&paths).collect();
    }
    if let Ok(paths) = env::var("CPP_TO_RUST_INCLUDE_PATHS") {
      self.include_paths = env::split_paths(&paths).collect();
    }
  }

  /// Returns paths added via `add_lib_path`.
  pub fn lib_paths(&self) -> &[PathBuf] {
    &self.lib_paths
  }

  /// Returns paths added via `add_framework_path`.
  pub fn framework_paths(&self) -> &[PathBuf] {
    &self.framework_paths
  }

  /// Returns paths added via `add_include_path`.
  pub fn include_paths(&self) -> &[PathBuf] {
    &self.include_paths
  }
}
