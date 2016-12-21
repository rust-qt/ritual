use target;

use std::path::PathBuf;

/// Information required to build the C++ wrapper library
/// on every supported platform.
#[derive(Default, Debug, Clone)]
pub struct CppBuildConfig {
  items: Vec<CppBuildConfigItem>,
}

#[derive(Debug, Clone)]
struct CppBuildConfigItem {
  condition: target::Condition,
  data: CppBuildConfigData,
}

/// Platform-specific information
/// required to build the C++ wrapper library.
#[derive(Debug, Default, Clone)]
pub struct CppBuildConfigData {
  linked_libs: Vec<String>,
  linked_frameworks: Vec<String>,
  cpp_compiler_flags: Vec<String>,
}

impl CppBuildConfigData {
  pub fn new() -> CppBuildConfigData {
    CppBuildConfigData::default()
  }

  /// Adds a library for linking. Used as `-l` option to the linker.
  pub fn add_linked_lib<P: Into<String>>(&mut self, lib: P) {
    self.linked_libs.push(lib.into());
  }

  /// Adds a framework for linking (OS X specific). Used as `-f` option to the linker.
  pub fn add_linked_framework<P: Into<String>>(&mut self, lib: P) {
    self.linked_frameworks.push(lib.into());
  }

  /// Adds a command line argument for the C++ compiler.
  pub fn add_cpp_compiler_flag<P: Into<String>>(&mut self, lib: P) {
    self.cpp_compiler_flags.push(lib.into());
  }

  /// Adds multiple flags. See `CppBuildConfigData::add_cpp_compiler_flag`.
  pub fn add_cpp_compiler_flags<Item, Iter>(&mut self, items: Iter)
    where Item: Into<String>,
          Iter: IntoIterator<Item = Item>
  {
    for item in items {
      self.cpp_compiler_flags.push(item.into());
    }
  }

  pub fn linked_libs(&self) -> &[String] {
    &self.linked_libs
  }

  pub fn linked_frameworks(&self) -> &[String] {
    &self.linked_frameworks
  }

  pub fn cpp_compiler_flags(&self) -> &[String] {
    &self.cpp_compiler_flags
  }


  fn add_from(&mut self, other: &CppBuildConfigData) {
    self.linked_libs.append(&mut other.linked_libs.clone());
    self.linked_frameworks.append(&mut other.linked_frameworks.clone());
    self.cpp_compiler_flags.append(&mut other.cpp_compiler_flags.clone());
  }
}

impl CppBuildConfig {
  pub fn new() -> CppBuildConfig {
    CppBuildConfig::default()
  }
  pub fn add(&mut self, condition: target::Condition, data: CppBuildConfigData) {
    self.items.push(CppBuildConfigItem {
      condition: condition,
      data: data,
    });
  }
  pub fn eval(&self, target: &target::Target) -> CppBuildConfigData {
    let mut data = CppBuildConfigData::default();
    for item in &self.items {
      if item.condition.eval(target) {
        data.add_from(&item.data);
      }
    }
    data
  }
}

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
