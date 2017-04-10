//! Types for expressing properties of different target platforms and platform-based conditions

pub use serializable::{Arch, OS, Family, Env, PointerWidth, Endian, Target, Condition};

#[cfg(target_arch = "x86")]
/// Returns current CPU architecture
pub fn current_arch() -> Arch {
  Arch::X86
}
#[cfg(target_arch = "x86_64")]
/// Returns current CPU architecture
pub fn current_arch() -> Arch {
  Arch::X86_64
}
#[cfg(target_arch = "mips")]
/// Returns current CPU architecture
pub fn current_arch() -> Arch {
  Arch::Mips
}
#[cfg(target_arch = "powerpc")]
/// Returns current CPU architecture
pub fn current_arch() -> Arch {
  Arch::PowerPC
}
#[cfg(target_arch = "powerpc64")]
/// Returns current CPU architecture
pub fn current_arch() -> Arch {
  Arch::PowerPC64
}
#[cfg(target_arch = "arm")]
/// Returns current CPU architecture
pub fn current_arch() -> Arch {
  Arch::Arm
}
#[cfg(target_arch = "aarch64")]
/// Returns current CPU architecture
pub fn current_arch() -> Arch {
  Arch::AArch64
}


#[cfg(target_os = "windows")]
/// Returns current operating system
pub fn current_os() -> OS {
  OS::Windows
}
#[cfg(target_os = "macos")]
/// Returns current operating system
pub fn current_os() -> OS {
  OS::MacOS
}
#[cfg(target_os = "ios")]
/// Returns current operating system
pub fn current_os() -> OS {
  OS::IOS
}
#[cfg(target_os = "linux")]
/// Returns current operating system
pub fn current_os() -> OS {
  OS::Linux
}
#[cfg(target_os = "android")]
/// Returns current operating system
pub fn current_os() -> OS {
  OS::Android
}
#[cfg(target_os = "freebsd")]
/// Returns current operating system
pub fn current_os() -> OS {
  OS::FreeBSD
}
#[cfg(target_os = "dragonfly")]
/// Returns current operating system
pub fn current_os() -> OS {
  OS::DragonFly
}
#[cfg(target_os = "bitrig")]
/// Returns current operating system
pub fn current_os() -> OS {
  OS::Bitrig
}
#[cfg(target_os = "openbsd")]
/// Returns current operating system
pub fn current_os() -> OS {
  OS::OpenBSD
}
#[cfg(target_os = "netbsd")]
/// Returns current operating system
pub fn current_os() -> OS {
  OS::NetBSD
}


#[cfg(target_family = "unix")]
/// Returns current operating system family
pub fn current_family() -> Family {
  Family::Unix
}
#[cfg(target_family = "windows")]
/// Returns current operating system family
pub fn current_family() -> Family {
  Family::Windows
}

#[cfg(target_env = "gnu")]
/// Returns current platform disambiguation
pub fn current_env() -> Env {
  Env::Gnu
}
#[cfg(target_env = "msvc")]
/// Returns current platform disambiguation
pub fn current_env() -> Env {
  Env::Msvc
}
#[cfg(target_env = "musl")]
/// Returns current platform disambiguation
pub fn current_env() -> Env {
  Env::Musl
}
#[cfg(target_env = "")]
/// Returns current platform disambiguation
pub fn current_env() -> Env {
  Env::None
}

#[cfg(target_pointer_width = "32")]
/// Returns current pointer width
pub fn current_pointer_width() -> PointerWidth {
  PointerWidth::P32
}
#[cfg(target_pointer_width = "64")]
/// Returns current pointer width
pub fn current_pointer_width() -> PointerWidth {
  PointerWidth::P64
}

#[cfg(target_endian = "little")]
/// Returns current CPU endianness
pub fn current_endian() -> Endian {
  Endian::Little
}
#[cfg(target_endian = "big")]
/// Returns current CPU endianness
pub fn current_endian() -> Endian {
  Endian::Big
}

/// Returns properties of the current target
pub fn current_target() -> Target {
  Target {
    arch: current_arch(),
    os: current_os(),
    family: current_family(),
    env: current_env(),
    pointer_width: current_pointer_width(),
    endian: current_endian(),
  }
}


impl Condition {
  /// Evaluate the condition for `target`. Returns true if
  /// `target` matches the condition.
  pub fn eval(&self, target: &Target) -> bool {
    use target::Condition::*;
    match *self {
      Arch(ref arch) => &target.arch == arch,
      OS(ref os) => &target.os == os,
      Family(ref family) => &target.family == family,
      Env(ref env) => &target.env == env,
      PointerWidth(ref pointer_width) => &target.pointer_width == pointer_width,
      Endian(ref endian) => &target.endian == endian,
      And(ref conditions) => conditions.iter().all(|c| c.eval(target)),
      Or(ref conditions) => conditions.iter().any(|c| c.eval(target)),
      Not(ref condition) => !condition.eval(target),
      True => true,
      False => false,
    }
  }
  /// Construct a condition opposite to `self`.
  pub fn negate(&self) -> Condition {
    Condition::Not(Box::new(self.clone()))
  }
}
