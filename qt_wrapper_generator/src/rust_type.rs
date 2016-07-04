use cpp_type::{CppType};
use cpp_ffi_type::CppToFfiTypeConversion;

#[derive(Debug, Clone, Eq, PartialEq)]
#[allow(dead_code)]
pub enum RustTypeIndirection {
  None,
  Ptr,
  Ref,
}

#[derive(Debug, Clone, Eq, PartialEq)]
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
  pub fn full_name(&self, current_crate: &String) -> String {
    if Some(current_crate) == self.crate_name() {
      format!("::{}", self.parts[1..].join("::"))
    } else {
      self.parts.join("::")
    }
  }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RustType {
  Void,
  NonVoid {
    base: RustName,
    generic_arguments: Option<Vec<RustType>>,
    is_const: bool,
    indirection: RustTypeIndirection,
    is_option: bool,
  },
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
