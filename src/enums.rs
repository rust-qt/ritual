use cpp_type_map::EnumValue;
use cpp_type::CppType;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppTypeIndirection {
  None,
  Ptr,
  Ref,
  PtrRef,
  PtrPtr,
  RefRef,
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CFunctionArgumentCppEquivalent {
  This,
  Argument(i8),
  ReturnValue,
}

impl CFunctionArgumentCppEquivalent {
  pub fn is_argument(&self) -> bool {
    match self {
      &CFunctionArgumentCppEquivalent::Argument(..) => true,
      _ => false,
    }
  }
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppTypeOrigin {
  CBuiltIn,
  Qt {
    include_file: String,
  },
  Unsupported(String),
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
