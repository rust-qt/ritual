//! Types holding information about generates Rust API.

use crate::cpp_data::CppPath;
use crate::cpp_ffi_data::CppFfiFunction;
use crate::cpp_type::CppType;
use crate::database::DbItem;
use crate::rust_code_generator::{rust_common_type_to_code, rust_type_to_code};
use crate::rust_type::{
    RustCommonType, RustFinalType, RustPath, RustPointerLikeTypeKind, RustType,
};
use ritual_common::errors::{bail, Result};
use serde_derive::{Deserialize, Serialize};

/// One variant of a Rust enum
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustEnumValue {
    pub path: RustPath,
    /// Corresponding value
    pub value: i64,
}

/// Information about a Qt slot wrapper on Rust side
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustQtSlotWrapper {
    /// Argument types of the slot
    pub arguments: Vec<RustFinalType>,
    pub raw_slot_wrapper: RustPath,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RustWrapperTypeKind {
    EnumWrapper,
    ImmovableClassWrapper,
    MovableClassWrapper { sized_type_path: RustPath },
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustSizedType {
    pub cpp_path: CppPath,
}

/// Information about a Rust type wrapper
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RustStructKind {
    WrapperType(RustWrapperTypeKind),
    QtSlotWrapper(RustQtSlotWrapper),
    SizedType(RustSizedType),
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

    pub fn has_same_kind(&self, other: &Self) -> bool {
        match self {
            RustStructKind::WrapperType(_) => {
                if let RustStructKind::WrapperType(_) = other {
                    true
                } else {
                    false
                }
            }
            RustStructKind::QtSlotWrapper(_) => {
                if let RustStructKind::QtSlotWrapper(_) = other {
                    true
                } else {
                    false
                }
            }
            RustStructKind::SizedType(_) => {
                if let RustStructKind::SizedType(_) = other {
                    true
                } else {
                    false
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustRawQtSlotWrapperData {
    pub arguments: Vec<RustType>,
    pub closure_wrapper: RustPath,
}

/// Exported information about a Rust wrapper type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustStruct {
    pub path: RustPath,
    /// Kind of the type and additional information.
    pub kind: RustStructKind,
    /// Indicates whether this type is public
    pub is_public: bool,

    pub raw_slot_wrapper_data: Option<RustRawQtSlotWrapperData>,
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
    pub ffi_function_path: RustPath,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustSignalOrSlotGetter {
    /// Type of the receiver.
    pub receiver_type: RustQtReceiverType,
    /// Identifier of the signal or slot for passing to `QObject::connect`.
    pub receiver_id: String,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RustFunctionKind {
    FfiWrapper(RustFfiWrapperData),
    SignalOrSlotGetter(RustSignalOrSlotGetter),
    FfiFunction,
}

impl RustFunctionKind {
    pub fn short_text(&self) -> String {
        match self {
            RustFunctionKind::FfiWrapper(data) => {
                format!("FfiWrapper({})", data.ffi_function_path.last())
            }
            RustFunctionKind::SignalOrSlotGetter(_) => "SignalOrSlotGetter".to_string(),
            RustFunctionKind::FfiFunction => "FfiFunction".to_string(),
        }
    }

    pub fn is_ffi_function(&self) -> bool {
        if let RustFunctionKind::FfiFunction = self {
            true
        } else {
            false
        }
    }

    pub fn is_signal_or_slot_getter(&self) -> bool {
        if let RustFunctionKind::SignalOrSlotGetter(_) = self {
            true
        } else {
            false
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
}

/// Information about type of `self` argument of the function.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize)]
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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RustTraitImplExtraKind {
    Normal,
    Deref,
    DerefMut,
}

/// Information about a trait implementation.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustTraitImpl {
    pub parent_path: RustPath,
    /// Type the trait is implemented for.
    pub target_type: RustType,
    /// Type of the trait.
    pub trait_type: RustCommonType,
    /// Values of associated types of the trait.
    pub associated_types: Vec<RustTraitAssociatedType>,
    /// Functions that implement the trait.
    pub functions: Vec<RustFunction>,
    pub extra_kind: RustTraitImplExtraKind,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub enum RustSpecialModuleKind {
    CrateRoot,
    Ffi,
    Ops,
    SizedTypes,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum RustModuleKind {
    Special(RustSpecialModuleKind),
    CppNamespace,
    CppNestedTypes,
}

impl RustModuleKind {
    pub fn is_in_separate_file(self) -> bool {
        match self {
            RustModuleKind::Special(kind) => match kind {
                RustSpecialModuleKind::CrateRoot => true,
                RustSpecialModuleKind::Ffi => false,
                RustSpecialModuleKind::Ops => true,
                RustSpecialModuleKind::SizedTypes => false,
            },
            RustModuleKind::CppNamespace { .. } => true,
            RustModuleKind::CppNestedTypes { .. } => false,
        }
    }

    pub fn is_cpp_nested_types(self) -> bool {
        if let RustModuleKind::CppNestedTypes { .. } = self {
            true
        } else {
            false
        }
    }
}

/// Information about a Rust module.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RustModule {
    pub is_public: bool,
    /// Path to the module.
    pub path: RustPath,
    pub kind: RustModuleKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RustTypeCaptionStrategy {
    LastName,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RustFunctionCaptionStrategy {
    pub mut_: bool,
    pub args_count: bool,
    pub arg_names: bool,
    pub arg_types: Option<RustTypeCaptionStrategy>,
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
                static_: true,
                ..S::default()
            },
            S {
                arg_types: Some(RustTypeCaptionStrategy::LastName),
                ..S::default()
            },
            S {
                arg_types: Some(RustTypeCaptionStrategy::LastName),
                static_: true,
                ..S::default()
            },
            S {
                arg_types: Some(RustTypeCaptionStrategy::Full),
                ..S::default()
            },
            S {
                arg_types: Some(RustTypeCaptionStrategy::Full),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustRawSlotReceiver {
    pub target_path: RustPath,
    pub arguments: RustType,
    pub receiver_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustFlagEnumImpl {
    pub enum_path: RustPath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RustExtraImplKind {
    FlagEnum(RustFlagEnumImpl),
    RawSlotReceiver(RustRawSlotReceiver),
}

impl RustExtraImplKind {
    pub fn has_same_kind(&self, other: &Self) -> bool {
        match self {
            RustExtraImplKind::FlagEnum(_) => {
                if let RustExtraImplKind::FlagEnum(_) = other {
                    true
                } else {
                    false
                }
            }
            RustExtraImplKind::RawSlotReceiver(_) => {
                if let RustExtraImplKind::RawSlotReceiver(_) = other {
                    true
                } else {
                    false
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustExtraImpl {
    pub parent_path: RustPath,
    pub kind: RustExtraImplKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RustReexportSource {
    DependencyCrate { crate_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustReexport {
    pub path: RustPath,
    pub target: RustPath,
    pub source: RustReexportSource,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RustItem {
    Module(RustModule),
    Struct(RustStruct),
    EnumValue(RustEnumValue),
    TraitImpl(RustTraitImpl),
    ExtraImpl(RustExtraImpl),
    Function(RustFunction),
    Reexport(RustReexport),
}

impl RustItem {
    pub fn path(&self) -> Option<&RustPath> {
        match self {
            RustItem::Module(data) => Some(&data.path),
            RustItem::Struct(data) => Some(&data.path),
            RustItem::EnumValue(data) => Some(&data.path),
            RustItem::Function(data) => Some(&data.path),
            RustItem::Reexport(data) => Some(&data.path),
            RustItem::TraitImpl(_) | RustItem::ExtraImpl(_) => None,
        }
    }

    pub fn parent_path(&self) -> Result<RustPath> {
        match self {
            RustItem::TraitImpl(trait_impl) => Ok(trait_impl.parent_path.clone()),
            RustItem::ExtraImpl(data) => Ok(data.parent_path.clone()),
            _ => self
                .path()
                .expect("item must have path because it's not a trait impl")
                .parent(),
        }
    }

    pub fn parent_path_parts(&self) -> Result<&[String]> {
        match self {
            RustItem::TraitImpl(trait_impl) => Ok(trait_impl.parent_path.parts()),
            RustItem::ExtraImpl(data) => Ok(data.parent_path.parts()),
            _ => self
                .path()
                .expect("item must have path because it's not a trait impl")
                .parent_parts(),
        }
    }

    pub fn is_child_of(&self, parent: &RustPath) -> bool {
        self.parent_path_parts().ok() == Some(parent.parts())
    }

    pub fn as_module_ref(&self) -> Option<&RustModule> {
        if let RustItem::Module(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_struct_ref(&self) -> Option<&RustStruct> {
        if let RustItem::Struct(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_enum_value_ref(&self) -> Option<&RustEnumValue> {
        if let RustItem::EnumValue(data) = self {
            Some(data)
        } else {
            None
        }
    }
    pub fn as_reexport_ref(&self) -> Option<&RustReexport> {
        if let RustItem::Reexport(value) = self {
            Some(value)
        } else {
            None
        }
    }
    pub fn as_function_ref(&self) -> Option<&RustFunction> {
        if let RustItem::Function(value) = self {
            Some(value)
        } else {
            None
        }
    }
    pub fn as_trait_impl_ref(&self) -> Option<&RustTraitImpl> {
        if let RustItem::TraitImpl(value) = self {
            Some(value)
        } else {
            None
        }
    }
    pub fn as_extra_impl_ref(&self) -> Option<&RustExtraImpl> {
        if let RustItem::ExtraImpl(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub fn has_same_kind(&self, other: &Self) -> bool {
        match self {
            RustItem::Module(data) => {
                if let RustItem::Module(other) = other {
                    data.kind == other.kind
                } else {
                    false
                }
            }
            RustItem::Struct(data) => {
                if let RustItem::Struct(other) = other {
                    data.kind.has_same_kind(&other.kind)
                } else {
                    false
                }
            }
            RustItem::EnumValue(_) => {
                if let RustItem::EnumValue(_) = other {
                    true
                } else {
                    false
                }
            }
            RustItem::TraitImpl(data) => {
                if let RustItem::TraitImpl(other) = other {
                    data.extra_kind == other.extra_kind
                } else {
                    false
                }
            }
            RustItem::ExtraImpl(data) => {
                if let RustItem::ExtraImpl(other) = other {
                    data.kind.has_same_kind(&other.kind)
                } else {
                    false
                }
            }
            RustItem::Function(data) => match &data.kind {
                RustFunctionKind::FfiWrapper(_) => {
                    if let RustItem::Function(other) = other {
                        if let RustFunctionKind::FfiWrapper(_) = &other.kind {
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                RustFunctionKind::SignalOrSlotGetter(_) => {
                    if let RustItem::Function(other) = other {
                        if let RustFunctionKind::SignalOrSlotGetter(_) = &other.kind {
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                RustFunctionKind::FfiFunction => {
                    if let RustItem::Function(other) = other {
                        if let RustFunctionKind::FfiFunction = &other.kind {
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
            },
            RustItem::Reexport(data) => {
                if let RustItem::Reexport(other) = other {
                    data.source == other.source
                } else {
                    false
                }
            }
        }
    }

    pub fn is_module(&self) -> bool {
        if let RustItem::Module(_) = self {
            true
        } else {
            false
        }
    }
    pub fn is_trait_impl(&self) -> bool {
        if let RustItem::TraitImpl(_) = self {
            true
        } else {
            false
        }
    }
    pub fn is_ffi_function(&self) -> bool {
        if let RustItem::Function(function) = self {
            function.kind.is_ffi_function()
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
    pub fn is_module_for_nested(&self) -> bool {
        if let RustItem::Module(module) = self {
            module.kind.is_cpp_nested_types()
        } else {
            false
        }
    }

    pub fn is_crate_root(&self) -> bool {
        if let RustItem::Module(module) = self {
            module.kind == RustModuleKind::Special(RustSpecialModuleKind::CrateRoot)
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
                rust_common_type_to_code(&data.trait_type, None),
                rust_type_to_code(&data.target_type, None)
            ),
            RustItem::ExtraImpl(data) => format!("extra impl {:?}", data.kind),
            RustItem::Function(data) => format!("fn {}", data.path.full_name(None)),
            RustItem::Reexport(data) => format!(
                "use {} as {}",
                data.path.full_name(None),
                data.target.last()
            ),
        }
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
    ApiFunction(DbItem<&'a CppFfiFunction>),
    ReceiverFunction {
        receiver_type: RustQtReceiverType,
    },
    SizedItem,
    QtSlotWrapper {
        signal_arguments: &'a [CppType],
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
