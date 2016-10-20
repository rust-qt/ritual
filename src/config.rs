use std::path::PathBuf;
use errors::Result;

#[derive(Default, Debug)]
pub struct Config {
  /// Extra libraries to be linked.
  /// Used as "-l" option to linker.
  extra_libs: Vec<String>,

  /// Paths to include directories supplied to the C++ parser
  /// and compiler of the C++ wrapper via '-I' option.
  /// This is detected automatically for Qt libraries using qmake.
  /// Paths can be relative to lib spec file's directory.
  include_paths: Vec<PathBuf>,

  /// Paths to library directories supplied to the linker
  /// via '-L' option or environment variables.
  /// This is detected automatically for Qt libraries using qmake.
  /// Paths can be relative to lib spec file's directory.
  lib_paths: Vec<PathBuf>,

  /// Paths to include directories of the library.
  /// Only types and methods declared within these directories
  /// will be parsed.
  /// This is detected automatically for Qt libraries using qmake.
  /// Paths can be relative to lib spec file's directory.
  target_include_paths: Vec<PathBuf>,
  // TODO: allow both dirs and files in target_include_paths
  /// Name of the library's include file
  include_directives: Vec<PathBuf>,
}

impl Config {
  /// Creates empty Config
  pub fn new() -> Config {
    Config::default()
  }

  pub fn add_extra_lib<P: Into<String>>(&mut self, lib: P) {
    self.extra_libs.push(lib.into());
  }

  pub fn add_include_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.include_paths.push(path.into());
  }

  pub fn add_lib_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.lib_paths.push(path.into());
  }

  pub fn add_target_include_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.target_include_paths.push(path.into());
  }

  pub fn add_include_directive<P: Into<PathBuf>>(&mut self, path: P) {
    self.include_directives.push(path.into());
  }

  pub fn exec(self) -> Result<()> {
    ::launcher::run_from_build_script(self)
  }

  pub fn extra_libs(&self) -> &[String] {
    &self.extra_libs
  }

  pub fn include_paths(&self) -> &[PathBuf] {
    &self.include_paths
  }

  pub fn lib_paths(&self) -> &[PathBuf] {
    &self.lib_paths
  }

  pub fn target_include_paths(&self) -> &[PathBuf] {
    &self.target_include_paths
  }

  pub fn include_directives(&self) -> &[PathBuf] {
    &self.include_directives
  }
}
