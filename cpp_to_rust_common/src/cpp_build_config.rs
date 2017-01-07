pub use serializable::{CppBuildConfig, CppBuildConfigData, CppLibraryType};

use std::path::PathBuf;

/// Machine-specific information
/// required to build the C++ wrapper library.
#[derive(Debug, Default, Clone)]
pub struct CppBuildPaths {
  lib_paths: Vec<PathBuf>,
  framework_paths: Vec<PathBuf>,
  include_paths: Vec<PathBuf>,
}

impl CppBuildPaths {
  pub fn new() -> CppBuildPaths {
    CppBuildPaths::default()
  }

  /// Adds path to a lib directory.
  /// It's supplied to the linker via `-L` option or environment variables.
  pub fn add_lib_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.lib_paths.push(path.into());
  }

  /// Adds path to a framework directory (OS X specific).
  /// It's supplied to the linker via `-F` option or environment variables.
  pub fn add_framework_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.framework_paths.push(path.into());
  }

  /// Adds path to an include directory.
  /// It's supplied to the C++ parser
  /// and the C++ compiler via `-I` option.
  pub fn add_include_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.include_paths.push(path.into());
  }

  pub fn apply_env(&mut self) {
    use ::std::env;
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

  pub fn lib_paths(&self) -> &[PathBuf] {
    &self.lib_paths
  }

  pub fn framework_paths(&self) -> &[PathBuf] {
    &self.framework_paths
  }

  pub fn include_paths(&self) -> &[PathBuf] {
    &self.include_paths
  }
}
