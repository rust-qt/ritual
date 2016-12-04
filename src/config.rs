use std::path::PathBuf;
use errors::Result;
use cpp_method::CppMethod;
use cpp_data::CppData;

/// Function type used in `Config::add_cpp_ffi_generator_filter`.
pub type CppFfiGeneratorFilterFn = Fn(&CppMethod) -> Result<bool>;

struct CppFfiGeneratorFilter(Box<CppFfiGeneratorFilterFn>);

impl ::std::fmt::Debug for CppFfiGeneratorFilter {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
    write!(f, "CppFfiGeneratorFilter")
  }
}

/// Function type used in `Config::add_cpp_data_filter`.
pub type CppDataFilterFn = Fn(&mut CppData) -> Result<()>;

struct CppDataFilter(Box<CppDataFilterFn>);

impl ::std::fmt::Debug for CppDataFilter {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
    write!(f, "CppDataFilter")
  }
}

/// Information about the generated crate
#[derive(Default, Debug, Clone)]
pub struct CrateProperties {
  /// Name of the crate
  pub name: String,
  /// Version of the crate (must be in compliance with cargo requirements)
  pub version: String,
  /// Authors of the crate
  pub authors: Vec<String>,
  /// Name of the C++ library
  pub links: Option<String>,
}


/// The starting point of `cpp_to_rust` API.
/// Create a `Config` object, set its properties,
/// add custom functions if necessary, and start
/// the processing with `Config::exec`.
#[derive(Default, Debug)]
pub struct Config {
  // see documentation for setters
  crate_properties: Option<CrateProperties>,
  output_dir_path: Option<PathBuf>,
  cache_dir_path: Option<PathBuf>,
  crate_template_path: Option<PathBuf>,
  dependency_paths: Vec<PathBuf>,
  linked_libs: Vec<String>,
  lib_paths: Vec<PathBuf>,
  linked_frameworks: Vec<String>,
  framework_paths: Vec<PathBuf>,
  include_paths: Vec<PathBuf>,
  target_include_paths: Vec<PathBuf>,
  include_directives: Vec<PathBuf>,
  cpp_parser_flags: Vec<String>,
  cpp_compiler_flags: Vec<String>,
  cpp_parser_blocked_names: Vec<String>,
  cpp_ffi_generator_filters: Vec<CppFfiGeneratorFilter>,
  cpp_data_filters: Vec<CppDataFilter>,
}

impl Config {
  /// Creates an empty `Config`.
  pub fn new() -> Config {
    Config::default()
  }

  /// Sets properties for Cargo.toml of the generated crate.
  pub fn set_crate_properties(&mut self, value: CrateProperties) {
    self.crate_properties = Some(value);
  }

  /// Sets the directory where new crate will be generated.
  pub fn set_output_dir_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.output_dir_path = Some(path.into());
  }

  /// Sets the directory for temporary files and cache.
  pub fn set_cache_dir_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.cache_dir_path = Some(path.into());
  }

  /// Sets the directory containing additional Rust code for the crate.
  pub fn set_crate_template_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.crate_template_path = Some(path.into());
  }

  /// Sets list of paths to cache directories of processed dependencies.
  pub fn set_dependency_paths(&mut self, paths: Vec<PathBuf>) {
    self.dependency_paths = paths;
  }

  /// Adds a library for linking. Used as `-l` option to the linker.
  pub fn add_linked_lib<P: Into<String>>(&mut self, lib: P) {
    self.linked_libs.push(lib.into());
  }

  /// Adds a framework for linking (OS X specific). Used as `-f` option to the linker.
  pub fn add_linked_framework<P: Into<String>>(&mut self, lib: P) {
    self.linked_frameworks.push(lib.into());
  }

  /// Adds a C++ identifier that should be skipped
  /// by the C++ parser. Identifier can contain namespaces
  /// and nested classes, with `::` separator (like in
  /// C++ identifiers). Identifier may refer to a method,
  /// a class, a enum or a namespace. All entities inside blacklisted
  /// entity (e.g. the methods of a blocked class or
  /// the contents of a blocked namespace)
  /// will also be skipped.
  /// All class methods with names matching the blocked name
  /// will be skipped, regardless of class name.
  pub fn add_cpp_parser_blocked_name<P: Into<String>>(&mut self, lib: P) {
    self.cpp_parser_blocked_names.push(lib.into());
  }

  /// Adds multiple blocked names. See `Config::add_cpp_parser_blocked_name`.
  pub fn add_cpp_parser_blocked_names<Item, Iter>(&mut self, items: Iter)
    where Item: Into<String>,
          Iter: IntoIterator<Item = Item>
  {
    for item in items {
      self.cpp_parser_blocked_names.push(item.into());
    }
  }

  /// Adds a command line argument for clang C++ parser.
  pub fn add_cpp_parser_flag<P: Into<String>>(&mut self, lib: P) {
    self.cpp_parser_flags.push(lib.into());
  }

  /// Adds multiple flags. See `Config::add_cpp_parser_flag`.
  pub fn add_cpp_parser_flags<Item, Iter>(&mut self, items: Iter)
    where Item: Into<String>,
          Iter: IntoIterator<Item = Item>
  {
    for item in items {
      self.cpp_parser_flags.push(item.into());
    }
  }

  /// Adds a command line argument for the C++ compiler.
  pub fn add_cpp_compiler_flag<P: Into<String>>(&mut self, lib: P) {
    self.cpp_compiler_flags.push(lib.into());
  }

  /// Adds multiple flags. See `Config::add_cpp_compiler_flag`.
  pub fn add_cpp_compiler_flags<Item, Iter>(&mut self, items: Iter)
    where Item: Into<String>,
          Iter: IntoIterator<Item = Item>
  {
    for item in items {
      self.cpp_compiler_flags.push(item.into());
    }
  }

  /// Adds path to an include directory.
  /// It's supplied to the C++ parser
  /// and the C++ compiler via `-I` option.
  pub fn add_include_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.include_paths.push(path.into());
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

  /// Adds path to an include directory or an include file
  /// of the target library.
  /// Any C++ types and methods will be parsed and used only
  /// if they are declared within one of files or directories
  /// added with this method.
  ///
  /// If no target include paths are added, all types and methods
  /// will be used. Most libraries include system headers and
  /// other libraries' header files, so this mode is often unwanted.
  pub fn add_target_include_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.target_include_paths.push(path.into());
  }

  /// Adds an include directive. Each directive will be added
  /// as `#include <path>` to the input file for the C++ parser.
  /// Relative paths should be used in this method.
  pub fn add_include_directive<P: Into<PathBuf>>(&mut self, path: P) {
    self.include_directives.push(path.into());
  }

  /// Adds a custom function that decides whether a C++ method should be
  /// added to the C++ wrapper library. For each C++ method,
  /// each function will be run once. Filters are executed in the same order they
  /// were added.
  ///
  /// Interpetation of the function's output:
  ///
  /// - `Err` indicates an unexpected failure and terminates the processing.
  /// - `Ok(true)` allows to continue processing of the method.
  /// If all functions return `Ok(true)`, the method is accepted.
  /// - `Ok(false)` blocks the method. Remaining filter functions are not run
  /// on this method.
  pub fn add_cpp_ffi_generator_filter(&mut self, f: Box<CppFfiGeneratorFilterFn>) {
    self.cpp_ffi_generator_filters.push(CppFfiGeneratorFilter(f));
  }

  /// Adds a custom function that visits `&mut CppData` and can perform any changes
  /// in the output of the C++ parser. Filters are executed in the same order they
  /// were added. If the function returns `Err`, the processing is terminated.
  pub fn add_cpp_data_filter(&mut self, f: Box<CppDataFilterFn>) {
    self.cpp_data_filters.push(CppDataFilter(f));
  }

  /// Starts execution of the generator.
  /// This function will print the necessary build script output to stdout.
  /// It also displays some debugging output that can be made visible by
  /// running cargo commands with `-vv` option.
  ///
  /// The result of this function must be checked. The recommended way:
  ///
  /// ```rust,should_panic
  /// # let config = cpp_to_rust::config::Config::new();
  /// if let Err(err) = config.exec() {
  ///   err.display_report();
  ///   std::process::exit(1);
  /// }
  /// ```
  pub fn exec(self) -> Result<()> {
    ::launcher::run(self)
  }

  pub fn crate_properties(&self) -> &CrateProperties {
    self.crate_properties.as_ref().expect("crate_properties must be set")
  }

  pub fn output_dir_path(&self) -> &PathBuf {
    self.output_dir_path.as_ref().expect("output_dir_path must be set")
  }

  pub fn cache_dir_path(&self) -> &PathBuf {
    self.cache_dir_path.as_ref().expect("cache_dir_path must be set")
  }

  pub fn crate_template_path(&self) -> Option<&PathBuf> {
    self.crate_template_path.as_ref()
  }

  pub fn dependency_paths(&self) -> &[PathBuf] {
    &self.dependency_paths
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

  pub fn cpp_parser_flags(&self) -> &[String] {
    &self.cpp_parser_flags
  }

  pub fn cpp_compiler_flags(&self) -> &[String] {
    &self.cpp_compiler_flags
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

pub use launcher::is_completed;
