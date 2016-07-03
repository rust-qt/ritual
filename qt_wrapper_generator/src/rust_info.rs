
use rust_type::{RustName, CompleteType};
use cpp_and_ffi_method::CppAndFfiMethod;
use cpp_data::{EnumValue};
use cpp_type::{CppType};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustMethodScope {
  Impl {
    type_name: RustName,
  },
  Free
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustMethodArgument {
  pub argument_type: CompleteType,
  pub name: String,
  pub ffi_index: i32,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustMethodArgumentsVariant {
  pub arguments: Vec<RustMethodArgument>,
  pub cpp_method: CppAndFfiMethod,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustMethodArguments {
  SingleVariant(RustMethodArgumentsVariant),
  MultipleVariants {
    params_enum_name: String,
    params_trait_name: String,
    argument_name: String,
    variants: Vec<RustMethodArgumentsVariant>,
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustMethod {
  pub scope: RustMethodScope,
  pub return_type: CompleteType,
  pub return_type_ffi_index: Option<i32>,
  pub name: String,
  pub arguments: RustMethodArguments,
}

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TraitImpl {
  pub target_type: RustName,
  pub trait_name: TraitName,
  pub methods: Vec<RustMethod>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustTypeWrapperKind {
  Enum {
    values: Vec<EnumValue>,
  },
  Struct {
    size: i32,
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustTypeDeclarationKind {
  CppTypeWrapper {
    kind: RustTypeWrapperKind,
    cpp_type_name: String,
    cpp_template_arguments: Option<Vec<CppType>>,
  },
  MethodParametersEnum,
  MethodParametersTrait
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustTypeDeclaration {
  pub name: String,
  pub kind: RustTypeDeclarationKind,
  pub methods: Vec<RustMethod>,
  pub traits: Vec<TraitImpl>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustModule {
  pub name: String,
  pub full_modules_name: String,
  pub crate_name: String,
  pub types: Vec<RustTypeDeclaration>,
  pub functions: Vec<RustMethod>,
  pub submodules: Vec<RustModule>,
}

//pub struct Package {
//  modules: Vec<RustModule>,
//  cpp_data: CppAndFfiData,
//
//}
