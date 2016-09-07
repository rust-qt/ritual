
use rust_type::{RustName, CompleteType, RustType, RustTypeIndirection};
use cpp_ffi_data::CppAndFfiMethod;
use cpp_type::CppType;
pub use serializable::RustExportInfo;

/// One variant of a Rust enum
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustEnumValue {
  /// Identifier
  pub name: String,
  /// Corresponding value
  pub value: i64,
  /// Original C++ name of the variant
  pub cpp_name: Option<String>,
  /// Documentation text
  pub doc: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustMethodScope {
  Impl { type_name: RustName },
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
  pub ffi_index: Option<i32>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustMethodArgumentsVariant {
  pub arguments: Vec<RustMethodArgument>,
  pub cpp_method: CppAndFfiMethod,
  pub return_type_ffi_index: Option<i32>,
  pub return_type: CompleteType,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[allow(dead_code)]
pub enum RustMethodArguments {
  SingleVariant(RustMethodArgumentsVariant),
  MultipleVariants {
    params_trait_name: String,
    params_trait_lifetime: Option<String>,
    shared_arguments: Vec<RustMethodArgument>,
    variant_argument_name: String,
  },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustMethod {
  pub scope: RustMethodScope,
  pub name: RustName,
  pub arguments: RustMethodArguments,
  pub doc: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum RustMethodSelfArgKind {
  Static,
  ConstRef,
  MutRef,
  Value,
}

impl RustMethodSelfArgKind {
  pub fn caption(&self) -> &'static str {
    match *self {
      RustMethodSelfArgKind::Static => "static",
      RustMethodSelfArgKind::ConstRef => "from_const",
      RustMethodSelfArgKind::MutRef => "from_mut",
      RustMethodSelfArgKind::Value => "from_value",
    }
  }
}

impl RustMethod {
  pub fn self_arg_kind(&self) -> RustMethodSelfArgKind {
    let args = match self.arguments {
      RustMethodArguments::SingleVariant(ref var) => &var.arguments,
      RustMethodArguments::MultipleVariants { ref shared_arguments, .. } => shared_arguments,
    };
    if args.len() == 0 {
      RustMethodSelfArgKind::Static
    } else {
      let arg = args.get(0).unwrap();
      if arg.name == "self" {
        if let RustType::Common { ref indirection, ref is_const, .. } = arg.argument_type
          .rust_api_type {
          match *indirection {
            RustTypeIndirection::Ref { .. } => {
              if *is_const {
                RustMethodSelfArgKind::ConstRef
              } else {
                RustMethodSelfArgKind::MutRef
              }
            }
            RustTypeIndirection::None => RustMethodSelfArgKind::Value,
            _ => panic!("invalid self argument type"),
          }
        } else {
          panic!("invalid self argument type")
        }
      } else {
        RustMethodSelfArgKind::Static
      }
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[allow(dead_code)]
pub enum TraitName {
  Clone,
  CppDeletable { deleter_name: String },
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
    match *self {
      TraitName::CppDeletable { .. } => "cpp_box::CppDeletable".to_string(),
      _ => format!("{:?}", self),
    }
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
    values: Vec<RustEnumValue>,
    is_flaggable: bool,
  },
  Struct { size: i32 },
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
  MethodParametersTrait {
    lifetime: Option<String>,
    shared_arguments: Vec<RustMethodArgument>,
    impls: Vec<RustMethodArgumentsVariant>,
  },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustTypeDeclaration {
  pub name: String,
  pub kind: RustTypeDeclarationKind,
  pub doc: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustModule {
  pub name: String,
  pub types: Vec<RustTypeDeclaration>,
  pub functions: Vec<RustMethod>,
  pub submodules: Vec<RustModule>,
}

// pub struct Package {
//  modules: Vec<RustModule>,
//  cpp_data: CppAndFfiData,
//
// }
