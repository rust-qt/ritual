
use rust_type::{RustName, CompleteType};
use cpp_and_ffi_method::CppAndFfiMethod;
use cpp_data::{EnumValue};
use cpp_type::{CppType};

pub enum RustMethodScope {
  Impl {
    type_name: RustName,
    method_name: String,
  },
  Free {
    method_name: RustName,
  },
}

pub struct RustMethodArgument {
  argument_type: CompleteType,
  name: String,
}

pub struct RustMethodArgumentsVariant {
  arguments: Vec<RustMethodArgument>,
  cpp_method: CppAndFfiMethod,
}

pub enum RustMethodArguments {
  SingleVariant(RustMethodArgumentsVariant),
  MultipleVariants {
    params_enum_name: String,
    params_trait_name: String,
    argument_name: String,
    variants: Vec<RustMethodArgumentsVariant>,
  }
}


pub struct RustMethod {
  scope: RustMethodScope,
  return_type: CompleteType,
}

pub enum TraitName {
  Clone,
  Debug,
  Default,
  Display,
  Drop,
  Eq,
  Hash,
  Ord,
  PartialEq,
  PartialOrd,
  Add,
  AddAssign,
  BitAnd,
  BitAndAssign,
  BitOr,
  BitOrAssign,
  BitXor,
  BitXorAssign,
  Div,
  DivAssign,
  Index,
  IndexMut,
  Mul,
  MulAssign,
  Neg,
  Not,
  Rem,
  RemAssign,
  Shl,
  ShlAssign,
  Shr,
  ShrAssign,
  Sub,
  SubAssign,
  DoubleEndedIterator,
  ExactSizeIterator,
  Extend,
  FromIterator,
  IntoIterator,
  Iterator,
}

pub struct TraitImpl {
  target_type: RustName,
  trait_name: TraitName,
  methods: Vec<RustMethod>,
}

pub enum RustTypeWrapperKind {
  Enum {
    values: Vec<EnumValue>,
  },
  Struct {
    size: i32,
  }
}

pub enum RustTypeDeclarationKind {
  CppTypeWrapper {
    kind: RustTypeWrapperKind,
    cpp_type_name: String,
    cpp_template_arguments: Option<Vec<CppType>>,
  },
  MethodParametersEnum,
  MethodParametersTrait
}

pub struct RustTypeDeclaration {
  pub name: String,
  pub kind: RustTypeDeclarationKind,
  pub methods: Vec<RustMethod>,
  pub traits: Vec<TraitImpl>,
}

pub struct RustModule {
  pub name: String,
  pub crate_name: String,
  pub types: Vec<RustTypeDeclaration>,
  pub functions: Vec<RustMethod>,
}

//pub struct Package {
//  modules: Vec<RustModule>,
//  cpp_data: CppAndFfiData,
//
//}
