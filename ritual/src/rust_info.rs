//! Types holding information about generates Rust API.

use crate::cpp_data::CppPath;
use crate::cpp_data::CppTypeDoc;
use crate::cpp_ffi_data::CppFfiFunction;
use crate::cpp_type::CppType;
use crate::rust_type::RustPointerLikeTypeKind;
use crate::rust_type::{RustFinalType, RustPath, RustType};
use itertools::Itertools;
use ritual_common::errors::{bail, Result};
use ritual_common::utils::MapIfOk;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;

/// One variant of a Rust enum
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustEnumValue {
    pub path: RustPath,
    /// Corresponding value
    pub value: i64,
    /// Documentation of corresponding C++ variants
    pub doc: RustEnumValueDoc,
}

/// C++ documentation data for a enum variant
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustEnumValueDoc {
    pub extra_doc: Option<String>,
    /// C++ path of the variant
    pub cpp_path: CppPath,
    /// HTML content
    pub cpp_doc: Option<String>,
}

/// Information about a Qt slot wrapper on Rust side
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustQtSlotWrapper {
    /// Argument types of the slot
    pub arguments: Vec<RustFinalType>,
    /// Identifier of the slot for `QObject::connect`
    pub receiver_id: String,
    /// Name of the extern callback function of this wrapper
    pub callback_path: RustPath,

    pub cpp_arguments: Vec<CppType>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RustWrapperTypeKind {
    EnumWrapper,
    ImmovableClassWrapper,
    MovableClassWrapper { sized_type_path: RustPath },
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustRawQtSlotWrapperDocData {
    pub public_wrapper_path: RustPath,
    pub rust_arguments: Vec<RustFinalType>,
    pub cpp_arguments: Vec<CppType>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustWrapperTypeDocData {
    /// Corresponding C++ type (for generating docs).
    pub cpp_path: CppPath,
    /// C++ documentation for this type
    pub cpp_doc: Option<CppTypeDoc>,

    pub raw_qt_slot_wrapper: Option<RustRawQtSlotWrapperDocData>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustWrapperType {
    pub doc_data: RustWrapperTypeDocData,
    pub kind: RustWrapperTypeKind,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustFfiClassTypeDoc {
    pub cpp_path: CppPath,
    pub public_rust_path: RustPath,
}

/// Information about a Rust type wrapper
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RustStructKind {
    WrapperType(RustWrapperType),
    QtSlotWrapper(RustQtSlotWrapper),
    SizedType(CppPath),
}

impl RustStructKind {
    pub fn is_wrapper_type(&self) -> bool {
        if let RustStructKind::WrapperType(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_sized_type(&self) -> bool {
        match *self {
            RustStructKind::SizedType(_) => true,
            _ => false,
        }
    }
}

/// Exported information about a Rust wrapper type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustStruct {
    /// Additional documentation content that will appear before C++ documentation or any other
    /// automatically generated content.
    pub extra_doc: Option<String>,
    pub path: RustPath,
    /// Kind of the type and additional information.
    pub kind: RustStructKind,
    /// Indicates whether this type is public
    pub is_public: bool,
}

/// Location of a Rust method.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RustFunctionScope {
    /// Inside `impl T {}`, where `T` is `target_type`.
    Impl { target_type: RustType },
    /// Inside a trait implementation.
    TraitImpl,
    /// A free function.
    Free,
}

/// Information about a Rust method argument.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustFunctionArgument {
    /// C++ and Rust types corresponding to this argument at all levels.
    pub argument_type: RustFinalType,
    /// Rust argument name.
    pub name: String,
    /// Index of the corresponding argument of the FFI function.
    pub ffi_index: usize,
}

/// Type of a receiver in Qt connection system.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RustQtReceiverType {
    Signal,
    Slot,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustFfiWrapperData {
    /// C++ method corresponding to this variant.
    pub cpp_ffi_function: CppFfiFunction,

    pub ffi_function_path: RustPath,
    /// Index of the FFI function argument used for acquiring the return value,
    /// if any. `None` if the return value is passed normally (as the return value
    /// of the FFI function).
    pub return_type_ffi_index: Option<usize>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RustFunctionKind {
    FfiWrapper(RustFfiWrapperData),
    CppDeletableImpl {
        deleter: RustPath,
    },
    SignalOrSlotGetter {
        /// Name of the type.
        type_path: RustPath,
        /// C++ name of the signal or slot
        cpp_path: CppPath,
        /// Type of the receiver.
        receiver_type: RustQtReceiverType,
        /// Identifier of the signal or slot for passing to `QObject::connect`.
        receiver_id: String,
        /// Types or arguments.
        arguments: Vec<RustType>,
    },
}

#[derive(Debug, Clone)]
pub struct UnnamedRustFunction {
    pub is_public: bool,
    pub is_unsafe: bool,
    pub kind: RustFunctionKind,
    pub arguments: Vec<RustFunctionArgument>,
    pub return_type: RustFinalType,
    pub extra_doc: Option<String>,
}

impl UnnamedRustFunction {
    pub fn with_path(self, path: RustPath) -> RustFunction {
        RustFunction {
            path,
            is_public: self.is_public,
            is_unsafe: self.is_unsafe,
            kind: self.kind,
            arguments: self.arguments,
            return_type: self.return_type,
            extra_doc: self.extra_doc,
        }
    }

    /// Returns information about `self` argument of this method.
    fn self_arg_kind(&self) -> Result<RustFunctionSelfArgKind> {
        if let Some(arg) = self.arguments.get(0) {
            if arg.name == "self" {
                match arg.argument_type.api_type {
                    RustType::PointerLike {
                        ref kind,
                        ref is_const,
                        ..
                    } => match *kind {
                        RustPointerLikeTypeKind::Pointer => {
                            bail!("pointer self arg is not supported")
                        }
                        RustPointerLikeTypeKind::Reference { .. } => {
                            if *is_const {
                                return Ok(RustFunctionSelfArgKind::ConstRef);
                            } else {
                                return Ok(RustFunctionSelfArgKind::MutRef);
                            }
                        }
                    },
                    RustType::Common { .. } => {
                        return Ok(RustFunctionSelfArgKind::Value);
                    }
                    _ => {
                        bail!("invalid self argument type: {:?}", self);
                    }
                }
            }
        }
        Ok(RustFunctionSelfArgKind::None)
    }

    /// Generates name suffix for this function using `caption_strategy`.
    /// `all_self_args` should contain all kinds of arguments found in
    /// the functions that have to be disambiguated using the name suffix.
    /// `index` is number of the function used in `RustFunctionCaptionStrategy::Index`.
    #[allow(dead_code)]
    fn name_suffix(
        &self,
        context: &RustPath,
        caption_strategy: &RustFunctionCaptionStrategy,
        all_self_args: &HashSet<RustFunctionSelfArgKind>,
        index: usize,
    ) -> Result<Option<String>> {
        if caption_strategy == &RustFunctionCaptionStrategy::UnsafeOnly {
            return Ok(if self.is_unsafe {
                Some("unsafe".to_string())
            } else {
                None
            });
        }
        let result = {
            let self_arg_kind = self.self_arg_kind()?;
            let self_arg_kind_caption =
                if all_self_args.len() == 1 || self_arg_kind == RustFunctionSelfArgKind::ConstRef {
                    None
                } else if self_arg_kind == RustFunctionSelfArgKind::None {
                    Some("static")
                } else if self_arg_kind == RustFunctionSelfArgKind::MutRef {
                    if all_self_args.contains(&RustFunctionSelfArgKind::ConstRef) {
                        Some("mut")
                    } else {
                        None
                    }
                } else {
                    bail!("unsupported self arg kinds combination");
                };
            let other_caption = match *caption_strategy {
                RustFunctionCaptionStrategy::SelfOnly => None,
                RustFunctionCaptionStrategy::UnsafeOnly => unreachable!(),
                RustFunctionCaptionStrategy::SelfAndIndex => Some(index.to_string()),
                RustFunctionCaptionStrategy::SelfAndArgNames => {
                    if self.arguments.is_empty() {
                        Some("no_args".to_string())
                    } else {
                        Some(self.arguments.iter().map(|a| &a.name).join("_"))
                    }
                }
                RustFunctionCaptionStrategy::SelfAndArgTypes => {
                    if self.arguments.is_empty() {
                        Some("no_args".to_string())
                    } else {
                        Some(
                            self.arguments
                                .iter()
                                .filter(|t| &t.name != "self")
                                .map_if_ok(|t| t.argument_type.api_type.caption(context))?
                                .join("_"),
                        )
                    }
                }
            };
            let mut key_caption_items = Vec::new();
            if let Some(c) = self_arg_kind_caption {
                key_caption_items.push(c.to_string());
            }
            if let Some(c) = other_caption {
                key_caption_items.push(c);
            }
            if key_caption_items.is_empty() {
                None
            } else {
                Some(key_caption_items.join("_"))
            }
        };
        Ok(result)
    }
}

/// Information about a public API function.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustFunction {
    pub is_public: bool,

    /// True if the function is `unsafe`.
    pub is_unsafe: bool,
    /// Full name of the function.
    pub path: RustPath,

    pub kind: RustFunctionKind,

    /// List of arguments. For an overloaded function, only the arguments
    /// involved in the overloading are listed in this field.
    /// There can also be arguments shared by all variants (typically the
    /// `self` argument), and they are not listed in this field.
    pub arguments: Vec<RustFunctionArgument>,
    /// C++ and Rust return types at all levels.
    pub return_type: RustFinalType,

    /// Documentation data.
    pub extra_doc: Option<String>,
}

/// Information about type of `self` argument of the function.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub enum RustFunctionSelfArgKind {
    /// No `self` argument (static function or a free function).
    None,
    /// `&self` argument.
    ConstRef,
    /// `&mut self` argument.
    MutRef,
    /// `self` argument.
    Value,
}

/// Information about an associated type value
/// within a trait implementation.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustTraitAssociatedType {
    /// Name of the associated type.
    pub name: String,
    /// Value of the associated type.
    pub value: RustType,
}

/// Information about a trait implementation.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustTraitImpl {
    pub parent_path: RustPath,
    /// Type the trait is implemented for.
    pub target_type: RustType,
    /// Type of the trait.
    pub trait_type: RustType,
    /// Values of associated types of the trait.
    pub associated_types: Vec<RustTraitAssociatedType>,
    /// Functions that implement the trait.
    pub functions: Vec<RustFunction>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustModuleDoc {
    pub extra_doc: Option<String>,
    pub cpp_path: Option<CppPath>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum RustModuleKind {
    CrateRoot,
    Ffi,
    SizedTypes,
    CppNamespace,
    CppNestedType,
}

impl RustModuleKind {
    pub fn is_in_separate_file(self) -> bool {
        match self {
            RustModuleKind::CrateRoot => true,
            RustModuleKind::Ffi => false,
            RustModuleKind::SizedTypes => false,
            RustModuleKind::CppNamespace => true,
            RustModuleKind::CppNestedType => false,
        }
    }
}

/// Information about a Rust module.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustModule {
    pub is_public: bool,

    /// Path to the module.
    pub path: RustPath,
    /// Markdown content of Rust documentation for this module.
    pub doc: RustModuleDoc,

    pub kind: RustModuleKind,
}

/// Function of generating name suffixes for disambiguating multiple Rust functions
/// with the same name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RustFunctionCaptionStrategy {
    /// Only type of `self` is used.
    SelfOnly,
    /// Unsafe functions have `unsafe` suffix, and safe functions have no suffix.
    UnsafeOnly,
    /// Type of `self` and types of other arguments are used.
    SelfAndArgTypes,
    /// Type of `self` and names of other arguments are used.
    SelfAndArgNames,
    /// Type of `self` and index of function are used.
    SelfAndIndex,
}

impl RustFunctionCaptionStrategy {
    /// Returns list of all available strategies sorted by priority
    /// (more preferred strategies go first).
    #[allow(dead_code)]
    pub fn all() -> &'static [RustFunctionCaptionStrategy] {
        use self::RustFunctionCaptionStrategy::*;
        &[
            SelfOnly,
            UnsafeOnly,
            SelfAndArgTypes,
            SelfAndArgNames,
            SelfAndIndex,
        ]
    }
}

/// Information about an argument of a Rust FFI function.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RustFFIArgument {
    /// Name of the argument.
    pub name: String,
    /// Type of the argument.
    pub argument_type: RustType,
}

/// Information about a Rust FFI function.
/// Name and signature of this function must be the same
/// as the corresponding C++ function on the other side of FFI.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RustFFIFunction {
    /// Return type of the function.
    pub return_type: RustType,
    /// Name of the function.
    pub path: RustPath,
    /// Arguments of the function.
    pub arguments: Vec<RustFFIArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustFlagEnumImpl {
    pub enum_path: RustPath,
    pub qflags: RustPath,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RustItemKind {
    Module(RustModule),
    Struct(RustStruct),
    EnumValue(RustEnumValue),
    TraitImpl(RustTraitImpl),
    FlagEnumImpl(RustFlagEnumImpl),
    FfiFunction(RustFFIFunction), // TODO: merge FfiFunction and Function
    Function(RustFunction),
}

impl RustItemKind {
    pub fn is_ffi_function(&self) -> bool {
        if let RustItemKind::FfiFunction(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_wrapper_type(&self) -> bool {
        if let RustItemKind::Struct(data) = self {
            data.kind.is_wrapper_type()
        } else {
            false
        }
    }

    pub fn is_module(&self) -> bool {
        if let RustItemKind::Module(_) = self {
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustDatabaseItem {
    pub kind: RustItemKind,
    pub cpp_item_index: Option<usize>,
}

impl RustDatabaseItem {
    pub fn path(&self) -> Option<&RustPath> {
        match self.kind {
            RustItemKind::Module(ref data) => Some(&data.path),
            RustItemKind::Struct(ref data) => Some(&data.path),
            RustItemKind::EnumValue(ref data) => Some(&data.path),
            RustItemKind::Function(ref data) => Some(&data.path),
            RustItemKind::FfiFunction(ref data) => Some(&data.path),
            RustItemKind::TraitImpl(_) => None,
            RustItemKind::FlagEnumImpl(_) => None,
        }
    }
    pub fn is_child_of(&self, parent: &RustPath) -> bool {
        match self.kind {
            RustItemKind::TraitImpl(ref trait_impl) => &trait_impl.parent_path == parent,
            RustItemKind::FlagEnumImpl(ref data) => {
                data.enum_path.parent().as_ref() == Some(parent)
            }
            _ => {
                let path = self
                    .path()
                    .expect("item must have path because it's not a trait impl");
                path.is_child_of(parent)
            }
        }
    }

    pub fn as_module_ref(&self) -> Option<&RustModule> {
        if let RustItemKind::Module(ref data) = self.kind {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RustDatabase {
    pub items: Vec<RustDatabaseItem>,
}

impl RustDatabase {
    pub fn find(&self, path: &RustPath) -> Option<&RustDatabaseItem> {
        self.items.iter().find(|item| item.path() == Some(path))
    }

    pub fn children<'a>(
        &'a self,
        path: &'a RustPath,
    ) -> impl Iterator<Item = &'a RustDatabaseItem> {
        self.items.iter().filter(move |item| item.is_child_of(path))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustPathScope {
    pub path: RustPath,
    pub prefix: Option<String>,
}

impl RustPathScope {
    pub fn apply(&self, name: &str) -> RustPath {
        let full_name = if let Some(ref prefix) = self.prefix {
            format!("{}{}", prefix, name)
        } else {
            name.to_string()
        };
        self.path.join(full_name)
    }
}
