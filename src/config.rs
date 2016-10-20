use std::path::PathBuf;
use errors::Result;
use cpp_method::CppMethod;

pub type CppFfiGeneratorFilterFn = Fn(&CppMethod) -> Result<bool>;

#[derive(Default)]
struct CppFfiGeneratorFilter(Option<Box<CppFfiGeneratorFilterFn>>);

impl ::std::fmt::Debug for CppFfiGeneratorFilter {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
    write!(f, "{}", if self.0.is_some() { "Some(fn)" } else { "None" })
  }
}

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

  /// List of C++ identifiers which should be skipped
  /// by C++ parser. Identifier can contain namespaces
  /// and nested classes, with "::" separator (like in
  /// C++ identifiers). Identifier may refer to a method,
  /// a class, a enum or a namespace. All entities inside blacklisted
  /// entity (e.g. the methods of a blocked class or
  /// the contents of a blocked namespace)
  /// will also be skipped.
  cpp_parser_blocked_names: Vec<String>,

  cpp_ffi_generator_filter: CppFfiGeneratorFilter,
}

impl Config {
  /// Creates empty Config
  pub fn new() -> Config {
    Config::default()
  }

  pub fn add_extra_lib<P: Into<String>>(&mut self, lib: P) {
    self.extra_libs.push(lib.into());
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

  pub fn add_target_include_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.target_include_paths.push(path.into());
  }

  pub fn add_include_directive<P: Into<PathBuf>>(&mut self, path: P) {
    self.include_directives.push(path.into());
  }

  pub fn set_cpp_ffi_generator_filter(&mut self, f: Box<CppFfiGeneratorFilterFn>) {
    self.cpp_ffi_generator_filter.0 = Some(f);
  }

  pub fn exec(self) -> Result<()> {
    ::launcher::run_from_build_script(self)
  }

  pub fn extra_libs(&self) -> &[String] {
    &self.extra_libs
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

  pub fn target_include_paths(&self) -> &[PathBuf] {
    &self.target_include_paths
  }

  pub fn include_directives(&self) -> &[PathBuf] {
    &self.include_directives
  }

  pub fn cpp_ffi_generator_filter(&self) -> Option<&Box<CppFfiGeneratorFilterFn>> {
    self.cpp_ffi_generator_filter.0.as_ref()
  }
}
