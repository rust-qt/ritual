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
  pub crate_name: String,
  pub module_name: String,
  pub own_name: String,
}

impl RustName {
  pub fn full_name(&self, current_crate: &String) -> String {
    format!("{}{}{}",
            if self.crate_name.is_empty() {
              String::new()
            } else {
              if current_crate == &self.crate_name {
                "::".to_string()
              } else {
                format!("{}::", self.crate_name)
              }
            },
            if self.module_name.is_empty() {
              String::new()
            } else {
              format!("{}::", self.module_name)
            },
            self.own_name)
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
  pub name: RustName,
  pub arguments: Vec<RustFFIArgument>,
}
