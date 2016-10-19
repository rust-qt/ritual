use std::path::{Path, PathBuf};
use errors::Result;

#[derive(Default, Debug)]
pub struct Config {
  /// Extra libraries to be linked.
  /// Used as "-l" option to linker.
  extra_libs: Vec<PathBuf>,

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

  pub fn add_extra_lib<P: AsRef<Path>>(&mut self, path: P) {
    self.extra_libs.push(path.as_ref().to_path_buf());
  }

  pub fn add_include_path<P: AsRef<Path>>(&mut self, path: P) {
    self.include_paths.push(path.as_ref().to_path_buf());
  }

  pub fn add_lib_path<P: AsRef<Path>>(&mut self, path: P) {
    self.lib_paths.push(path.as_ref().to_path_buf());
  }

  pub fn add_target_include_path<P: AsRef<Path>>(&mut self, path: P) {
    self.target_include_paths.push(path.as_ref().to_path_buf());
  }

  pub fn add_include_directive<P: AsRef<Path>>(&mut self, path: P) {
    self.include_directives.push(path.as_ref().to_path_buf());
  }

  pub fn exec(self) -> Result<()> {
    ::launcher::run_from_build_script(self)
  }
}
