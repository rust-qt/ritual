use cpp_ffi_data::IndirectionChange;
use cpp_type::CppType;
use errors::{Result, unexpected, ChainErr};
use string_utils::CaseOperations;
use utils::MapIfOk;

extern crate libc;

pub use serializable::RustName;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[allow(dead_code)]
pub enum RustTypeIndirection {
  None,
  Ptr,
  Ref { lifetime: Option<String> },
  PtrPtr,
  PtrRef { lifetime: Option<String> },
}

impl RustName {
  pub fn new(parts: Vec<String>) -> Result<RustName> {
    if parts.is_empty() {
      return Err(unexpected("RustName can't be empty").into());
    }
    Ok(RustName { parts: parts })
  }

  pub fn crate_name(&self) -> Option<&String> {
    assert!(self.parts.len() > 0);
    if self.parts.len() > 1 {
      Some(&self.parts[0])
    } else {
      None
    }
  }
  pub fn last_name(&self) -> Result<&String> {
    self.parts.last().chain_err(|| unexpected("RustName can't be empty"))
  }
  pub fn full_name(&self, current_crate: Option<&str>) -> String {
    if let Some(current_crate) = current_crate {
      if let Some(self_crate) = self.crate_name() {
        if self_crate == current_crate {
          return format!("::{}", self.parts[1..].join("::"));
        }
      }
    }
    self.parts.join("::")
  }

  pub fn includes(&self, other: &RustName) -> bool {
    let extra_modules_count = other.parts.len() as isize - self.parts.len() as isize;
    extra_modules_count > 0 && other.parts[0..self.parts.len()] == self.parts[..]
  }

  pub fn includes_directly(&self, other: &RustName) -> bool {
    let extra_modules_count = other.parts.len() as isize - self.parts.len() as isize;
    self.includes(other) && extra_modules_count == 1
  }
}

trait ToRustName {
  fn to_rust_name() -> Result<RustName>;
}

impl ToRustName for u8 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["u8".to_string()])
  }
}
impl ToRustName for i8 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["i8".to_string()])
  }
}
impl ToRustName for u16 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["u16".to_string()])
  }
}
impl ToRustName for i16 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["i16".to_string()])
  }
}
impl ToRustName for u32 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["u32".to_string()])
  }
}
impl ToRustName for i32 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["i32".to_string()])
  }
}
impl ToRustName for u64 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["u64".to_string()])
  }
}
impl ToRustName for i64 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["i64".to_string()])
  }
}
impl ToRustName for f32 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["f32".to_string()])
  }
}
impl ToRustName for f64 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["f64".to_string()])
  }
}





#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum RustType {
  Void,
  Common {
    base: RustName,
    generic_arguments: Option<Vec<RustType>>,
    is_const: bool,
    is_const2: bool,
    indirection: RustTypeIndirection,
  },
  FunctionPointer {
    return_type: Box<RustType>,
    arguments: Vec<RustType>,
  },
}

impl RustType {
  #[allow(dead_code)]
  pub fn caption(&self) -> Result<String> {
    Ok(match *self {
      RustType::Void => "void".to_string(),
      RustType::Common { ref base,
                         ref generic_arguments,
                         ref is_const,
                         ref is_const2,
                         ref indirection } => {
        let mut name = try!(base.last_name()).to_snake_case();
        if let Some(ref args) = *generic_arguments {
          name = format!("{}_{}",
                         name,
                         try!(args.iter().map_if_ok(|x| x.caption())).join("_"));
        }
        let mut_text = if *is_const { "" } else { "_mut" };
        match *indirection {
          RustTypeIndirection::None => {}
          RustTypeIndirection::Ref { .. } => {
            name = format!("{}{}_ref", name, mut_text);
          }
          RustTypeIndirection::Ptr => {
            name = format!("{}{}_ptr", name, mut_text);
          }
          RustTypeIndirection::PtrPtr => {
            let mut_text2 = if *is_const2 { "" } else { "_mut" };
            name = format!("{}{}_ptr{}_ptr", name, mut_text, mut_text2);
          }
          RustTypeIndirection::PtrRef { .. } => {
            let mut_text2 = if *is_const2 { "" } else { "_mut" };
            name = format!("{}{}_ptr{}_ref", name, mut_text, mut_text2);
          }
        }
        name
      }
      RustType::FunctionPointer { .. } => "fn".to_string(),
    })
  }

  #[allow(dead_code)]
  pub fn is_ref(&self) -> bool {
    match *self {
      RustType::Common { ref indirection, .. } => {
        match *indirection {
          RustTypeIndirection::Ref { .. } |
          RustTypeIndirection::PtrRef { .. } => true,
          _ => false,
        }
      }
      RustType::Void |
      RustType::FunctionPointer { .. } => false,
    }
  }

  pub fn with_lifetime(&self, new_lifetime: String) -> RustType {
    let mut r = self.clone();
    if let RustType::Common { ref mut indirection, .. } = r {
      match *indirection {
        RustTypeIndirection::Ref { ref mut lifetime } |
        RustTypeIndirection::PtrRef { ref mut lifetime } => *lifetime = Some(new_lifetime),
        _ => {}
      }
    }
    r
  }

  pub fn lifetime(&self) -> Option<&String> {
    match *self {
      RustType::Common { ref indirection, .. } => {
        match *indirection {
          RustTypeIndirection::Ref { ref lifetime } |
          RustTypeIndirection::PtrRef { ref lifetime } => lifetime.as_ref(),
          _ => None,
        }
      }
      _ => None,
    }
  }

  pub fn dealias_libc(&self) -> Result<RustType> {
    Ok(match *self {
      RustType::Void => self.clone(),
      RustType::Common { ref base,
                         ref generic_arguments,
                         ref is_const,
                         ref is_const2,
                         ref indirection } => {
        if base.parts.len() == 2 && &base.parts[0] == "libc" {
          let real_name = match base.parts[1].as_ref() {
            "c_void" => return Ok(self.clone()),
            "c_schar" => libc::c_schar::to_rust_name(),
            "c_char" => libc::c_char::to_rust_name(),
            "c_uchar" => libc::c_uchar::to_rust_name(),
            "wchar_t" => libc::wchar_t::to_rust_name(),
            "c_short" => libc::c_short::to_rust_name(),
            "c_ushort" => libc::c_ushort::to_rust_name(),
            "c_int" => libc::c_int::to_rust_name(),
            "c_uint" => libc::c_uint::to_rust_name(),
            "c_long" => libc::c_long::to_rust_name(),
            "c_ulong" => libc::c_ulong::to_rust_name(),
            "c_longlong" => libc::c_longlong::to_rust_name(),
            "c_ulonglong" => libc::c_ulonglong::to_rust_name(),
            "c_float" => libc::c_float::to_rust_name(),
            "c_double" => libc::c_double::to_rust_name(),
            _ => return Err(unexpected(format!("unknown libc type: {:?}", base)).into()),
          };
          RustType::Common {
            base: try!(real_name),
            generic_arguments: generic_arguments.clone(),
            is_const: *is_const,
            is_const2: *is_const2,
            indirection: indirection.clone(),
          }
        } else {
          self.clone()
        }
      }
      RustType::FunctionPointer { ref return_type, ref arguments } => {
        RustType::FunctionPointer {
          return_type: Box::new(try!(return_type.as_ref().dealias_libc())),
          arguments: try!(arguments.iter().map_if_ok(|arg| arg.dealias_libc())),
        }
      }
    })
  }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[allow(dead_code)]
pub enum RustToCTypeConversion {
  None,
  RefToPtr,
  ValueToPtr,
  CppBoxToPtr,
  QFlagsToUInt,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CompleteType {
  pub cpp_type: CppType,
  pub cpp_ffi_type: CppType,
  pub cpp_to_ffi_conversion: IndirectionChange,
  pub rust_ffi_type: RustType,
  pub rust_api_type: RustType,
  pub rust_api_to_c_conversion: RustToCTypeConversion,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RustFFIArgument {
  pub name: String,
  pub argument_type: RustType,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RustFFIFunction {
  pub return_type: RustType,
  pub name: String,
  pub arguments: Vec<RustFFIArgument>,
}
