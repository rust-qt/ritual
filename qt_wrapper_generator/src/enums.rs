use cpp_type_map::EnumValue;
use cpp_type::CppType;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppTypeIndirection {
  None,
  Ptr,
  Ref,
  PtrRef,
  PtrPtr,
  RValueRef,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IndirectionChange {
  NoChange,
  ValueToPointer,
  ReferenceToPointer,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppMethodScope {
  Global,
  Class(String),
}

impl CppMethodScope {
  pub fn class_name(&self) -> Option<&String> {
    match *self {
      CppMethodScope::Global => None,
      CppMethodScope::Class(ref s) => Some(s)
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppFfiArgumentMeaning {
  This,
  Argument(i8),
  ReturnValue,
}

impl CppFfiArgumentMeaning {
  pub fn is_argument(&self) -> bool {
    match self {
      &CppFfiArgumentMeaning::Argument(..) => true,
      _ => false,
    }
  }
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppTypeOriginLocation {
  pub include_file_path: String,
  pub line: u32,
  pub column: u32,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppTypeOrigin {
  CBuiltIn,
  IncludeFile {
    include_file: String,
    location: Option<CppTypeOriginLocation>,
  },
  Unknown,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppTypeKind {
  CPrimitive,
  Enum {
    values: Vec<EnumValue>,
  },
  Flags {
    enum_name: String,
  },
  TypeDef {
    meaning: CppType,
  },
  Class {
    inherits: Option<CppType>,
  },
  Unknown,
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AllocationPlace {
  Stack,
  Heap,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AllocationPlaceImportance {
  Important,
  NotImportant,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppVisibility {
  Public,
  Protected,
  Private
}