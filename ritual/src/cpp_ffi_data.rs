use crate::cpp_code_generator;
use crate::cpp_data::CppPath;
use crate::cpp_function::ReturnValueAllocationPlace;
use crate::cpp_type::{CppBuiltInNumericType, CppFunctionPointerType, CppType};
use crate::database::DatabaseClient;
use itertools::Itertools;
use ritual_common::errors::{bail, Result};
use serde_derive::{Deserialize, Serialize};

/// Variation of a field accessor method
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize)]
pub enum CppFieldAccessorType {
    /// Returns copy of the field
    CopyGetter,
    /// Returns const reference to the field
    ConstRefGetter,
    /// Returns mutable reference to the field
    MutRefGetter,
    /// Copies value from its argument to the field
    Setter,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub enum CppCast {
    Static {
        /// If true, this is an unsafe (from base to derived) `static_cast` wrapper.
        is_unsafe: bool,

        /// Contains index of the base (e.g. 0 for the first base; always
        /// 0 if the class only has one base).
        base_index: Option<usize>,
    },
    Dynamic,
    QObject,
}

impl CppCast {
    pub fn cpp_method_name(&self) -> &'static str {
        match *self {
            CppCast::Static { .. } => "static_cast",
            CppCast::Dynamic => "dynamic_cast",
            CppCast::QObject => "qobject_cast",
        }
    }

    pub fn is_unsafe_static_cast(&self) -> bool {
        match self {
            CppCast::Static { is_unsafe, .. } => *is_unsafe,
            _ => false,
        }
    }
    pub fn is_first_static_cast(&self) -> bool {
        match self {
            CppCast::Static { base_index, .. } => base_index.as_ref() == Some(&0),
            _ => false,
        }
    }
}

/// Information about real nature of a C++ FFI method.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum CppFfiFunctionKind {
    /// This is a real C++ function.
    Function,
    /// This is a field accessor, i.e. a non-existing getter or setter
    /// method for a public field.
    FieldAccessor {
        /// Type of the accessor
        accessor_type: CppFieldAccessorType,
    },
}

/// Relation between original C++ method's argument value
/// and corresponding FFI function's argument value
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum CppToFfiTypeConversion {
    /// Argument types are identical.
    NoChange,
    /// C++ argument is a class value (like QPoint)
    /// and FFI argument is a pointer (like QPoint*)
    ValueToPointer { is_ffi_const: bool },
    /// C++ argument is a reference (like QPoint&)
    /// and FFI argument is a pointer (like QPoint*)
    ReferenceToPointer,
    /// C++ argument is QFlags<T>
    /// and FFI argument is uint
    QFlagsToInt,
    /// Implicit conversion is used.
    ImplicitCast { ffi_type: CppType },
}

/// Information that indicates how an FFI function argument
/// should be interpreted
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum CppFfiArgumentMeaning {
    /// This argument contains value for "this" pointer
    /// used to call C++ class member functions
    This,
    /// Value of this argument should be passed as an argument to
    /// the original C++ method. Associated value is index of the
    /// C++ method's argument (counting from 0).
    Argument(usize),
    /// This argument receives pointer to the buffer where
    /// the return value should be transferred to using placement new.
    ReturnValue,
}

impl CppFfiArgumentMeaning {
    /// Checks if this argument corresponds to an original
    /// C++ method's argument
    pub fn is_argument(&self) -> bool {
        matches!(self, CppFfiArgumentMeaning::Argument(..))
    }
}

/// Representation of an argument of a FFI function
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CppFfiFunctionArgument {
    /// Identifier
    pub name: String,
    /// Type
    pub argument_type: CppFfiType,
    /// C++ equivalent
    pub meaning: CppFfiArgumentMeaning,
}

impl CppFfiFunctionArgument {
    /// Generates C++ code for the part of FFI function signature
    /// corresponding to this argument
    pub fn to_cpp_code(&self) -> Result<String> {
        if let CppType::FunctionPointer(..) = self.argument_type.ffi_type {
            Ok(self.argument_type.ffi_type.to_cpp_code(Some(&self.name))?)
        } else {
            Ok(format!(
                "{} {}",
                self.argument_type.ffi_type.to_cpp_code(None)?,
                self.name
            ))
        }
    }
}

/// Information about arguments and return type of a FFI function
/// with no final function name
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CppFfiFunction {
    /// List of arguments
    pub arguments: Vec<CppFfiFunctionArgument>,
    /// Return type
    pub return_type: CppFfiType,

    /// Allocation place method used for converting
    /// the return type of the method
    /// or used to determine implementation of the destructor
    pub allocation_place: ReturnValueAllocationPlace,

    /// Final name of FFI method
    pub path: CppPath,

    pub kind: CppFfiFunctionKind,
}

impl CppFfiFunction {
    /// Returns true if this signature has const this_ptr argument,
    /// indicating that original C++ method has const attribute.
    /// Returns false if there is no this argument or it's not const.
    pub fn has_const_this(&self) -> bool {
        self.arguments.iter().any(|arg| {
            arg.meaning == CppFfiArgumentMeaning::This
                && match arg.argument_type.ffi_type {
                    CppType::PointerLike { is_const, .. } => is_const,
                    _ => false,
                }
        })
    }

    pub fn has_same_kind(&self, other: &Self) -> bool {
        match &self.kind {
            CppFfiFunctionKind::Function { .. } => {
                matches!(&other.kind, CppFfiFunctionKind::Function { .. })
            }
            CppFfiFunctionKind::FieldAccessor { accessor_type, .. } => {
                if let CppFfiFunctionKind::FieldAccessor {
                    accessor_type: other_accessor_type,
                    ..
                } = &other.kind
                {
                    accessor_type == other_accessor_type
                } else {
                    false
                }
            }
        }
    }
}

/// FFI function type with attached information about
/// corresponding original C++ type and their relation
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CppFfiType {
    /// Original C++ type
    original_type: CppType,
    /// FFI function type
    ffi_type: CppType,
    /// Relation
    conversion: CppToFfiTypeConversion,
}

impl CppFfiType {
    pub fn new(original_type: CppType, conversion: CppToFfiTypeConversion) -> Result<Self> {
        match conversion.clone() {
            CppToFfiTypeConversion::NoChange => Ok(CppFfiType {
                ffi_type: original_type.clone(),
                original_type,
                conversion,
            }),
            CppToFfiTypeConversion::ValueToPointer { is_ffi_const } => Ok(CppFfiType {
                ffi_type: CppType::new_pointer(is_ffi_const, original_type.clone()),
                original_type,
                conversion,
            }),
            CppToFfiTypeConversion::ReferenceToPointer => {
                let target = original_type.pointer_like_to_target()?;
                let is_const = original_type.pointer_like_is_const()?;
                Ok(CppFfiType {
                    ffi_type: CppType::new_pointer(is_const, target.clone()),
                    original_type,
                    conversion,
                })
            }
            CppToFfiTypeConversion::QFlagsToInt => Ok(CppFfiType {
                ffi_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
                original_type,
                conversion,
            }),
            CppToFfiTypeConversion::ImplicitCast { ffi_type } => Ok(CppFfiType {
                original_type,
                ffi_type,
                conversion,
            }),
        }
    }

    /// Generates an object representing the void type
    pub fn void() -> Self {
        CppFfiType {
            original_type: CppType::Void,
            ffi_type: CppType::Void,
            conversion: CppToFfiTypeConversion::NoChange,
        }
    }

    pub fn original_type(&self) -> &CppType {
        &self.original_type
    }

    pub fn ffi_type(&self) -> &CppType {
        &self.ffi_type
    }

    pub fn conversion(&self) -> &CppToFfiTypeConversion {
        &self.conversion
    }
}

/// Information about a Qt signal wrapper with
/// certain signal arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QtSignalWrapper {
    pub signal_arguments: Vec<CppType>,
    /// Generated name of the wrapper class
    pub class_path: CppPath,
}

/// Information about a Qt slot wrapper with
/// certain slot arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QtSlotWrapper {
    pub signal_arguments: Vec<CppType>,
    /// Generated name of the wrapper class
    pub class_path: CppPath,
    /// Arguments of the slot.
    pub arguments: Vec<CppFfiType>,
    /// The function pointer type accepted by this wrapper
    pub function_type: CppFunctionPointerType,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CppFfiItem {
    Function(CppFfiFunction),
    QtSlotWrapper(QtSlotWrapper),
    QtSignalWrapper(QtSignalWrapper),
}

impl CppFfiItem {
    pub fn as_function_ref(&self) -> Option<&CppFfiFunction> {
        if let CppFfiItem::Function(data) = self {
            Some(data)
        } else {
            None
        }
    }

    pub fn is_function(&self) -> bool {
        matches!(self, CppFfiItem::Function(_))
    }

    pub fn as_slot_wrapper_ref(&self) -> Option<&QtSlotWrapper> {
        if let CppFfiItem::QtSlotWrapper(data) = self {
            Some(data)
        } else {
            None
        }
    }

    pub fn as_signal_wrapper_ref(&self) -> Option<&QtSignalWrapper> {
        if let CppFfiItem::QtSignalWrapper(data) = self {
            Some(data)
        } else {
            None
        }
    }

    pub fn is_slot_wrapper(&self) -> bool {
        matches!(self, CppFfiItem::QtSlotWrapper(_))
    }

    pub fn is_signal_wrapper(&self) -> bool {
        matches!(self, CppFfiItem::QtSignalWrapper(_))
    }

    pub fn short_text(&self) -> String {
        match self {
            CppFfiItem::Function(function) => function.path.to_cpp_pseudo_code(),
            CppFfiItem::QtSlotWrapper(slot_wrapper) => format!(
                "slot wrapper for ({})",
                slot_wrapper
                    .signal_arguments
                    .iter()
                    .map(CppType::to_cpp_pseudo_code)
                    .join(", ")
            ),
            CppFfiItem::QtSignalWrapper(signal_wrapper) => format!(
                "signal wrapper for ({})",
                signal_wrapper
                    .signal_arguments
                    .iter()
                    .map(CppType::to_cpp_pseudo_code)
                    .join(", ")
            ),
        }
    }

    pub fn has_same_kind(&self, other: &Self) -> bool {
        match self {
            CppFfiItem::Function(function) => {
                if let CppFfiItem::Function(other_function) = other {
                    function.has_same_kind(other_function)
                } else {
                    false
                }
            }
            CppFfiItem::QtSlotWrapper(wrapper) => {
                if let CppFfiItem::QtSlotWrapper(other_wrapper) = other {
                    wrapper.signal_arguments == other_wrapper.signal_arguments
                } else {
                    false
                }
            }
            CppFfiItem::QtSignalWrapper(wrapper) => {
                if let CppFfiItem::QtSignalWrapper(other_wrapper) = other {
                    wrapper.signal_arguments == other_wrapper.signal_arguments
                } else {
                    false
                }
            }
        }
    }

    pub fn path(&self) -> &CppPath {
        match self {
            CppFfiItem::Function(f) => &f.path,
            CppFfiItem::QtSlotWrapper(s) => &s.class_path,
            CppFfiItem::QtSignalWrapper(s) => &s.class_path,
        }
    }

    pub fn is_source_item(&self) -> bool {
        match self {
            CppFfiItem::Function(_) => false,
            CppFfiItem::QtSlotWrapper(_) | CppFfiItem::QtSignalWrapper(_) => true,
        }
    }

    pub fn source_item_cpp_code(&self, db: &DatabaseClient) -> Result<String> {
        match self {
            CppFfiItem::Function(_) => bail!("not a source item"),
            CppFfiItem::QtSlotWrapper(slot_wrapper) => {
                cpp_code_generator::qt_slot_wrapper(db, slot_wrapper)
            }
            CppFfiItem::QtSignalWrapper(signal_wrapper) => {
                cpp_code_generator::qt_signal_wrapper(db, signal_wrapper)
            }
        }
    }
}
