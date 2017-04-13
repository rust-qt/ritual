use std::path::PathBuf;
use common::errors::Result;
use cpp_method::CppMethod;
use cpp_data::CppData;
pub use cpp_data::CppTypeAllocationPlace;
use common::cpp_build_config::CppBuildConfig;
use std::collections::HashMap;

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

/// Information about an extra non-`cpp_to_rust`-based dependency.
#[derive(Default, Debug, Clone)]
pub struct CrateDependency {
  name: String,
  version: String,
  local_path: Option<PathBuf>,
}

impl CrateDependency {
  /// Name of the crate (as in `Cargo.toml`)
  pub fn name(&self) -> &str {
    &self.name
  }
  /// Version of the crate (as in `Cargo.toml`)
  pub fn version(&self) -> &str {
    &self.version
  }
  /// Local path to the dependency (if present).
  pub fn local_path(&self) -> Option<&PathBuf> {
    self.local_path.as_ref()
  }
}

/// Information about the crate being generated.
/// Most of information in this object will be used in
/// the output `Cargo.toml`.
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
  /// Extra dependencies for output `Cargo.toml`
  dependencies: Vec<CrateDependency>,
  /// Extra build dependencies for output `Cargo.toml`
  build_dependencies: Vec<CrateDependency>,
  /// Don't add default dependencies to `Cargo.toml`
  remove_default_dependencies: bool,
  /// Don't add default build dependencies to `Cargo.toml`
  remove_default_build_dependencies: bool,
}

impl CrateProperties {
  /// Creates a new object with `name` and `version`.
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

  /// Adds an author for `authors` field of `Cargo.toml`.
  pub fn add_author<S: Into<String>>(&mut self, author: S) {
    self.authors.push(author.into());
  }
  /// Sets value for `links` field of `Cargo.toml`.
  /// Native name of the C++ library should be used here.
  pub fn set_links_attribute<S: Into<String>>(&mut self, links: S) {
    self.links = Some(links.into());
  }
  /// Adds an extra non-`cpp_to_rust`-based dependency with
  /// `name`, `version` and optionally `local_path`.
  pub fn add_dependency<S1: Into<String>, S2: Into<String>>(&mut self,
                                                            name: S1,
                                                            version: S2,
                                                            local_path: Option<PathBuf>) {
    self
      .dependencies
      .push(CrateDependency {
              name: name.into(),
              version: version.into(),
              local_path: local_path,
            });
  }
  /// Adds an extra build dependency with
  /// `name`, `version` and optionally `local_path`.
  pub fn add_build_dependency<S1: Into<String>, S2: Into<String>>(&mut self,
                                                                  name: S1,
                                                                  version: S2,
                                                                  local_path: Option<PathBuf>) {
    self
      .build_dependencies
      .push(CrateDependency {
              name: name.into(),
              version: version.into(),
              local_path: local_path,
            });
  }
  /// Removes default dependencies from output `Cargo.toml`. Default
  /// dependencies are `libc`, `cpp_utils` and crates added using
  /// `Config::set_dependency_cache_paths`.
  pub fn remove_default_dependencies(&mut self) {
    self.remove_default_dependencies = true;
  }
  /// Removes default build dependencies from output `Cargo.toml`. Default
  /// build dependency is `cpp_to_rust_build_tools`.
  pub fn remove_default_build_dependencies(&mut self) {
    self.remove_default_build_dependencies = true;
  }

  /// Name of the crate
  pub fn name(&self) -> &String {
    &self.name
  }
  /// Version of the crate
  pub fn version(&self) -> &String {
    &self.version
  }
  /// Authors of the crate
  pub fn authors(&self) -> &Vec<String> {
    &self.authors
  }
  /// `links` property of the crate
  pub fn links_attribute(&self) -> Option<&String> {
    self.links.as_ref()
  }
  /// Extra non-`cpp_to_rust`-based dependencies of the crate
  pub fn dependencies(&self) -> &Vec<CrateDependency> {
    &self.dependencies
  }
  /// Extra build dependencies of the crate
  pub fn build_dependencies(&self) -> &Vec<CrateDependency> {
    &self.build_dependencies
  }
  /// Returns true if default dependencies were removed.
  pub fn should_remove_default_dependencies(&self) -> bool {
    self.remove_default_dependencies
  }
  /// Returns true if default build dependencies were removed.
  pub fn should_remove_default_build_dependencies(&self) -> bool {
    self.remove_default_build_dependencies
  }
}

/// Value of this enum determines how `cpp_to_rust` uses
/// data from the cache directory accumulated in a previous processing
/// of the same library.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CacheUsage {
  /// The most aggressive caching. The generator will use all cached
  /// data and may even completely skip processing if it was completed before.
  Full,
  /// The generator will use raw or processed C++ data if possible.
  CppDataOnly,
  /// The generator will use raw C++ data if possible.
  RawCppDataOnly,
  /// No cached data will be used.
  None,
}

impl CacheUsage {
  /// Returns true if raw C++ data file can be used in this mode.
  pub fn can_use_raw_cpp_data(&self) -> bool {
    self != &CacheUsage::None
  }
  /// Returns true if processed C++ data file can be used in this mode.
  pub fn can_use_cpp_data(&self) -> bool {
    match *self {
      CacheUsage::Full | CacheUsage::CppDataOnly => true,
      _ => false,
    }
  }
  /// Returns true if this mode allows to skip processing completely.
  pub fn can_skip_all(&self) -> bool {
    self == &CacheUsage::Full
  }
}

impl Default for CacheUsage {
  fn default() -> CacheUsage {
    CacheUsage::Full
  }
}

/// Value of this enum determines how extra logging information
/// will be used.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DebugLoggingConfig {
  /// Output debug logs to stderr (noisy).
  Print,
  /// Save debug logs to `log` subdirectory of the cache directory.
  SaveToFile,
  /// Disable debug logs.
  Disable,
}

impl Default for DebugLoggingConfig {
  fn default() -> DebugLoggingConfig {
    DebugLoggingConfig::Disable
  }
}

/// The starting point of `cpp_to_rust` API.
/// Create a `Config` object, set its properties,
/// add custom functions if necessary, and start
/// the processing with `Config::exec`.
#[derive(Debug)]
pub struct Config {
  // see setters documentation for information about these properties
  cache_usage: CacheUsage,
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
  type_allocation_places: HashMap<String, CppTypeAllocationPlace>,
  debug_logging_config: DebugLoggingConfig,
  quiet_mode: bool,
  write_cache: bool,
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
      type_allocation_places: Default::default(),
      write_dependencies_local_paths: true,
      cache_usage: CacheUsage::default(),
      debug_logging_config: DebugLoggingConfig::default(),
      quiet_mode: false,
      write_cache: true,
    }
  }

  /// Returns true if a completion marker exists in the cache directory,
  /// indicating that processing of the library in this cache directory
  /// was completed before. Note that the marker is not created if
  /// `write_cache` was set to false.
  pub fn is_completed(&self) -> bool {
    ::launcher::is_completed(&self.cache_dir_path)
  }
  /// Returns path to the completion marker file
  /// indicating that processing of the library in this cache directory
  /// was completed before.
  pub fn completed_marker_path(&self) -> PathBuf {
    ::launcher::completed_marker_path(&self.cache_dir_path)
  }
  /// Defines how cached data is used in repeated runs of the generator.
  pub fn set_cache_usage(&mut self, value: CacheUsage) {
    self.cache_usage = value;
  }

  /// Sets the directory containing additional Rust code for the crate.
  pub fn set_crate_template_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.crate_template_path = Some(path.into());
  }

  /// Sets list of paths to cache directories of processed dependencies.
  /// The generator will integrate API of the current library with its
  /// dependencies and re-use their types.
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
  ///
  /// Note that this value is not used when building the wrapper library.
  /// Use `Config::cpp_build_config_mut` or a similar method to
  /// configure building the wrapper library.
  pub fn add_cpp_parser_flag<P: Into<String>>(&mut self, lib: P) {
    self.cpp_parser_flags.push(lib.into());
  }

  /// Adds multiple command line arguments for clang C++ parser.
  /// See `Config::add_cpp_parser_flag`.
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
  ///
  /// Note that this value is not used when building the wrapper library.
  /// Use `Config::cpp_build_config_mut` or a similar method to
  /// configure building the wrapper library.
  pub fn add_include_path<P: Into<PathBuf>>(&mut self, path: P) {
    self.include_paths.push(path.into());
  }

  /// Adds path to a framework directory (OS X specific).
  /// It's supplied to the C++ parser via `-F` option.
  ///
  /// Note that this value is not used when building the wrapper library.
  /// Use `Config::cpp_build_config_mut` or a similar method to
  /// configure building the wrapper library.
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
  /// File name only paths or relative paths should be used in this method.
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
    self
      .cpp_ffi_generator_filters
      .push(CppFfiGeneratorFilter(f));
  }

  /// Adds a custom function that visits `&mut CppData` and can perform any changes
  /// in the output of the C++ parser. Filters are executed in the same order they
  /// were added. If the function returns `Err`, the processing is terminated.
  pub fn add_cpp_data_filter(&mut self, f: Box<CppDataFilterFn>) {
    self.cpp_data_filters.push(CppDataFilter(f));
  }

  /// Overrides automatic selection of type allocation place for `type_name` and uses `place`
  /// instead. See `CppTypeAllocationPlace` for more information.
  pub fn set_type_allocation_place<S: Into<String>>(&mut self,
                                                    place: CppTypeAllocationPlace,
                                                    type_name: S) {
    self
      .type_allocation_places
      .insert(type_name.into(), place);
  }
  /// Overrides automatic selection of type allocation place for `types` and uses `place`
  /// instead. See also `Config::set_type_allocation_place`.
  pub fn set_types_allocation_place<SI, S>(&mut self, place: CppTypeAllocationPlace, types: SI)
    where SI: IntoIterator<Item = S>,
          S: Into<String>
  {
    for t in types {
      self
        .type_allocation_places
        .insert(t.into(), place.clone());
    }
  }

  /// Changes how debug logs are handled.
  pub fn set_debug_logging_config(&mut self, config: DebugLoggingConfig) {
    self.debug_logging_config = config;
  }

  /// Sets quiet mode. In quiet mode status messages and debug logs are
  /// redirected to `log` subdirectory of the cache directory. Only error messages
  /// are always written to stderr. Quiet mode is disabled by default.
  pub fn set_quiet_mode(&mut self, quiet_mode: bool) {
    self.quiet_mode = quiet_mode;
  }

  /// Sets writing to cache mode. If enabled, result of processing is saved to
  /// extra files in cache directory. The main use of these files are loading
  /// dependency data. They are also used for speeding up repeated runs of the generator
  /// on the same library.
  ///
  /// Writing to cache is enabled by default. If the library is not intended to be used as
  /// a dependency when running the generator on another library, it's safe to disable
  /// writing to cache in order to speed up the generator.
  pub fn set_write_cache(&mut self, write_cache: bool) {
    self.write_cache = write_cache;
  }

  /// Sets `CppBuildConfig` value that will be passed to the build script
  /// of the generated crate.
  pub fn set_cpp_build_config(&mut self, cpp_build_config: CppBuildConfig) {
    self.cpp_build_config = cpp_build_config;
  }

  /// Allows to change `CppBuildConfig` value that will be passed to the build script
  /// of the generated crate.
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

  /// Returns value set by `Config::set_cache_usage`.
  pub fn cache_usage(&self) -> &CacheUsage {
    &self.cache_usage
  }

  /// Returns crate properties passed to `Config::new`.
  pub fn crate_properties(&self) -> &CrateProperties {
    &self.crate_properties
  }

  /// Returns path to the output directory passed to `Config::new`.
  pub fn output_dir_path(&self) -> &PathBuf {
    &self.output_dir_path
  }

  /// Returns path to the cache directory passed to `Config::new`.
  pub fn cache_dir_path(&self) -> &PathBuf {
    &self.cache_dir_path
  }

  /// Returns value set by `Config::set_crate_template_path`.
  pub fn crate_template_path(&self) -> Option<&PathBuf> {
    self.crate_template_path.as_ref()
  }

  /// Returns value set by `Config::set_dependency_cache_paths`.
  pub fn dependency_cache_paths(&self) -> &[PathBuf] {
    &self.dependency_cache_paths
  }

  /// Returns names added with `Config::add_cpp_parser_blocked_name`
  /// and similar methods.
  pub fn cpp_parser_blocked_names(&self) -> &[String] {
    &self.cpp_parser_blocked_names
  }

  /// Returns names added with `Config::add_cpp_parser_flag`
  /// and similar methods.
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
    self
      .cpp_ffi_generator_filters
      .iter()
      .map(|x| &x.0)
      .collect()
  }

  pub fn cpp_data_filters(&self) -> Vec<&Box<CppDataFilterFn>> {
    self.cpp_data_filters.iter().map(|x| &x.0).collect()
  }

  pub fn cpp_build_config(&self) -> &CppBuildConfig {
    &self.cpp_build_config
  }
  pub fn type_allocation_places(&self) -> &HashMap<String, CppTypeAllocationPlace> {
    &self.type_allocation_places
  }

  pub fn set_write_dependencies_local_paths(&mut self, value: bool) {
    self.write_dependencies_local_paths = value;
  }
  pub fn write_dependencies_local_paths(&self) -> bool {
    self.write_dependencies_local_paths
  }
  pub fn debug_logging_config(&self) -> &DebugLoggingConfig {
    &self.debug_logging_config
  }
  pub fn quiet_mode(&self) -> bool {
    self.quiet_mode
  }
  pub fn write_cache(&self) -> bool {
    self.write_cache
  }
}

pub use launcher::{is_completed, completed_marker_path};
