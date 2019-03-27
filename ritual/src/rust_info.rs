//! Types holding information about generates Rust API.

use crate::cpp_data::{CppPath, CppTypeDoc};
use crate::cpp_ffi_data::CppFfiFunction;
use crate::cpp_type::CppType;
use crate::rust_code_generator::rust_type_to_code;
use crate::rust_type::{RustFinalType, RustPath, RustPointerLikeTypeKind, RustType};
use ritual_common::errors::{bail, Result};
use ritual_common::string_utils::ends_with_digit;
use serde_derive::{Deserialize, Serialize};

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
    pub signal_arguments: Vec<CppType>,
    pub raw_slot_wrapper: RustPath,
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
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
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
        /// C++ name of the signal or slot
        cpp_path: CppPath,
        /// Type of the receiver.
        receiver_type: RustQtReceiverType,
        /// Identifier of the signal or slot for passing to `QObject::connect`.
        receiver_id: String,

        qobject_path: RustPath,
    },
}

impl RustFunctionKind {
    pub fn short_text(&self) -> String {
        match self {
            RustFunctionKind::FfiWrapper(data) => data.cpp_ffi_function.short_text(),
            RustFunctionKind::CppDeletableImpl { .. } => format!("{:?}", self),
            RustFunctionKind::SignalOrSlotGetter { cpp_path, .. } => {
                format!("SignalOrSlotGetter({}", cpp_path.to_cpp_pseudo_code())
            }
        }
    }
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
    pub fn self_arg_kind(&self) -> Result<RustFunctionSelfArgKind> {
        if let Some(arg) = self.arguments.get(0) {
            if arg.name == "self" {
                match arg.argument_type.api_type() {
                    RustType::PointerLike { kind, is_const, .. } => match *kind {
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

    /*/// Generates name suffix for this function using `caption_strategy`.
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
                                .map_if_ok(|t| t.argument_type.api_type().caption(context))?
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
    }*/
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
            RustModuleKind::CrateRoot | RustModuleKind::CppNamespace => true,
            RustModuleKind::Ffi | RustModuleKind::SizedTypes | RustModuleKind::CppNestedType => {
                false
            }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RustFunctionCaptionStrategy {
    pub mut_: bool,
    pub args_count: bool,
    pub arg_names: bool,
    pub arg_types: bool,
    pub static_: bool,
}

impl RustFunctionCaptionStrategy {
    /// Returns list of all available strategies sorted by priority
    /// (more preferred strategies go first).
    #[allow(dead_code)]
    pub fn all() -> Vec<Self> {
        use self::RustFunctionCaptionStrategy as S;

        let mut all = Vec::new();
        all.push(S {
            mut_: true,
            ..S::default()
        });

        let other = &[
            S {
                args_count: true,
                ..S::default()
            },
            S {
                arg_names: true,
                ..S::default()
            },
            S {
                arg_types: true,
                ..S::default()
            },
            S {
                static_: true,
                ..S::default()
            },
        ];

        for item in other {
            all.push(item.clone());
            all.push(S {
                mut_: true,
                ..item.clone()
            });
        }

        //        all.push(S {
        //            index: true,
        //            ..S::default()
        //        });

        all
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
pub struct RustRawSlotReceiver {
    pub target_path: RustPath,
    pub arguments: RustType,
    pub receiver_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RustExtraImplKind {
    FlagEnum { enum_path: RustPath },
    RawSlotReceiver(RustRawSlotReceiver),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustExtraImpl {
    pub parent_path: RustPath,
    pub kind: RustExtraImplKind,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RustItem {
    Module(RustModule),
    Struct(RustStruct),
    EnumValue(RustEnumValue),
    TraitImpl(RustTraitImpl),
    ExtraImpl(RustExtraImpl),
    FfiFunction(RustFFIFunction), // TODO: merge FfiFunction and Function
    Function(RustFunction),
}

impl RustItem {
    pub fn is_ffi_function(&self) -> bool {
        if let RustItem::FfiFunction(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_wrapper_type(&self) -> bool {
        if let RustItem::Struct(data) = self {
            data.kind.is_wrapper_type()
        } else {
            false
        }
    }

    pub fn is_module(&self) -> bool {
        if let RustItem::Module(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_module_for_nested(&self) -> bool {
        if let RustItem::Module(module) = self {
            module.kind == RustModuleKind::CppNestedType
        } else {
            false
        }
    }

    pub fn short_text(&self) -> String {
        match self {
            RustItem::Module(data) => format!("mod {}", data.path.full_name(None)),
            RustItem::Struct(data) => format!("struct {}", data.path.full_name(None)),
            RustItem::EnumValue(data) => format!("enum value {}", data.path.full_name(None)),
            RustItem::TraitImpl(data) => format!(
                "impl {} for {}",
                rust_type_to_code(&data.trait_type, None),
                rust_type_to_code(&data.target_type, None)
            ),
            RustItem::ExtraImpl(data) => format!("extra impl {:?}", data.kind),
            RustItem::FfiFunction(data) => format!("ffi fn {}", data.path.full_name(None)),
            RustItem::Function(data) => format!("fn {}", data.path.full_name(None)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustDatabaseItem {
    pub item: RustItem,
    pub cpp_item_index: Option<usize>,
    pub ffi_item_index: Option<usize>,
}

impl RustDatabaseItem {
    pub fn path(&self) -> Option<&RustPath> {
        match &self.item {
            RustItem::Module(data) => Some(&data.path),
            RustItem::Struct(data) => Some(&data.path),
            RustItem::EnumValue(data) => Some(&data.path),
            RustItem::Function(data) => Some(&data.path),
            RustItem::FfiFunction(data) => Some(&data.path),
            RustItem::TraitImpl(_) | RustItem::ExtraImpl(_) => None,
        }
    }
    pub fn is_child_of(&self, parent: &RustPath) -> bool {
        match &self.item {
            RustItem::TraitImpl(trait_impl) => &trait_impl.parent_path == parent,
            RustItem::ExtraImpl(data) => &data.parent_path == parent,
            _ => {
                let path = self
                    .path()
                    .expect("item must have path because it's not a trait impl");
                path.is_child_of(parent)
            }
        }
    }

    pub fn as_module_ref(&self) -> Option<&RustModule> {
        if let RustItem::Module(data) = &self.item {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RustDatabase {
    items: Vec<RustDatabaseItem>,
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

    pub fn items(&self) -> &[RustDatabaseItem] {
        &self.items
    }

    pub fn add_item(&mut self, item: RustDatabaseItem) {
        self.items.push(item);
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn make_unique_path(&self, path: &RustPath) -> RustPath {
        let mut number = None;
        let mut path_try = path.clone();
        loop {
            if let Some(number) = number {
                *path_try.last_mut() = format!(
                    "{}{}{}",
                    path.last(),
                    if ends_with_digit(path.last()) {
                        "_"
                    } else {
                        ""
                    },
                    number
                );
            }
            if self.find(&path_try).is_none() {
                return path_try;
            }

            number = Some(number.unwrap_or(1) + 1);
        }
        // TODO: check for conflicts with types from crate template (how?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustPathScope {
    pub path: RustPath,
    pub prefix: Option<String>,
}

impl RustPathScope {
    pub fn apply(&self, name: &str) -> RustPath {
        let full_name = if let Some(prefix) = &self.prefix {
            format!("{}{}", prefix, name)
        } else {
            name.to_string()
        };
        self.path.join(full_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameType<'a> {
    Type,
    EnumValue,
    Module,
    FfiFunction,
    ApiFunction(&'a CppFfiFunction),
    ReceiverFunction {
        receiver_type: RustQtReceiverType,
    },
    SizedItem,
    QtSlotWrapper {
        signal_arguments: Vec<CppType>,
        is_public: bool,
    },
}

impl NameType<'_> {
    pub fn is_api_function(&self) -> bool {
        match self {
            NameType::ApiFunction(_) => true,
            _ => false,
        }
    }
}
