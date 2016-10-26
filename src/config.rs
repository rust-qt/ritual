use std::path::PathBuf;
use errors::Result;
use cpp_method::CppMethod;
use cpp_data::CppData;

pub type CppFfiGeneratorFilterFn = Fn(&CppMethod) -> Result<bool>;

struct CppFfiGeneratorFilter(Box<CppFfiGeneratorFilterFn>);

impl ::std::fmt::Debug for CppFfiGeneratorFilter {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
    write!(f, "CppFfiGeneratorFilter")
  }
}

pub type CppDataFilterFn = Fn(&mut CppData) -> Result<()>;

struct CppDataFilter(Box<CppDataFilterFn>);

impl ::std::fmt::Debug for CppDataFilter {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
    write!(f, "CppDataFilter")
  }
}



#[derive(Default, Debug)]
pub struct Config {
  /// Extra libraries to be linked.
  /// Used as "-l" option to linker.
  linked_libs: Vec<String>,
  /// Paths to library directories supplied to the linker
  /// via '-L' option or environment variables.
  lib_paths: Vec<PathBuf>,

  /// Extra frameworks to be linked (MacOS specific).
  /// Used as "-f" option to linker.
  linked_frameworks: Vec<String>,

  /// Paths to framework directories supplied to the linker
  /// via '-F' option or environment variables.
  framework_paths: Vec<PathBuf>,

  /// Paths to include directories supplied to the C++ parser
  /// and compiler of the C++ wrapper via '-I' option.
  /// This is detected automatically for Qt libraries using qmake.
  /// Paths can be relative to lib spec file's directory.
  include_paths: Vec<PathBuf>,

  /// Paths to include directories of the library.
  /// Only types and methods declared within these directories
  /// will be parsed.
  /// This is detected automatically for Qt libraries using qmake.
  /// Paths can be relative to lib spec file's directory.
  target_include_paths: Vec<PathBuf>,
  // TODO: allow both dirs and files in target_include_paths
  /// Name of the library's include file
  include_directives: Vec<PathBuf>,

  /// List of C++ identifiers which should be skipped
  /// by C++ parser. Identifier can contain namespaces
  /// and nested classes, with "::" separator (like in
  /// C++ identifiers). Identifier may refer to a method,
  /// a class, a enum or a namespace. All entities inside blacklisted
  /// entity (e.g. the methods of a blocked class or
  /// the contents of a blocked namespace)
  /// will also be skipped.
  cpp_parser_blocked_names: Vec<String>,

  /// Custom function that decides whether a C++ method should be
  /// added to FFI library wrapper. Err indicates an unexpected failure
  /// and terminates processing. Ok(true) allows the method, and
  /// Ok(false) blocks the method.
  cpp_ffi_generator_filters: Vec<CppFfiGeneratorFilter>,
  cpp_data_filters: Vec<CppDataFilter>,
}

impl Config {
  /// Creates empty Config
  pub fn new() -> Config {
    Config::default()
  }

  pub fn add_linked_lib<P: Into<String>>(&mut self, lib: P) {
    self.linked_libs.push(lib.into());
  }

  pub fn add_linked_framework<P: Into<String>>(&mut self, lib: P) {
    self.linked_frameworks.push(lib.into());
  }

  pub fn add_cpp_parser_blocked_name<P: Into<String>>(&mut self, lib: P) {
    self.cpp_parser_blocked_names.push(lib.into());
  }

  pub fn add_cpp_parser_blocked_names<Item, Iter>(&mut self, items: Iter)
    where Item: Into<String>,
          Iter: IntoIterator<Item = Item>
  {
    for item in items {
      self.cpp_parser_blocked_names.push(item.into());
    }
  }

  pub fn add_include_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.include_paths.push(path.into());
  }

  pub fn add_lib_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.lib_paths.push(path.into());
  }

  pub fn add_framework_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.framework_paths.push(path.into());
  }

  pub fn add_target_include_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.target_include_paths.push(path.into());
  }

  pub fn add_include_directive<P: Into<PathBuf>>(&mut self, path: P) {
    self.include_directives.push(path.into());
  }

  pub fn add_cpp_ffi_generator_filter(&mut self, f: Box<CppFfiGeneratorFilterFn>) {
    self.cpp_ffi_generator_filters.push(CppFfiGeneratorFilter(f));
  }

  pub fn add_cpp_data_filter(&mut self, f: Box<CppDataFilterFn>) {
    self.cpp_data_filters.push(CppDataFilter(f));
  }

  pub fn exec(self) -> Result<()> {
    ::launcher::run_from_build_script(self)
  }

  pub fn linked_libs(&self) -> &[String] {
    &self.linked_libs
  }

  pub fn linked_frameworks(&self) -> &[String] {
    &self.linked_frameworks
  }

  pub fn cpp_parser_blocked_names(&self) -> &[String] {
    &self.cpp_parser_blocked_names
  }

  pub fn include_paths(&self) -> &[PathBuf] {
    &self.include_paths
  }

  pub fn lib_paths(&self) -> &[PathBuf] {
    &self.lib_paths
  }

  pub fn framework_paths(&self) -> &[PathBuf] {
    &self.framework_paths
  }

  pub fn target_include_paths(&self) -> &[PathBuf] {
    &self.target_include_paths
  }

  pub fn include_directives(&self) -> &[PathBuf] {
    &self.include_directives
  }

  pub fn cpp_ffi_generator_filters(&self) -> Vec<&Box<CppFfiGeneratorFilterFn>> {
    self.cpp_ffi_generator_filters.iter().map(|x| &x.0).collect()
  }

  pub fn cpp_data_filters(&self) -> Vec<&Box<CppDataFilterFn>> {
    self.cpp_data_filters.iter().map(|x| &x.0).collect()
  }
}
