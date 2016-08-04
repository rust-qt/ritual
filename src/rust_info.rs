
use rust_type::{RustName, CompleteType, RustType};
use cpp_and_ffi_method::CppAndFfiMethod;
use cpp_data::EnumValue;
use cpp_type::CppType;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustMethodScope {
  Impl {
    type_name: RustName,
  },
  TraitImpl {
    type_name: RustName,
    trait_name: TraitName,
  },
  Free,
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
  pub return_type_ffi_index: Option<i32>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[allow(dead_code)]
pub enum RustMethodArguments {
  SingleVariant(RustMethodArgumentsVariant),
  MultipleVariants {
    params_enum_name: String,
    params_trait_name: String,
    shared_arguments: Vec<RustMethodArgument>,
    variant_argument_name: String,
    variants: Vec<RustMethodArgumentsVariant>,
  },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustMethod {
  pub scope: RustMethodScope,
  pub return_type: CompleteType,
  pub name: RustName,
  pub arguments: RustMethodArguments,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[allow(dead_code)]
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
impl TraitName {
  pub fn to_string(&self) -> String {
    format!("{:?}", self)
  }
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
    is_flaggable: bool,
  },
  Struct {
    size: i32,
  },
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[allow(dead_code)]
pub enum RustTypeDeclarationKind {
  CppTypeWrapper {
    kind: RustTypeWrapperKind,
    cpp_type_name: String,
    cpp_template_arguments: Option<Vec<CppType>>,
    methods: Vec<RustMethod>,
    traits: Vec<TraitImpl>,
  },
  MethodParametersEnum {
    variants: Vec<Vec<RustType>>,
    trait_name: RustName,
  },
  MethodParametersTrait {
    enum_name: RustName,
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustTypeDeclaration {
  pub name: RustName,
  pub kind: RustTypeDeclarationKind,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustModule {
  pub name: RustName,
  pub types: Vec<RustTypeDeclaration>,
  pub functions: Vec<RustMethod>,
  pub submodules: Vec<RustModule>,
}

// pub struct Package {
//  modules: Vec<RustModule>,
//  cpp_data: CppAndFfiData,
//
// }
