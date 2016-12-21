
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum Family {
  Windows,
  Unix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum Env {
  Gnu,
  Msvc,
  Musl,
  None,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum PointerWidth {
  P64,
  P32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum Endian {
  Little,
  Big,
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct Target {
  pub arch: Arch,
  pub os: OS,
  pub family: Family,
  pub env: Env,
  pub pointer_width: PointerWidth,
  //pub vendor: Vendor,
  pub endian: Endian,
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum Condition {
  Arch(Arch),
  OS(OS),
  Family(Family),
  Env(Env),
  PointerWidth(PointerWidth),
  //Vendor(Vendor),
  Endian(Endian),
  And(Vec<Condition>),
  Or(Vec<Condition>),
  Not(Box<Condition>),
  True,
  False,
}


// -------------

/// Information required to build the C++ wrapper library
/// on every supported platform.
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

/// Platform-specific information
/// required to build the C++ wrapper library.
#[derive(Debug, Default, Clone)]
#[derive(Serialize, Deserialize)]
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
  pub fn add(&mut self, condition: ::target::Condition, data: CppBuildConfigData) {
    self.items.push(CppBuildConfigItem {
      condition: condition,
      data: data,
    });
  }
  pub fn eval(&self, target: &::target::Target) -> CppBuildConfigData {
    let mut data = CppBuildConfigData::default();
    for item in &self.items {
      if item.condition.eval(target) {
        data.add_from(&item.data);
      }
    }
    data
  }
}
