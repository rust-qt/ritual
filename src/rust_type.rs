use cpp_type::CppType;
use cpp_ffi_type::CppToFfiTypeConversion;
use utils::JoinWithString;
use utils::CaseOperations;

extern crate libc;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[allow(dead_code)]
pub enum RustTypeIndirection {
  None,
  Ptr,
  Ref {
    lifetime: Option<String>,
  },
  PtrPtr,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct RustName {
  pub parts: Vec<String>,
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
  NonVoid {
    base: RustName,
    generic_arguments: Option<Vec<RustType>>,
    is_const: bool,
    indirection: RustTypeIndirection,
  },
}

impl RustType {
  pub fn caption(&self) -> String {
    match *self {
      RustType::Void => "void".to_string(),
      RustType::NonVoid { ref base, ref generic_arguments, ref is_const, ref indirection } => {
        let mut name = base.last_name().to_snake_case();
        if let &Some(ref args) = generic_arguments {
          name = format!("{}_{}", name, args.iter().map(|x| x.caption()).join("_"));
        }
        let mut_text = if *is_const {
          ""
        } else {
          "_mut"
        };
        match *indirection {
          RustTypeIndirection::None => {}
          RustTypeIndirection::Ref { .. } => {
            name = format!("{}{}_ref", name, mut_text);
          }
          RustTypeIndirection::Ptr => {
            name = format!("{}{}_ptr", name, mut_text);
          }
          RustTypeIndirection::PtrPtr => {
            name = format!("{}{}_ptr_ptr", name, mut_text);
          }
        }
        name
      }
    }
  }

  pub fn is_ref(&self) -> bool {
    match *self {
      RustType::NonVoid { ref indirection, .. } => {
        match *indirection {
          RustTypeIndirection::Ref { .. } => true,
          _ => false,
        }
      }
      RustType::Void => false
    }
  }

  pub fn with_lifetime(&self, new_lifetime: String) -> RustType {
    let mut r = self.clone();
    if let RustType::NonVoid { ref mut indirection, .. } = r {
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
      RustType::NonVoid { ref base, ref generic_arguments, ref is_const, ref indirection } => {
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
          RustType::NonVoid {
            base: real_name,
            generic_arguments: generic_arguments.clone(),
            is_const: is_const.clone(),
            indirection: indirection.clone(),
          }
        } else {
          self.clone()
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
  pub cpp_to_ffi_conversion: CppToFfiTypeConversion,
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
