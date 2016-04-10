use c_type::{CType, CTypeExtended, CppToCTypeConversion};
use cpp_type::CppType;

extern crate inflector;
use self::inflector::Inflector;

#[derive(Debug, Clone)]
pub enum RustTypeIndirection {
  None,
  Ptr,
  Ref,
}

#[derive(Debug, Clone)]
pub struct RustTypeName {
  pub crate_name: String,
  pub module_name: String,
  pub own_name: String,
}

impl RustTypeName {
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

#[derive(Debug, Clone)]
pub enum RustType {
  Void,
  NonVoid {
    base: RustTypeName,
    is_const: bool,
    indirection: RustTypeIndirection,
    is_option: bool,
  },
}

#[derive(Debug, Clone)]
pub enum RustToCTypeConversion {
  None,
  RefToPtr,
  ValueToPtr,
}

#[derive(Debug, Clone)]
pub struct CompleteType {
  pub c_type: CType,
  pub cpp_type: CppType,
  pub cpp_to_c_conversion: CppToCTypeConversion,
  pub rust_ffi_type: RustType, /* pub rust_api_type: RustType,
                                * pub rust_api_to_c_conversion: RustToCTypeConversion, */
}
