use cpp_type::CppType;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IndirectionChange {
  NoChange,
  ValueToPointer,
  ReferenceToPointer,
  QFlagsToUInt,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppToFfiTypeConversion {
  pub indirection_change: IndirectionChange,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppFfiType {
  pub original_type: CppType,
  pub ffi_type: CppType,
  pub conversion: CppToFfiTypeConversion,
}

impl CppFfiType {
  pub fn void() -> Self {
    CppFfiType {
      original_type: CppType::void(),
      ffi_type: CppType::void(),
      conversion: CppToFfiTypeConversion {
        indirection_change: IndirectionChange::NoChange,
      },
    }
  }
}
