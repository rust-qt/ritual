use cpp_ffi_data::CppAndFfiMethod;
use cpp_type::CppType;
use common::errors::{Result, unexpected};
use rust_type::{RustName, CompleteType, RustType, RustTypeIndirection};
use cpp_method::CppMethodDoc;
use cpp_data::CppTypeDoc;
pub use serializable::{RustEnumValue, RustTypeWrapperKind, RustProcessedTypeInfo, RustExportInfo,
                       CppEnumValueDocItem, RustQtSlotWrapper};

use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustMethodDocItem {
  pub doc: Option<CppMethodDoc>,
  pub rust_fns: Vec<String>,
  pub cpp_fn: String,
  pub rust_cross_references: Vec<RustCrossReference>,
}



#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustMethodScope {
  Impl { target_type: RustType },
  TraitImpl,
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
    cpp_method_name: String,
  },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustMethod {
  pub scope: RustMethodScope,
  pub is_unsafe: bool,
  pub name: RustName,
  pub arguments: RustMethodArguments,
  pub docs: Vec<RustMethodDocItem>,
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

  #[allow(dead_code)]
  pub fn cpp_cross_references(&self) -> Vec<String> {
    let mut r = Vec::new();
    for doc in &self.docs {
      if let Some(ref doc) = doc.doc {
        r.append(&mut doc.cross_references.clone());
      }
    }
    r
  }

  #[allow(dead_code)]
  pub fn add_rust_cross_references(&mut self, table: HashMap<String, RustCrossReference>) {
    for doc in &mut self.docs {
      let mut result = Vec::new();
      if let Some(ref doc) = doc.doc {
        for reference in &doc.cross_references {
          if let Some(r) = table.get(reference) {
            result.push(r.clone());
          }
        }
      }
      doc.rust_cross_references = result;
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TraitImplExtra {
  CppDeletable { deleter_name: String },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TraitImpl {
  pub target_type: RustType,
  pub trait_type: RustType,
  pub extra: Option<TraitImplExtra>,
  pub methods: Vec<RustMethod>,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustCrossReferenceKind {
  Method { scope: RustMethodScope },
  Type,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustCrossReference {
  name: RustName,
  kind: RustCrossReferenceKind,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustQtReceiverType {
  Signal,
  Slot,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustQtReceiverDeclaration {
  pub type_name: String,
  pub method_name: String,
  pub receiver_type: RustQtReceiverType,
  pub receiver_id: String,
  pub arguments: Vec<RustType>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustTypeDeclarationKind {
  CppTypeWrapper {
    kind: RustTypeWrapperKind,
    cpp_type_name: String,
    cpp_template_arguments: Option<Vec<CppType>>,
    cpp_doc: Option<CppTypeDoc>,
    rust_cross_references: Vec<RustCrossReference>,
    methods: Vec<RustMethod>,
    trait_impls: Vec<TraitImpl>,
    qt_receivers: Vec<RustQtReceiverDeclaration>,
  },
  MethodParametersTrait {
    lifetime: Option<String>,
    shared_arguments: Vec<RustMethodArgument>,
    return_type: Option<RustType>,
    impls: Vec<RustMethodArgumentsVariant>,
    method_scope: RustMethodScope,
    method_name: RustName,
    is_unsafe: bool,
  },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustTypeDeclaration {
  pub is_public: bool,
  pub name: RustName,
  pub kind: RustTypeDeclarationKind,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustModule {
  pub name: String,
  pub types: Vec<RustTypeDeclaration>,
  pub functions: Vec<RustMethod>,
  pub trait_impls: Vec<TraitImpl>,
  pub submodules: Vec<RustModule>,
}
