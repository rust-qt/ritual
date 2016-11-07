use cpp_ffi_data::CppAndFfiMethod;
use cpp_type::CppType;
use errors::{Result, ChainErr, unexpected};
use file_utils::load_toml;
use rust_type::{RustName, CompleteType, RustType, RustTypeIndirection};
use utils::MapIfOk;

pub use serializable::{RustEnumValue, RustTypeWrapperKind, RustProcessedTypeInfo, RustExportInfo};


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
    params_trait_return_type: Option<RustType>,
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

impl RustMethod {
  pub fn self_arg_kind(&self) -> Result<RustMethodSelfArgKind> {
    let args = match self.arguments {
      RustMethodArguments::SingleVariant(ref var) => &var.arguments,
      RustMethodArguments::MultipleVariants { ref shared_arguments, .. } => shared_arguments,
    };
    Ok(if let Some(arg) = args.get(0) {
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
            _ => return Err(unexpected("invalid self argument type").into()),
          }
        } else {
          return Err(unexpected("invalid self argument type").into());
        }
      } else {
        RustMethodSelfArgKind::Static
      }
    } else {
      RustMethodSelfArgKind::Static
    })
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
      TraitName::CppDeletable { .. } => "cpp_utils::CppDeletable".to_string(),
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
    return_type: Option<RustType>,
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


use std::path::PathBuf;

pub struct InputCargoTomlData {
  /// Name of the crate
  pub name: String,
  /// Version of the crate
  pub version: String,
  /// Authors of the crate
  pub authors: Vec<String>,
  /// Name of the C++ library
  pub links: Option<String>,
}


impl InputCargoTomlData {
  pub fn from_file(path: &PathBuf) -> Result<InputCargoTomlData> {
    let value = try!(load_toml(path));
    let package = try!(value.get("package")
      .chain_err(|| "'package' field not found in Cargo.toml"));
    let package = try!(package.as_table().chain_err(|| "'package' must be a table"));
    Ok(InputCargoTomlData {
      name: {
        let name = try!(package.get("name")
          .chain_err(|| "'package.name' field not found in Cargo.toml"));
        try!(name.as_str().chain_err(|| "'package.name' must be a string")).to_string()
      },
      version: {
        let version = try!(package.get("version")
          .chain_err(|| "'package.version' field not found in Cargo.toml"));
        try!(version.as_str().chain_err(|| "'package.version' must be a string")).to_string()
      },
      authors: if let Some(authors) = package.get("authors") {
        let authors = try!(authors.as_slice().chain_err(|| "'package.authors' must be an array"));
        try!(authors.iter().map_if_ok(|x| -> Result<_> {
          Ok(try!(x.as_str().chain_err(|| "'package.authors[i]' must be a string")).to_string())
        }))
      } else {
        Vec::new()
      },
      links: if let Some(links) = package.get("links") {
        Some(try!(links.as_str().chain_err(|| "'package.links' must be a string")).to_string())
      } else {
        None
      },
    })
  }
}
