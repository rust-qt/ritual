//! Interface for configuring and running the generator.

use crate::cpp_data::CppPath;
use crate::processor::{ProcessingSteps, ProcessorData};
use crate::rust_info::{NameType, RustPathScope};
use crate::rust_type::RustPath;
use ritual_common::cpp_build_config::{CppBuildConfig, CppBuildPaths};
use ritual_common::errors::Result;
use ritual_common::toml;
use std::path::PathBuf;

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
    /// Extra properties to be merged with auto generated content of `Cargo.toml`
    custom_fields: toml::value::Table,
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
    pub fn new<S1: Into<String>, S2: Into<String>>(name: S1, version: S2) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            custom_fields: Default::default(),
            dependencies: Vec::new(),
            build_dependencies: Vec::new(),
            remove_default_dependencies: false,
            remove_default_build_dependencies: false,
        }
    }

    /// Adds an extra non-`cpp_to_rust`-based dependency with
    /// `name`, `version` and optionally `local_path`.
    pub fn add_dependency<S1: Into<String>, S2: Into<String>>(
        &mut self,
        name: S1,
        version: S2,
        local_path: Option<PathBuf>,
    ) {
        self.dependencies.push(CrateDependency {
            name: name.into(),
            version: version.into(),
            local_path,
        });
    }
    /// Adds an extra build dependency with
    /// `name`, `version` and optionally `local_path`.
    pub fn add_build_dependency<S1: Into<String>, S2: Into<String>>(
        &mut self,
        name: S1,
        version: S2,
        local_path: Option<PathBuf>,
    ) {
        self.build_dependencies.push(CrateDependency {
            name: name.into(),
            version: version.into(),
            local_path,
        });
    }
    /// Removes default dependencies from output `Cargo.toml`. Default
    /// dependencies are `libc`, `cpp_utils` and crates added using
    /// `Config::set_dependent_cpp_crates`.
    pub fn remove_default_dependencies(&mut self) {
        self.remove_default_dependencies = true;
    }
    /// Removes default build dependencies from output `Cargo.toml`. Default
    /// build dependency is `cpp_to_rust_build_tools`.
    pub fn remove_default_build_dependencies(&mut self) {
        self.remove_default_build_dependencies = true;
    }

    /// Sets custom fields for output `Cargo.toml`. These fields will
    /// be added to auto-generated fields (or replace them in case of a name conflict).
    pub fn set_custom_fields(&mut self, value: toml::value::Table) {
        self.custom_fields = value;
    }

    /// Name of the crate
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Version of the crate
    pub fn version(&self) -> &str {
        &self.version
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

    pub fn custom_fields(&self) -> &toml::value::Table {
        &self.custom_fields
    }
}

pub type RustPathScopeHook = dyn Fn(&CppPath) -> Result<Option<RustPathScope>> + 'static;
pub type RustPathHook =
    dyn Fn(&CppPath, &NameType<'_>, &ProcessorData<'_>) -> Result<Option<RustPath>> + 'static;
pub type AfterCppParserHook = dyn Fn(&mut ProcessorData<'_>) -> Result<()> + 'static;

/// The starting point of `cpp_to_rust` API.
/// Create a `Config` object, set its properties,
/// add custom functions if necessary, and start
/// the processing with `Config::exec`.
pub struct Config {
    // see setters documentation for information about these properties
    crate_properties: CrateProperties,
    cpp_lib_version: Option<String>,
    crate_template_path: Option<PathBuf>,
    dependent_cpp_crates: Vec<String>,
    include_directives: Vec<PathBuf>,
    target_include_paths: Vec<PathBuf>,
    cpp_build_config: CppBuildConfig,
    cpp_build_paths: CppBuildPaths,
    cpp_parser_arguments: Vec<String>,
    processing_steps: ProcessingSteps,
    movable_types_hook: Option<Box<dyn Fn(&CppPath) -> Result<MovableTypesHookOutput>>>,
    cpp_parser_path_hook: Option<Box<dyn Fn(&CppPath) -> Result<bool>>>,
    rust_path_scope_hook: Option<Box<RustPathScopeHook>>,
    rust_path_hook: Option<Box<RustPathHook>>,
    after_cpp_parser_hook: Option<Box<AfterCppParserHook>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovableTypesHookOutput {
    Movable,
    Immovable,
    Unknown,
}

impl Config {
    /// Creates a `Config`.
    /// `crate_properties` are used in Cargo.toml of the generated crate.
    pub fn new(crate_properties: CrateProperties) -> Self {
        Config {
            crate_properties,
            crate_template_path: Default::default(),
            dependent_cpp_crates: Default::default(),
            cpp_build_paths: Default::default(),
            target_include_paths: Default::default(),
            include_directives: Default::default(),
            cpp_parser_arguments: Default::default(),
            cpp_build_config: Default::default(),
            movable_types_hook: Default::default(),
            processing_steps: Default::default(),
            cpp_lib_version: Default::default(),
            cpp_parser_path_hook: Default::default(),
            rust_path_scope_hook: Default::default(),
            rust_path_hook: Default::default(),
            after_cpp_parser_hook: Default::default(),
        }
    }

    /// Sets the directory containing additional files for the crate.
    /// Any files and directories found in the crate template will be copied
    /// to the generated crate's directory, although some of them (such as `Cargo.toml`)
    /// may be overwritten with the generates files. It's common to put `tests` and
    /// `examples` subdirectories in the crate template so that `cargo` recognizes them
    /// automatically in the generated crate.
    ///
    /// If you want to add some extra code
    /// to the generated modules, put `src/module_name.rs` file in the crate template and
    /// add `include_generated!();` line in the file. This line will be replaced with
    /// the generated content. You can also add extra modules as separate files,
    /// but you'll also need to create `src/lib.rs` in the crate template and
    /// declare new module in it using `[pub] mod module_name;`. Use `include_generated!();`
    /// in `src/lib.rs` to include declaration of automatically generated modules.
    ///
    /// If the crate template contains `rustfmt.toml` file, it's used to format the generated
    /// Rust code instead of the default `rustfmt.toml`.
    ///
    /// Creating crate template is optional. The generator can make a crate without a template.
    pub fn set_crate_template_path<P: Into<PathBuf>>(&mut self, path: P) {
        self.crate_template_path = Some(path.into());
    }

    /// Sets list of names of crates created with `cpp_to_rust`.
    /// The generator will integrate API of the current library with its
    /// dependencies and re-use their types.
    pub fn set_dependent_cpp_crates(&mut self, paths: Vec<String>) {
        self.dependent_cpp_crates = paths;
    }

    /// Adds a command line argument for clang C++ parser.
    ///
    /// Note that this value is not used when building the wrapper library.
    /// Use `Config::cpp_build_config_mut` or a similar method to
    /// configure building the wrapper library.
    pub fn add_cpp_parser_argument<P: Into<String>>(&mut self, lib: P) {
        self.cpp_parser_arguments.push(lib.into());
    }

    /// Adds multiple command line arguments for clang C++ parser.
    /// See `Config::add_cpp_parser_argument`.
    pub fn add_cpp_parser_arguments<Item, Iter>(&mut self, items: Iter)
    where
        Item: Into<String>,
        Iter: IntoIterator<Item = Item>,
    {
        for item in items {
            self.cpp_parser_arguments.push(item.into());
        }
    }

    /// Sets `CppBuildPaths` value for this config. These paths
    /// are used for testing C++ methods while processing the library,
    /// but they are not used when building the generated crate.
    pub fn set_cpp_build_paths(&mut self, paths: CppBuildPaths) {
        self.cpp_build_paths = paths;
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

    pub fn set_cpp_lib_version<S: Into<String>>(&mut self, version: S) {
        self.cpp_lib_version = Some(version.into());
    }

    pub fn cpp_lib_version(&self) -> Option<&str> {
        self.cpp_lib_version.as_ref().map(|x| x.as_str())
    }

    pub fn processing_steps(&self) -> &ProcessingSteps {
        &self.processing_steps
    }

    pub fn processing_steps_mut(&mut self) -> &mut ProcessingSteps {
        &mut self.processing_steps
    }

    /// Returns crate properties passed to `Config::new`.
    pub fn crate_properties(&self) -> &CrateProperties {
        &self.crate_properties
    }

    /// Returns value set by `Config::set_crate_template_path`.
    pub fn crate_template_path(&self) -> Option<&PathBuf> {
        self.crate_template_path.as_ref()
    }

    /// Returns value set by `Config::set_dependent_cpp_crates`.
    pub fn dependent_cpp_crates(&self) -> &[String] {
        &self.dependent_cpp_crates
    }

    /// Returns names added with `Config::add_cpp_parser_argument`
    /// and similar methods.
    pub fn cpp_parser_arguments(&self) -> &[String] {
        &self.cpp_parser_arguments
    }

    /// Returns values added by `Config::set_cpp_build_paths`.
    pub fn cpp_build_paths(&self) -> &CppBuildPaths {
        &self.cpp_build_paths
    }

    /// Returns values added by `Config::add_target_include_path`.
    pub fn target_include_paths(&self) -> &[PathBuf] {
        &self.target_include_paths
    }

    /// Returns values added by `Config::add_include_directive`.
    pub fn include_directives(&self) -> &[PathBuf] {
        &self.include_directives
    }

    /// Returns current `CppBuildConfig` value.
    pub fn cpp_build_config(&self) -> &CppBuildConfig {
        &self.cpp_build_config
    }

    pub fn set_movable_types_hook(
        &mut self,
        hook: impl Fn(&CppPath) -> Result<MovableTypesHookOutput> + 'static,
    ) {
        self.movable_types_hook = Some(Box::new(hook));
    }

    pub fn movable_types_hook(
        &self,
    ) -> Option<&(dyn Fn(&CppPath) -> Result<MovableTypesHookOutput> + 'static)> {
        self.movable_types_hook.as_ref().map(|b| &**b)
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
    pub fn set_cpp_parser_path_hook(&mut self, hook: impl Fn(&CppPath) -> Result<bool> + 'static) {
        self.cpp_parser_path_hook = Some(Box::new(hook));
    }

    pub fn cpp_parser_path_hook(&self) -> Option<&(dyn Fn(&CppPath) -> Result<bool> + 'static)> {
        self.cpp_parser_path_hook.as_ref().map(|b| &**b)
    }

    pub fn set_rust_path_scope_hook(
        &mut self,
        hook: impl Fn(&CppPath) -> Result<Option<RustPathScope>> + 'static,
    ) {
        self.rust_path_scope_hook = Some(Box::new(hook));
    }

    pub fn rust_path_scope_hook(&self) -> Option<&RustPathScopeHook> {
        self.rust_path_scope_hook.as_ref().map(|b| &**b)
    }

    pub fn set_rust_path_hook(
        &mut self,
        hook: impl Fn(&CppPath, &NameType<'_>, &ProcessorData<'_>) -> Result<Option<RustPath>> + 'static,
    ) {
        self.rust_path_hook = Some(Box::new(hook));
    }

    pub fn rust_path_hook(&self) -> Option<&RustPathHook> {
        self.rust_path_hook.as_ref().map(|b| &**b)
    }

    pub fn set_after_cpp_parser_hook(
        &mut self,
        hook: impl Fn(&mut ProcessorData<'_>) -> Result<()> + 'static,
    ) {
        self.after_cpp_parser_hook = Some(Box::new(hook));
    }

    pub fn after_cpp_parser_hook(&self) -> Option<&AfterCppParserHook> {
        self.after_cpp_parser_hook.as_ref().map(|b| &**b)
    }
}

#[derive(Default)]
pub struct GlobalConfig {
    create_config_hook: Option<Box<dyn FnMut(&str) -> Result<Config>>>,
    all_crate_names: Vec<String>,
}

impl GlobalConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_create_config_hook<F: FnMut(&str) -> Result<Config> + 'static>(&mut self, f: F) {
        self.create_config_hook = Some(Box::new(f));
    }

    pub fn create_config_hook(
        &mut self,
    ) -> Option<&mut (dyn FnMut(&str) -> Result<Config> + 'static)> {
        self.create_config_hook.as_mut().map(|b| &mut **b)
    }

    pub fn set_all_crate_names(&mut self, names: Vec<String>) {
        self.all_crate_names = names;
    }

    pub fn all_crate_names(&self) -> &[String] {
        &self.all_crate_names
    }
}
