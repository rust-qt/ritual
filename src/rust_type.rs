use cpp_type::CppType;
use cpp_ffi_data::IndirectionChange;
use utils::JoinWithString;
use utils::CaseOperations;
pub use serializable::RustName;
extern crate libc;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[allow(dead_code)]
pub enum RustTypeIndirection {
  None,
  Ptr,
  Ref { lifetime: Option<String> },
  PtrPtr,
}



impl RustName {
  pub fn new(parts: Vec<String>) -> RustName {
    assert!(parts.len() > 0);
    RustName { parts: parts }
  }

  pub fn crate_name(&self) -> Option<&String> {
    assert!(self.parts.len() > 0);
    if self.parts.len() > 1 {
      Some(&self.parts[0])
    } else {
      None
    }
  }
  pub fn last_name(&self) -> &String {
    self.parts.last().unwrap()
  }
  pub fn full_name(&self, current_crate: Option<&String>) -> String {
    if current_crate.is_some() && self.crate_name().is_some() &&
       current_crate.unwrap() == self.crate_name().unwrap() {
      format!("::{}", self.parts[1..].join("::"))
    } else {
      self.parts.join("::")
    }
  }

  pub fn includes(&self, other: &RustName) -> bool {
    let extra_modules_count = other.parts.len() - self.parts.len();
    extra_modules_count > 0 && other.parts[0..self.parts.len()] == self.parts[..]
  }

  pub fn includes_directly(&self, other: &RustName) -> bool {
    let extra_modules_count = other.parts.len() - self.parts.len();
    self.includes(other) && extra_modules_count == 1
  }
}

trait ToRustName {
  fn to_rust_name() -> RustName;
}

impl ToRustName for u8 {
  fn to_rust_name() -> RustName {
    RustName::new(vec!["u8".to_string()])
  }
}
impl ToRustName for i8 {
  fn to_rust_name() -> RustName {
    RustName::new(vec!["i8".to_string()])
  }
}
impl ToRustName for u16 {
  fn to_rust_name() -> RustName {
    RustName::new(vec!["u16".to_string()])
  }
}
impl ToRustName for i16 {
  fn to_rust_name() -> RustName {
    RustName::new(vec!["i16".to_string()])
  }
}
impl ToRustName for u32 {
  fn to_rust_name() -> RustName {
    RustName::new(vec!["u32".to_string()])
  }
}
impl ToRustName for i32 {
  fn to_rust_name() -> RustName {
    RustName::new(vec!["i32".to_string()])
  }
}
impl ToRustName for u64 {
  fn to_rust_name() -> RustName {
    RustName::new(vec!["u64".to_string()])
  }
}
impl ToRustName for i64 {
  fn to_rust_name() -> RustName {
    RustName::new(vec!["i64".to_string()])
  }
}
impl ToRustName for f32 {
  fn to_rust_name() -> RustName {
    RustName::new(vec!["f32".to_string()])
  }
}
impl ToRustName for f64 {
  fn to_rust_name() -> RustName {
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
  pub fn caption(&self) -> String {
    match *self {
      RustType::Void => "void".to_string(),
      RustType::Common { ref base,
                         ref generic_arguments,
                         ref is_const,
                         ref is_const2,
                         ref indirection } => {
        let mut name = base.last_name().to_snake_case();
        if let &Some(ref args) = generic_arguments {
          name = format!("{}_{}", name, args.iter().map(|x| x.caption()).join("_"));
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
        }
        name
      }
      RustType::FunctionPointer { .. } => "fn".to_string(),
    }
  }

  #[allow(dead_code)]
  pub fn is_ref(&self) -> bool {
    match *self {
      RustType::Common { ref indirection, .. } => {
        match *indirection {
          RustTypeIndirection::Ref { .. } => true,
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
      if let RustTypeIndirection::Ref { ref mut lifetime } = *indirection {
        assert!(lifetime.is_none());
        *lifetime = Some(new_lifetime);
      }
    }
    r
  }

  pub fn dealias_libc(&self) -> RustType {
    match *self {
      RustType::Void => self.clone(),
      RustType::Common { ref base,
                         ref generic_arguments,
                         ref is_const,
                         ref is_const2,
                         ref indirection } => {
        if base.parts.len() == 2 && &base.parts[0] == "libc" {
          let real_name = match base.parts[1].as_ref() {
            "c_void" => return self.clone(),
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
            _ => panic!("unknown libc type: {:?}", base),
          };
          RustType::Common {
            base: real_name,
            generic_arguments: generic_arguments.clone(),
            is_const: is_const.clone(),
            is_const2: is_const2.clone(),
            indirection: indirection.clone(),
          }
        } else {
          self.clone()
        }
      }
      RustType::FunctionPointer { ref return_type, ref arguments } => {
        RustType::FunctionPointer {
          return_type: Box::new(return_type.as_ref().dealias_libc()),
          arguments: arguments.iter().map(|arg| arg.dealias_libc()).collect(),
        }
      }
    }
  }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[allow(dead_code)]
pub enum RustToCTypeConversion {
  None,
  RefToPtr,
  ValueToPtr,
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
