use std::path::PathBuf;
use common::errors::Result;
use cpp_method::CppMethod;
use cpp_data::CppData;
use common::cpp_build_config::CppBuildConfig;

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

#[derive(Default, Debug, Clone)]
pub struct CrateDependency {
  pub name: String,
  pub version: String,
  pub local_path: Option<PathBuf>,
}

/// Information about the generated crate
#[derive(Default, Debug, Clone)]
pub struct CrateProperties {
  /// Name of the crate
  name: String,
  /// Version of the crate (must be in compliance with cargo requirements)
  version: String,
  /// Authors of the crate
  authors: Vec<String>,
  /// Name of the C++ library
  links: Option<String>,
  dependencies: Vec<CrateDependency>,
  build_dependencies: Vec<CrateDependency>,
  remove_default_dependencies: bool,
  remove_default_build_dependencies: bool,
}

impl CrateProperties {
  pub fn new<S1: Into<String>, S2: Into<String>>(name: S1, version: S2) -> CrateProperties {
    CrateProperties {
      name: name.into(),
      version: version.into(),
      authors: Vec::new(),
      links: None,
      dependencies: Vec::new(),
      build_dependencies: Vec::new(),
      remove_default_dependencies: false,
      remove_default_build_dependencies: false,
    }
  }

  pub fn add_author<S: Into<String>>(&mut self, author: S) {
    self.authors.push(author.into());
  }
  pub fn set_links_attribute<S: Into<String>>(&mut self, links: S) {
    self.links = Some(links.into());
  }
  pub fn add_dependency<S1: Into<String>, S2: Into<String>>(&mut self,
                                                            name: S1,
                                                            version: S2,
                                                            local_path: Option<PathBuf>) {
    self.dependencies.push(CrateDependency {
      name: name.into(),
      version: version.into(),
      local_path: local_path,
    });
  }
  pub fn add_build_dependency<S1: Into<String>, S2: Into<String>>(&mut self,
                                                                  name: S1,
                                                                  version: S2,
                                                                  local_path: Option<PathBuf>) {
    self.build_dependencies.push(CrateDependency {
      name: name.into(),
      version: version.into(),
      local_path: local_path,
    });
  }
  pub fn remove_default_dependencies(&mut self) {
    self.remove_default_dependencies = true;
  }
  pub fn remove_default_build_dependencies(&mut self) {
    self.remove_default_build_dependencies = true;
  }

  pub fn name(&self) -> &String {
    &self.name
  }
  pub fn version(&self) -> &String {
    &self.version
  }
  pub fn authors(&self) -> &Vec<String> {
    &self.authors
  }
  pub fn links_attribute(&self) -> Option<&String> {
    self.links.as_ref()
  }
  pub fn dependencies(&self) -> &Vec<CrateDependency> {
    &self.dependencies
  }
  pub fn build_dependencies(&self) -> &Vec<CrateDependency> {
    &self.build_dependencies
  }
  pub fn should_remove_default_dependencies(&self) -> bool {
    self.remove_default_dependencies
  }
  pub fn should_remove_default_build_dependencies(&self) -> bool {
    self.remove_default_build_dependencies
  }
}





/// The starting point of `cpp_to_rust` API.
/// Create a `Config` object, set its properties,
/// add custom functions if necessary, and start
/// the processing with `Config::exec`.
#[derive(Debug)]
pub struct Config {
  // see documentation for setters
  crate_properties: CrateProperties,
  output_dir_path: PathBuf,
  cache_dir_path: PathBuf,
  crate_template_path: Option<PathBuf>,
  dependency_cache_paths: Vec<PathBuf>,
  include_paths: Vec<PathBuf>,
  framework_paths: Vec<PathBuf>,
  target_include_paths: Vec<PathBuf>,
  include_directives: Vec<PathBuf>,
  cpp_parser_flags: Vec<String>,
  cpp_parser_blocked_names: Vec<String>,
  cpp_ffi_generator_filters: Vec<CppFfiGeneratorFilter>,
  cpp_data_filters: Vec<CppDataFilter>,
  cpp_build_config: CppBuildConfig, // TODO: add CppBuildPaths when needed
  write_dependencies_local_paths: bool,
}

impl Config {
  /// Creates a `Config`.
  /// `crate_properties` are used in Cargo.toml of the generated crate.
  /// `output_dir_path` will contain the generated crate.
  /// `cache_dir_path` will be used for cache, temporary files and
  /// inter-library information files.
  pub fn new<P1: Into<PathBuf>, P2: Into<PathBuf>>(output_dir_path: P1,
                                                   cache_dir_path: P2,
                                                   crate_properties: CrateProperties)
                                                   -> Config {
    Config {
      crate_properties: crate_properties,
      output_dir_path: output_dir_path.into(),
      cache_dir_path: cache_dir_path.into(),
      crate_template_path: Default::default(),
      dependency_cache_paths: Default::default(),
      include_paths: Default::default(),
      framework_paths: Default::default(),
      target_include_paths: Default::default(),
      include_directives: Default::default(),
      cpp_parser_flags: Default::default(),
      cpp_parser_blocked_names: Default::default(),
      cpp_ffi_generator_filters: Default::default(),
      cpp_data_filters: Default::default(),
      cpp_build_config: Default::default(),
      write_dependencies_local_paths: true,
    }
  }

  /// Sets the directory containing additional Rust code for the crate.
  pub fn set_crate_template_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.crate_template_path = Some(path.into());
  }

  /// Sets list of paths to cache directories of processed dependencies.
  pub fn set_dependency_cache_paths(&mut self, paths: Vec<PathBuf>) {
    self.dependency_cache_paths = paths;
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


  /// Adds path to an include directory.
  /// It's supplied to the C++ parser via `-I` option.
  pub fn add_include_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.include_paths.push(path.into());
  }

  /// Adds path to a framework directory (OS X specific).
  /// It's supplied to the C++ parser via `-F` option.
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

  pub fn set_cpp_build_config(&mut self, cpp_build_config: CppBuildConfig) {
    self.cpp_build_config = cpp_build_config;
  }

  pub fn cpp_build_config_mut(&mut self) -> &mut CppBuildConfig {
    &mut self.cpp_build_config
  }

  /// Starts execution of the generator.
  /// This function will print the necessary build script output to stdout.
  /// It also displays some debugging output that can be made visible by
  /// running cargo commands with `-vv` option.
  ///
  /// The result of this function must be checked. You can use
  /// `::errors::fancy_unwrap` to check the result and display
  /// additional error information.
  pub fn exec(self) -> Result<()> {
    ::launcher::run(self)
  }

  pub fn crate_properties(&self) -> &CrateProperties {
    &self.crate_properties
  }

  pub fn output_dir_path(&self) -> &PathBuf {
    &self.output_dir_path
  }

  pub fn cache_dir_path(&self) -> &PathBuf {
    &self.cache_dir_path
  }

  pub fn crate_template_path(&self) -> Option<&PathBuf> {
    self.crate_template_path.as_ref()
  }

  pub fn dependency_cache_paths(&self) -> &[PathBuf] {
    &self.dependency_cache_paths
  }

  pub fn cpp_parser_blocked_names(&self) -> &[String] {
    &self.cpp_parser_blocked_names
  }

  pub fn cpp_parser_flags(&self) -> &[String] {
    &self.cpp_parser_flags
  }

  pub fn include_paths(&self) -> &[PathBuf] {
    &self.include_paths
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

  pub fn cpp_build_config(&self) -> &CppBuildConfig {
    &self.cpp_build_config
  }

  pub fn set_write_dependencies_local_paths(&mut self, value: bool) {
    self.write_dependencies_local_paths = value;
  }
  pub fn write_dependencies_local_paths(&self) -> bool {
    self.write_dependencies_local_paths
  }
}

pub use launcher::{is_completed, completed_marker_path};
