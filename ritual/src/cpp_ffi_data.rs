use crate::cpp_data::CppClassField;
use crate::cpp_data::CppPath;
use crate::cpp_function::{CppFunction, ReturnValueAllocationPlace};
use crate::cpp_type::{CppBuiltInNumericType, CppFunctionPointerType, CppType};
use ritual_common::errors::Result;
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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum CppCast {
    Static {
        /// If true, this is an unsafe (from base to derived) `static_cast` wrapper.
        is_unsafe: bool,

        /// Contains index of the base (e.g. 0 for the first base; always
        /// 0 if the class only has one base).
        base_index: usize,
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
            CppCast::Static { base_index, .. } => base_index == &0,
            _ => false,
        }
    }
}

/// Information about real nature of a C++ FFI method.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum CppFfiFunctionKind {
    /// This is a real C++ function.
    Function {
        cpp_function: CppFunction,
        /// If `Some`, the method is derived from another method by omitting arguments,
        /// and this field contains the number of omitted arguments.
        omitted_arguments: Option<usize>,
        /// If Some, this is an instance of `static_cast`, `dynamic_cast` or
        /// `qobject_cast` function call.
        cast: Option<CppCast>,
    },
    /// This is a field accessor, i.e. a non-existing getter or setter
    /// method for a public field.
    FieldAccessor {
        /// Type of the accessor
        accessor_type: CppFieldAccessorType,
        // /// Name of the C++ field
        field: CppClassField,
    },
}

impl CppFfiFunctionKind {
    pub fn cpp_function(&self) -> Option<&CppFunction> {
        if let CppFfiFunctionKind::Function { cpp_function, .. } = self {
            Some(cpp_function)
        } else {
            None
        }
    }
    pub fn cpp_field(&self) -> Option<&CppClassField> {
        if let CppFfiFunctionKind::FieldAccessor { field, .. } = self {
            Some(field)
        } else {
            None
        }
    }
}

/// Relation between original C++ method's argument value
/// and corresponding FFI function's argument value
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum CppTypeConversionToFfi {
    /// Argument types are identical
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
        match *self {
            CppFfiArgumentMeaning::Argument(..) => true,
            _ => false,
        }
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

    pub fn short_text(&self) -> String {
        match &self.kind {
            CppFfiFunctionKind::Function {
                cpp_function,
                omitted_arguments,
                ..
            } => {
                let omitted_args_text = if let Some(args) = omitted_arguments {
                    format!(" (omitted arguments: {}", args)
                } else {
                    String::new()
                };
                format!(
                    "FFI function call{}: {}",
                    omitted_args_text,
                    cpp_function.short_text()
                )
            }
            CppFfiFunctionKind::FieldAccessor {
                field,
                accessor_type,
            } => format!("FFI field {:?}: {}", accessor_type, field.short_text()),
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
    conversion: CppTypeConversionToFfi,
}

impl CppFfiType {
    pub fn new(original_type: CppType, conversion: CppTypeConversionToFfi) -> Result<Self> {
        match conversion {
            CppTypeConversionToFfi::NoChange => Ok(CppFfiType {
                ffi_type: original_type.clone(),
                original_type,
                conversion,
            }),
            CppTypeConversionToFfi::ValueToPointer { is_ffi_const } => Ok(CppFfiType {
                ffi_type: CppType::new_pointer(is_ffi_const, original_type.clone()),
                original_type,
                conversion,
            }),
            CppTypeConversionToFfi::ReferenceToPointer => {
                let target = original_type.pointer_like_to_target()?;
                let is_const = original_type.pointer_like_is_const()?;
                Ok(CppFfiType {
                    ffi_type: CppType::new_pointer(is_const, target.clone()),
                    original_type,
                    conversion,
                })
            }
            CppTypeConversionToFfi::QFlagsToInt => Ok(CppFfiType {
                ffi_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
                original_type,
                conversion,
            }),
        }
    }

    /// Generates an object representing the void type
    pub fn void() -> Self {
        CppFfiType {
            original_type: CppType::Void,
            ffi_type: CppType::Void,
            conversion: CppTypeConversionToFfi::NoChange,
        }
    }

    pub fn original_type(&self) -> &CppType {
        &self.original_type
    }

    pub fn ffi_type(&self) -> &CppType {
        &self.ffi_type
    }

    pub fn conversion(&self) -> CppTypeConversionToFfi {
        self.conversion
    }
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
    // /// String identifier passed to `QObject::connect` function to
    // /// specify the object's slot.
    //pub receiver_id: String,
}
