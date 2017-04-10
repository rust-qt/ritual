use errors::Result;

/// CPU architecture, as reported by `target_arch`.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum Arch {
  X86,
  X86_64,
  Mips,
  PowerPC,
  PowerPC64,
  Arm,
  AArch64,
}

/// Operating system, as reported by `target_os`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum OS {
  Windows,
  MacOS,
  IOS,
  Linux,
  Android,
  FreeBSD,
  DragonFly,
  Bitrig,
  OpenBSD,
  NetBSD,
}

/// Operating system family, as reported by `target_family`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum Family {
  Windows,
  Unix,
}

/// Further disambiguates the target platform with information about the ABI/libc,
/// as reported by `target_env`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum Env {
  Gnu,
  Msvc,
  Musl,
  None,
}

/// Pointer width in bits,
/// as reported by `target_pointer_width`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum PointerWidth {
  P64,
  P32,
}

/// CPU endianness, as reported by `target_endian`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum Endian {
  Little,
  Big,
}

/// Combined information about a target, as reported by configuration
/// values of the Rust compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct Target {
  /// CPU architecture
  pub arch: Arch,
  /// Operating system
  pub os: OS,
  /// Operating system family
  pub family: Family,
  /// Further disambiguates the target platform with information about the ABI/libc,
  pub env: Env,
  /// Pointer width in bits,
  pub pointer_width: PointerWidth,
  /// Endianness of the target CPU
  pub endian: Endian,
}

/// Condition on properties of the target. Simple conditions
/// are considered true if the property of the current platform
/// is the same as the associated value of the enum. For
/// example, `Condition::OS(OS::Windows)` will be true on Windows
/// and false otherwise. `And`, `Or` and `Not` variants provide
/// logical operations on nested conditions. `True` and `False`
/// variants provide conditions which are always true and false,
/// respectively.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum Condition {
  Arch(Arch),
  OS(OS),
  Family(Family),
  Env(Env),
  PointerWidth(PointerWidth),
  // Vendor(Vendor),
  Endian(Endian),
  And(Vec<Condition>),
  Or(Vec<Condition>),
  Not(Box<Condition>),
  True,
  False,
}


// -------------

/// Information required to build the C++ wrapper library
/// on every supported platform. it contains list of linked
/// libraries, frameworks, compiler types and selected type of
/// C++ wrapper library (shared or static). Default value of this
/// object is set before generation of the crate using
/// `cpp_to_rust_generator::config::Config::set_cpp_build_config` or
/// `cpp_build_config_mut` and intended to be cross-platform.
///
/// In order to allow target-dependent build configuration,
/// multiple configurations can be added to one `CppBuildConfig` object,
/// each with a condition.
/// During evaluation, each configuration item
/// will only be used if the associated condition is currently true.
/// All properties from all matching configuration are combined.
///
/// If this conditional evaluation is not enough, a custom build script
/// can modify this config during build script execution using
/// `cpp_to_rust_build_tools::Config::set_cpp_build_config` or
/// `cpp_build_config_mut`.
#[derive(Default, Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct CppBuildConfig {
  items: Vec<CppBuildConfigItem>,
}

#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
struct CppBuildConfigItem {
  condition: ::target::Condition,
  data: CppBuildConfigData,
}

/// Type of a C++ library (shared or static).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum CppLibraryType {
  Shared,
  Static,
}

/// Platform-specific information
/// required to build the C++ wrapper library.
/// This type contains one configuration item of `CppBuildConfig`.
#[derive(Debug, Clone, Default)]
#[derive(Serialize, Deserialize)]
pub struct CppBuildConfigData {
  linked_libs: Vec<String>,
  linked_frameworks: Vec<String>,
  compiler_flags: Vec<String>,
  library_type: Option<CppLibraryType>,
}

impl CppBuildConfigData {
  /// Constructs an empty object.
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
  pub fn add_compiler_flag<P: Into<String>>(&mut self, lib: P) {
    self.compiler_flags.push(lib.into());
  }

  /// Adds multiple flags. See `CppBuildConfigData::add_cpp_compiler_flag`.
  pub fn add_compiler_flags<Item, Iter>(&mut self, items: Iter)
    where Item: Into<String>,
          Iter: IntoIterator<Item = Item>
  {
    for item in items {
      self.compiler_flags.push(item.into());
    }
  }

  /// Sets library type. C++ wrapper is static by default.
  /// Shared library can be used to work around MSVC linker's limitations.
  pub fn set_library_type(&mut self, t: CppLibraryType) {
    self.library_type = Some(t);
  }

  /// Returns names of linked libraries.
  pub fn linked_libs(&self) -> &[String] {
    &self.linked_libs
  }

  /// Returns names of linked frameworks.
  pub fn linked_frameworks(&self) -> &[String] {
    &self.linked_frameworks
  }

  /// Returns C++ compiler flags.
  pub fn compiler_flags(&self) -> &[String] {
    &self.compiler_flags
  }

  /// Returns type of C++ wrapper libary (shared or static).
  pub fn library_type(&self) -> Option<CppLibraryType> {
    self.library_type
  }

  fn add_from(&mut self, other: &CppBuildConfigData) -> Result<()> {
    self.linked_libs.append(&mut other.linked_libs.clone());
    self
      .linked_frameworks
      .append(&mut other.linked_frameworks.clone());
    self
      .compiler_flags
      .append(&mut other.compiler_flags.clone());
    if self.library_type.is_some() {
      if other.library_type.is_some() && other.library_type != self.library_type {
        return Err("conflicting library types specified".into());
      }
    } else {
      self.library_type = other.library_type;
    }
    Ok(())
  }
}

impl CppBuildConfig {
  /// Create an empty configuration
  pub fn new() -> CppBuildConfig {
    CppBuildConfig::default()
  }
  /// Add `data` with `condition`.
  pub fn add(&mut self, condition: ::target::Condition, data: CppBuildConfigData) {
    self
      .items
      .push(CppBuildConfigItem {
              condition: condition,
              data: data,
            });
  }
  /// Select all conditions that are true on `target`, combine all corresponding
  /// configuration items and return the result.
  pub fn eval(&self, target: &::target::Target) -> Result<CppBuildConfigData> {
    let mut data = CppBuildConfigData::default();
    for item in &self.items {
      if item.condition.eval(target) {
        data.add_from(&item.data)?;
      }
    }
    Ok(data)
  }
}

/// This type contains data serialized by the generator and placed to the
/// generated crate's directory. The build script reads and uses this value.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct BuildScriptData {
  /// Information required to build the C++ wrapper library
  pub cpp_build_config: CppBuildConfig,
  /// Name of C++ wrapper library
  pub cpp_wrapper_lib_name: String,
}
