//! Types holding information about generates Rust API.

use cpp_ffi_data::CppAndFfiMethod;
use cpp_type::CppType;
use rust_type::{RustName, CompleteType, RustType};
use cpp_method::CppMethodDoc;
use cpp_data::CppTypeDoc;
use std::path::PathBuf;

/// One variant of a Rust enum
#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct RustEnumValue {
  /// Identifier
  pub name: String,
  /// Corresponding value
  pub value: i64,
  /// Documentation of corresponding C++ variants
  pub cpp_docs: Vec<CppEnumValueDocItem>,
  /// True if this variant was added because enums with
  /// one variant are not supported
  pub is_dummy: bool,
}


/// C++ documentation data for a enum variant
#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct CppEnumValueDocItem {
  /// C++ name of the variant
  pub variant_name: String,
  /// HTML content
  pub doc: Option<String>,
}

/// Information about a Qt slot wrapper on Rust side
#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct RustQtSlotWrapper {
  /// Argument types of the slot
  pub arguments: Vec<CompleteType>,
  /// Identifier of the slot for `QObject::connect`
  pub receiver_id: String,
  /// Name of the public Rust struct of this wrapper
  pub public_type_name: String,
  /// Name of the extern callback function of this wrapper
  pub callback_name: String,
}

/// Information about a Rust type wrapper
#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub enum RustTypeWrapperKind {
  /// Enum wrapper
  Enum {
    /// List of enum values
    values: Vec<RustEnumValue>,
    /// True if `FlaggableEnum` trait is implemented
    /// for this type, i.e. if `QFlags<T>` with this C++ type
    /// is used in API.
    is_flaggable: bool,
  },
  /// Struct wrapper
  Struct {
    /// Name of the constant containing size of the corresponding
    /// C++ type in bytes. Value of the constant is determined at
    /// crate compile time.
    /// If `None`, this struct can only be used as pointer, like an
    /// empty enum.
    size_const_name: Option<String>,
    /// True if `CppDeletable` trait is implemented
    /// for this type, i.e. if this C++ type has public destructor
    /// and type allocation place was set to `Heap`.
    is_deletable: bool,
    /// Additional information for a Qt slot wrapper struct
    slot_wrapper: Option<RustQtSlotWrapper>,
  },
}

/// Exported information about a Rust wrapper type
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct RustProcessedTypeInfo {
  /// Full name of corresponding C++ type (class or enum).
  pub cpp_name: String,
  /// C++ documentation for this type
  pub cpp_doc: Option<CppTypeDoc>,
  /// Template arguments. None if C++ type is not a template class.
  pub cpp_template_arguments: Option<Vec<CppType>>,
  /// Kind of the type and additional information.
  pub kind: RustTypeWrapperKind,
  /// Identifier of Rust type
  pub rust_name: RustName,
  /// Indicates whether this type is public
  pub is_public: bool,
}



/// Exported information about generated crate
/// for future use of it as a dependency. This information
/// is saved to the cache directory but not to the
/// output crate directory, so the crate's build script
/// cannot access it (as opposed to `BuildScriptData`).
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct RustExportInfo {
  /// Name of the crate
  pub crate_name: String,
  /// Version of the crate
  pub crate_version: String,
  /// Directory with the generated crate
  pub output_path: String,
  /// List of generated types
  pub rust_types: Vec<RustProcessedTypeInfo>,
}

/// Information for generating Rust documentation for a method
/// or an item of information for an overloaded method.
/// One value of `RustMethodDocItem` corresponds to a single
/// C++ method.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustMethodDocItem {
  /// C++ documentation of the corresponding C++ method.
  pub doc: Option<CppMethodDoc>,
  /// Pseudo-code illustrating Rust argument types. There may be
  /// multiple Rust variants for one C++ method if that method's
  /// arguments have default values.
  pub rust_fns: Vec<String>,
  /// C++ code containing declaration of the corresponding C++ method.
  pub cpp_fn: String,
}


/// Location of a Rust method.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustMethodScope {
  /// Inside `impl T {}`, where `T` is `target_type`.
  Impl { target_type: RustType },
  /// Inside a trait implementation.
  TraitImpl,
  /// A free function.
  Free,
}

/// Information about a Rust method argument.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustMethodArgument {
  /// C++ and Rust types corresponding to this argument at all levels.
  pub argument_type: CompleteType,
  /// Rust argument name.
  pub name: String,
  /// Index of the corresponding argument of the FFI function.
  pub ffi_index: usize,
}

/// Information about arguments of a Rust method without overloading
/// or one variant of an overloaded method.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustMethodArgumentsVariant {
  /// List of arguments. For an overloaded method, only the arguments
  /// involved in the overloading are listed in this field.
  /// There can also be arguments shared by all variants (typically the
  /// `self` argument), and they are not listed in this field.
  pub arguments: Vec<RustMethodArgument>,
  /// C++ method corresponding to this variant.
  pub cpp_method: CppAndFfiMethod,
  /// Index of the FFI function argument used for acquiring the return value,
  /// if any. `None` if the return value is passed normally (as the return value
  /// of the FFI function).
  pub return_type_ffi_index: Option<usize>,
  /// C++ and Rust return types at all levels.
  pub return_type: CompleteType,
}

/// Arguments of a Rust method
#[derive(Debug, PartialEq, Eq, Clone)]
#[allow(dead_code)]
pub enum RustMethodArguments {
  /// Method without overloading
  SingleVariant(RustMethodArgumentsVariant),
  /// Method with overloading emulation
  MultipleVariants {
    /// Last name of the parameters trait
    params_trait_name: String,
    /// Lifetime name of the parameters trait, if any.
    params_trait_lifetime: Option<String>,
    /// Return type of all variants, or `None` if they have different return types.
    common_return_type: Option<RustType>,
    /// Arguments that don't participate in overloading
    /// (typically `self` argument, if present).
    shared_arguments: Vec<RustMethodArgument>,
    /// Name of the argument receiving overloaded values.
    variant_argument_name: String,
    /// Fully qualified name of the corresponding C++ method
    /// (used for generating documentation).
    cpp_method_name: String,
  },
}

/// Information about a public API method.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustMethod {
  /// Location of the method.
  pub scope: RustMethodScope,
  /// True if the method is `unsafe`.
  pub is_unsafe: bool,
  /// Name of the method. For free functions, this is the full name.
  /// for `impl` methods, this is only the method's own name.
  pub name: RustName,
  /// Arguments of the method.
  pub arguments: RustMethodArguments,
  /// Documentation data (one item per corresponding C++ method).
  pub docs: Vec<RustMethodDocItem>,
}

/// Information about type of `self` argument of the method.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum RustMethodSelfArgKind {
  /// No `self` argument (static method or a free function).
  None,
  /// `&self` argument.
  ConstRef,
  /// `&mut self` argument.
  MutRef,
  /// `self` argument.
  Value,
}

/// Additional information about a trait implementation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TraitImplExtra {
  /// For `CppDeletable` trait implementation,
  /// `deleter_name` contains name of the FFI function used as deleter.
  CppDeletable { deleter_name: String },
}

/// Information about an associated type value
/// within a trait implementation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TraitAssociatedType {
  /// Name of the associated type.
  pub name: String,
  /// Value of the associated type.
  pub value: RustType,
}

/// Information about a trait implementation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TraitImpl {
  /// Type the trait is implemented for.
  pub target_type: RustType,
  /// Type of the trait.
  pub trait_type: RustType,
  /// Values of associated types of the trait.
  pub associated_types: Vec<TraitAssociatedType>,
  /// Extra information about the implementation.
  pub extra: Option<TraitImplExtra>,
  /// List of methods.
  pub methods: Vec<RustMethod>,
}

/// Type of a receiver in Qt connection system.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustQtReceiverType {
  Signal,
  Slot,
}

/// Declaration of a Qt receiver type providing access to a signal
/// or a slot of a built-in Qt class.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustQtReceiverDeclaration {
  /// Name of the type.
  pub type_name: String,
  /// Name of the method in `Signals` or `Slots` type that
  /// creates an object of this type.
  pub method_name: String,
  /// Type of the receiver.
  pub receiver_type: RustQtReceiverType,
  /// Identifier of the signal or slot for passing to `QObject::connect`.
  pub receiver_id: String,
  /// Types or arguments.
  pub arguments: Vec<RustType>,
}

/// Part of the information about a Rust type declaration.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RustTypeDeclarationKind {
  /// Information about a Rust type for which a corresponding C++ type exists.
  CppTypeWrapper {
    /// Information about the wrapper properties.
    kind: RustTypeWrapperKind,
    /// Fully qualified name of the C++ type.
    cpp_type_name: String,
    /// Values of template arguments of the C++ type.
    /// `None` if the C++ type is not a template type.
    cpp_template_arguments: Option<Vec<CppType>>,
    /// C++ documentation of the type.
    cpp_doc: Option<CppTypeDoc>,
    /// Methods in direct `impl` for this type.
    methods: Vec<RustMethod>,
    /// Trait implementations for this type.
    trait_impls: Vec<TraitImpl>,
    /// List of Qt receiver types for signals and slots of
    /// this C++ type.
    qt_receivers: Vec<RustQtReceiverDeclaration>,
  },
  /// Information about a Rust trait created for overloading emulation.
  MethodParametersTrait {
    /// Name of the lifetime parameter of the trait and all references within it,
    /// or `None` if there are no references within it.
    lifetime: Option<String>,
    /// If true, the method of the trait is `unsafe`.
    is_unsafe: bool,
    /// Common arguments of all method variants (typically the `self` argument
    /// if present).
    shared_arguments: Vec<RustMethodArgument>,
    /// Common return type of all variants, or `None` if return types differ.
    common_return_type: Option<RustType>,
    /// List of argument variants for which the trait must be implemented.
    impls: Vec<RustMethodArgumentsVariant>,
    /// Scope of the public API method this trait was created for
    /// (used for generating documentation).
    method_scope: RustMethodScope,
    /// Name of the public API method this trait was created for
    /// (used for generating documentation).
    method_name: RustName,
  },
}

/// Information about a Rust type declaration.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustTypeDeclaration {
  /// True if this type should be declared with `pub`.
  pub is_public: bool,
  /// Full name of the type.
  pub name: RustName,
  /// Additional information depending on kind of the type.
  pub kind: RustTypeDeclarationKind,
}

/// Information about a Rust module.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RustModule {
  /// Last name of the module.
  pub name: String,
  /// Type declarations.
  /// Each type may also contain its own functions
  /// and trait implementations.
  pub types: Vec<RustTypeDeclaration>,
  /// Free functions within the module.
  pub functions: Vec<RustMethod>,
  /// Trait implementations associated with free functions.
  pub trait_impls: Vec<TraitImpl>,
  /// Markdown content of Rust documentation for this module.
  pub doc: Option<String>,
  /// Submodules of this module.
  pub submodules: Vec<RustModule>,
}

/// Information about a loaded dependency.
#[derive(Debug, Clone)]
pub struct DependencyInfo {
  /// Information loaded from the cache directory of this dependency.
  pub rust_export_info: RustExportInfo,
  /// Cache directory of this dependency.
  pub cache_path: PathBuf,
}

/// Method of generating name suffixes for disambiguating multiple Rust methods
/// with the same name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustMethodCaptionStrategy {
  /// Only type of `self` is used.
  SelfOnly,
  /// Unsafe methods have `unsafe` suffix, and safe methods have no suffix.
  UnsafeOnly,
  /// Type of `self` and types of other arguments are used.
  SelfAndArgTypes,
  /// Type of `self` and names of other arguments are used.
  SelfAndArgNames,
  /// Type of `self` and index of method are used.
  SelfAndIndex,
}

impl RustMethodCaptionStrategy {
  /// Returns list of all available strategies sorted by priority
  /// (more preferred strategies go first).
  pub fn all() -> &'static [RustMethodCaptionStrategy] {
    use self::RustMethodCaptionStrategy::*;
    const LIST: &'static [RustMethodCaptionStrategy] = &[SelfOnly,
                                                         UnsafeOnly,
                                                         SelfAndArgTypes,
                                                         SelfAndArgNames,
                                                         SelfAndIndex];
    return LIST;
  }
}


impl RustProcessedTypeInfo {
  /// Implements sanity check of the data.
  /// Returns true if this type was properly declared within any of the modules.
  pub fn is_declared_in(&self, modules: &[RustModule]) -> bool {
    for module in modules {
      if module
           .types
           .iter()
           .any(|t| match t.kind {
                  RustTypeDeclarationKind::CppTypeWrapper {
                    ref cpp_type_name,
                    ref cpp_template_arguments,
                    ..
                  } => {
                    cpp_type_name == &self.cpp_name &&
                    cpp_template_arguments == &self.cpp_template_arguments
                  }
                  _ => false,
                }) {
        return true;
      }
      if self.is_declared_in(&module.submodules) {
        return true;
      }
    }
    false
  }
}

/// Information about an argument of a Rust FFI function.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RustFFIArgument {
  /// Name of the argument.
  pub name: String,
  /// Type of the argument.
  pub argument_type: RustType,
}

/// Information about a Rust FFI function.
/// Name and signature of this function must be the same
/// as the corresponding C++ function on the other side of FFI.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RustFFIFunction {
  /// Return type of the function.
  pub return_type: RustType,
  /// Name of the function.
  pub name: String,
  /// Arguments of the function.
  pub arguments: Vec<RustFFIArgument>,
}
