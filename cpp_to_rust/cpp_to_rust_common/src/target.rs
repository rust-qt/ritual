pub use serializable::{Arch, OS, Family, Env, PointerWidth, Endian, Target, Condition};


#[cfg(target_arch = "x86")]
pub fn current_arch() -> Arch {
  Arch::X86
}
#[cfg(target_arch = "x86_64")]
pub fn current_arch() -> Arch {
  Arch::X86_64
}
#[cfg(target_arch = "mips")]
pub fn current_arch() -> Arch {
  Arch::Mips
}
#[cfg(target_arch = "powerpc")]
pub fn current_arch() -> Arch {
  Arch::PowerPC
}
#[cfg(target_arch = "powerpc64")]
pub fn current_arch() -> Arch {
  Arch::PowerPC64
}
#[cfg(target_arch = "arm")]
pub fn current_arch() -> Arch {
  Arch::Arm
}
#[cfg(target_arch = "aarch64")]
pub fn current_arch() -> Arch {
  Arch::AArch64
}


#[cfg(target_os = "windows")]
pub fn current_os() -> OS {
  OS::Windows
}
#[cfg(target_os = "macos")]
pub fn current_os() -> OS {
  OS::MacOS
}
#[cfg(target_os = "ios")]
pub fn current_os() -> OS {
  OS::IOS
}
#[cfg(target_os = "linux")]
pub fn current_os() -> OS {
  OS::Linux
}
#[cfg(target_os = "android")]
pub fn current_os() -> OS {
  OS::Android
}
#[cfg(target_os = "freebsd")]
pub fn current_os() -> OS {
  OS::FreeBSD
}
#[cfg(target_os = "dragonfly")]
pub fn current_os() -> OS {
  OS::DragonFly
}
#[cfg(target_os = "bitrig")]
pub fn current_os() -> OS {
  OS::Bitrig
}
#[cfg(target_os = "openbsd")]
pub fn current_os() -> OS {
  OS::OpenBSD
}
#[cfg(target_os = "netbsd")]
pub fn current_os() -> OS {
  OS::NetBSD
}


#[cfg(target_family = "unix")]
pub fn current_family() -> Family {
  Family::Unix
}
#[cfg(target_family = "windows")]
pub fn current_family() -> Family {
  Family::Windows
}

#[cfg(target_env = "gnu")]
pub fn current_env() -> Env {
  Env::Gnu
}
#[cfg(target_env = "msvc")]
pub fn current_env() -> Env {
  Env::Msvc
}
#[cfg(target_env = "musl")]
pub fn current_env() -> Env {
  Env::Musl
}
#[cfg(target_env = "")]
pub fn current_env() -> Env {
  Env::None
}

#[cfg(target_pointer_width = "32")]
pub fn current_pointer_width() -> PointerWidth {
  PointerWidth::P32
}
#[cfg(target_pointer_width = "64")]
pub fn current_pointer_width() -> PointerWidth {
  PointerWidth::P64
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum Vendor {
//  Apple,
//  PC,
//  Unknown,
// }
// #[cfg(target_vendor = "apple")]
// pub fn current_vendor() -> Vendor {
//  Vendor::Apple
// }
// #[cfg(target_vendor = "pc")]
// pub fn current_vendor() -> Vendor {
//  Vendor::PC
// }
// #[cfg(target_vendor = "unknown")]
// pub fn current_vendor() -> Vendor {
//  Vendor::Unknown
// }

#[cfg(target_endian = "little")]
pub fn current_endian() -> Endian {
  Endian::Little
}
#[cfg(target_endian = "big")]
pub fn current_endian() -> Endian {
  Endian::Big
}


pub fn current_target() -> Target {
  Target {
    arch: current_arch(),
    os: current_os(),
    family: current_family(),
    env: current_env(),
    pointer_width: current_pointer_width(),
    // vendor: current_vendor(),
    endian: current_endian(),
  }
}


impl Condition {
  pub fn eval(&self, target: &Target) -> bool {
    use target::Condition::*;
    match *self {
      Arch(ref arch) => &target.arch == arch,
      OS(ref os) => &target.os == os,
      Family(ref family) => &target.family == family,
      Env(ref env) => &target.env == env,
      PointerWidth(ref pointer_width) => &target.pointer_width == pointer_width,
      // Vendor(ref vendor) => &target.vendor == vendor,
      Endian(ref endian) => &target.endian == endian,
      And(ref conditions) => conditions.iter().all(|c| c.eval(target)),
      Or(ref conditions) => conditions.iter().any(|c| c.eval(target)),
      Not(ref condition) => !condition.eval(target),
      True => true,
      False => false,
    }
  }
  pub fn negate(&self) -> Condition {
    Condition::Not(Box::new(self.clone()))
  }
}
