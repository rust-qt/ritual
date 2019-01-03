//! Types for handling information about C++ types.

use crate::common::errors::{bail, Result, ResultExt};
use crate::common::string_utils::JoinWithSeparator;
use crate::cpp_data::CppName;
use crate::cpp_ffi_data::{CppFfiType, CppTypeConversionToFfi};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub enum CppPointerLikeTypeKind {
    Pointer,
    Reference,
    RValueReference,
}

/// Available built-in C++ numeric types.
/// All these types have corresponding
/// `clang::TypeKind` values (except for `CharS` and `CharU`
/// which map to `CppBuiltInNumericType::Char`)
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub enum CppBuiltInNumericType {
    Bool,
    Char,
    SChar,
    UChar,
    WChar,
    Char16,
    Char32,
    Short,
    UShort,
    Int,
    UInt,
    Long,
    ULong,
    LongLong,
    ULongLong,
    Int128,
    UInt128,
    Float,
    Double,
    LongDouble,
}

/// Information about a fixed-size primitive type
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum CppSpecificNumericTypeKind {
    Integer { is_signed: bool },
    FloatingPoint,
}

/// Information about base C++ class type
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppClassType {
    /// Name, including namespaces and nested classes
    pub name: CppName,
    /// For template classes, C++ types used as template
    /// arguments in this type,
    /// like [QString, int] in QHash<QString, int>
    pub template_arguments: Option<Vec<CppType>>,
}

/// Information about a C++ function pointer type
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppFunctionPointerType {
    /// Return type of the function
    pub return_type: Box<CppType>,
    /// Arguments of the function
    pub arguments: Vec<CppType>,
    /// Whether arguments are terminated with "..."
    pub allows_variadic_arguments: bool,
}

/// Information about a numeric C++ type that is
/// guaranteed to be the same on all platforms,
/// e.g. `uint32_t`.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct CppSpecificNumericType {
    /// Type identifier (most likely a typedef name)
    pub name: CppName,
    /// Size of type in bits
    pub bits: usize,
    /// Information about the type (float or integer,
    /// signed or unsigned)
    pub kind: CppSpecificNumericTypeKind,
}

/// Base C++ type. `CppType` can add indirection
/// and constness to `CppTypeBase`, but otherwise
/// this enum lists all supported types.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub enum CppType {
    /// Void
    Void,
    /// Built-in C++ primitive type, like int
    BuiltInNumeric(CppBuiltInNumericType),
    /// Fixed-size primitive type, like qint64 or int64_t
    /// (may be translated to Rust's i64)
    SpecificNumeric(CppSpecificNumericType),
    /// Pointer sized integer, like qintptr
    /// (may be translated to Rust's isize)
    PointerSizedInteger { name: CppName, is_signed: bool },
    /// Enum type
    Enum {
        /// Name, including namespaces and nested classes
        name: CppName,
    },
    /// Class type
    Class(CppClassType),
    /// Template parameter, like `"T"` anywhere inside
    /// `QVector<T>` declaration
    TemplateParameter {
        /// Template instantiation level. For example,
        /// if there is a template class and a template method in it,
        /// the class's template parameters will have level = 0 and
        /// the method's template parameters will have level = 1.
        /// If only the class or only the method is a template,
        /// the level will be 0.
        nested_level: usize,
        /// Index of the parameter. In `QHash<K, V>` `"K"` has `index = 0`
        /// and `"V"` has `index = 1`.
        index: usize,

        /// Declared name of this template parameter
        name: String,
    },
    /// Function pointer type
    FunctionPointer(CppFunctionPointerType),
    PointerLike {
        kind: CppPointerLikeTypeKind,
        is_const: bool,
        target: Box<CppType>,
    },
}

impl CppBuiltInNumericType {
    /// Returns C++ code representing this type.
    pub fn to_cpp_code(&self) -> &'static str {
        use self::CppBuiltInNumericType::*;
        match *self {
            Bool => "bool",
            Char => "char",
            SChar => "signed char",
            UChar => "unsigned char",
            WChar => "wchar_t",
            Char16 => "char16_t",
            Char32 => "char32_t",
            Short => "short",
            UShort => "unsigned short",
            Int => "int",
            UInt => "unsigned int",
            Long => "long",
            ULong => "unsigned long",
            LongLong => "long long",
            ULongLong => "unsigned long long",
            Int128 => "__int128_t",
            UInt128 => "__uint128_t",
            Float => "float",
            Double => "double",
            LongDouble => "long double",
        }
    }

    /// Returns true if this type is some sort of floating point type.
    pub fn is_float(&self) -> bool {
        use self::CppBuiltInNumericType::*;
        match *self {
            Float | Double | LongDouble => true,
            _ => false,
        }
    }

    /// Returns true if this type is a signed integer.
    pub fn is_signed_integer(&self) -> bool {
        use self::CppBuiltInNumericType::*;
        match *self {
            SChar | Short | Int | Long | LongLong | Int128 => true,
            _ => false,
        }
    }

    /// Returns true if this type is an unsigned integer.
    pub fn is_unsigned_integer(&self) -> bool {
        use self::CppBuiltInNumericType::*;
        match *self {
            UChar | Char16 | Char32 | UShort | UInt | ULong | ULongLong | UInt128 => true,
            _ => false,
        }
    }

    /// Returns true if this type is integer but may be signed or
    /// unsigned, depending on the platform.
    pub fn is_integer_with_undefined_signedness(&self) -> bool {
        use self::CppBuiltInNumericType::*;
        match *self {
            Char | WChar => true,
            _ => false,
        }
    }

    /// Returns all supported types.
    pub fn all() -> &'static [CppBuiltInNumericType] {
        use self::CppBuiltInNumericType::*;
        static LIST: &'static [CppBuiltInNumericType] = &[
            Bool, Char, SChar, UChar, WChar, Char16, Char32, Short, UShort, Int, UInt, Long, ULong,
            LongLong, ULongLong, Int128, UInt128, Float, Double, LongDouble,
        ];
        return LIST;
    }
}

impl CppClassType {
    /// Returns C++ code representing this type.
    pub fn to_cpp_code(&self) -> Result<String> {
        match self.template_arguments {
            Some(ref args) => {
                let mut arg_texts = Vec::new();
                for arg in args {
                    arg_texts.push(arg.to_cpp_code(None)?);
                }
                Ok(format!("{}< {} >", self.name, arg_texts.join(", ")))
            }
            None => Ok(self.name.to_cpp_code()),
        }
    }

    /// Returns string representation of this type for debugging output.
    pub fn to_cpp_pseudo_code(&self) -> String {
        if let Some(ref template_arguments) = self.template_arguments {
            format!(
                "{}<{}>",
                self.name,
                template_arguments
                    .iter()
                    .map(|x| x.to_cpp_pseudo_code())
                    .join(", ")
            )
        } else {
            self.name.to_cpp_code()
        }
    }

    /// Attempts to replace template types at `nested_level1`
    /// within this type with `template_arguments1`.
    pub fn instantiate(
        &self,
        nested_level1: usize,
        template_arguments1: &[CppType],
    ) -> Result<CppClassType> {
        Ok(CppClassType {
            name: self.name.clone(),
            template_arguments: match self.template_arguments {
                Some(ref template_arguments) => {
                    let mut args = Vec::new();
                    for arg in template_arguments {
                        args.push(arg.instantiate(nested_level1, template_arguments1)?);
                    }
                    Some(args)
                }
                None => None,
            },
        })
    }
}

impl CppType {
    pub fn new_pointer(is_const: bool, target: CppType) -> Self {
        CppType::PointerLike {
            kind: CppPointerLikeTypeKind::Pointer,
            is_const,
            target: Box::new(target),
        }
    }

    pub fn new_reference(is_const: bool, target: CppType) -> Self {
        CppType::PointerLike {
            kind: CppPointerLikeTypeKind::Reference,
            is_const,
            target: Box::new(target),
        }
    }

    #[allow(dead_code)]
    /// Returns true if this is `void` type.
    pub fn is_void(&self) -> bool {
        match *self {
            CppType::Void => true,
            _ => false,
        }
    }
    /// Returns true if this is a class type.
    pub fn is_class(&self) -> bool {
        match *self {
            CppType::Class(..) => true,
            _ => false,
        }
    }
    /// Returns true if this is a template parameter.
    pub fn is_template_parameter(&self) -> bool {
        match *self {
            CppType::TemplateParameter { .. } => true,
            _ => false,
        }
    }
    /// Returns true if this is a function pointer.
    pub fn is_function_pointer(&self) -> bool {
        match *self {
            CppType::FunctionPointer(..) => true,
            _ => false,
        }
    }
    /// Returns true if this is a template parameter or a type that
    /// contains any template parameters.
    pub fn is_or_contains_template_parameter(&self) -> bool {
        match *self {
            CppType::TemplateParameter { .. } => true,
            CppType::PointerLike { ref target, .. } => target.is_or_contains_template_parameter(),
            CppType::FunctionPointer(ref type1) => {
                type1.return_type.is_or_contains_template_parameter()
                    || type1
                        .arguments
                        .iter()
                        .any(|arg| arg.is_or_contains_template_parameter())
            }
            CppType::Class(CppClassType {
                ref template_arguments,
                ..
            }) => {
                if let Some(ref template_arguments) = *template_arguments {
                    template_arguments
                        .iter()
                        .any(|arg| arg.is_or_contains_template_parameter())
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Returns C++ code representing this type.
    pub fn to_cpp_code(&self, function_pointer_inner_text: Option<&str>) -> Result<String> {
        if !self.is_function_pointer() && function_pointer_inner_text.is_some() {
            bail!("unexpected function_pointer_inner_text");
        }
        match *self {
            CppType::Void => Ok("void".to_string()),
            CppType::BuiltInNumeric(ref t) => Ok(t.to_cpp_code().to_string()),
            CppType::Enum { ref name }
            | CppType::SpecificNumeric(CppSpecificNumericType { ref name, .. })
            | CppType::PointerSizedInteger { ref name, .. } => Ok(name.to_cpp_code()),
            //      CppTypeBase::SpecificNumeric { ref name, .. } => Ok(name.clone()),
            //      CppTypeBase::PointerSizedInteger { ref name, .. } => Ok(name.clone()),
            CppType::Class(ref info) => info.to_cpp_code(),
            CppType::TemplateParameter { .. } => {
                bail!("template parameters are not allowed in C++ code generator");
            }
            CppType::FunctionPointer(CppFunctionPointerType {
                ref return_type,
                ref arguments,
                ref allows_variadic_arguments,
            }) => {
                if *allows_variadic_arguments {
                    bail!("function pointers with variadic arguments are not supported");
                }
                let mut arg_texts = Vec::new();
                for arg in arguments {
                    arg_texts.push(arg.to_cpp_code(None)?);
                }
                if let Some(function_pointer_inner_text) = function_pointer_inner_text {
                    Ok(format!(
                        "{} (*{})({})",
                        return_type.as_ref().to_cpp_code(None)?,
                        function_pointer_inner_text,
                        arg_texts.join(", ")
                    ))
                } else {
                    bail!("function_pointer_inner_text argument is missing");
                }
            }
            CppType::PointerLike {
                ref kind,
                ref is_const,
                ref target,
            } => Ok(format!(
                "{}{}{}",
                if *is_const { "const " } else { "" },
                target.to_cpp_code(function_pointer_inner_text)?,
                match *kind {
                    CppPointerLikeTypeKind::Pointer => "*",
                    CppPointerLikeTypeKind::Reference => "&",
                    CppPointerLikeTypeKind::RValueReference => "&&",
                }
            )),
        }
    }

    /// Generates string representation of this type
    /// for debugging output.
    pub fn to_cpp_pseudo_code(&self) -> String {
        match *self {
            CppType::TemplateParameter { ref name, .. } => {
                return name.to_string(); // format!("T{}_{}", nested_level, index);
            }
            CppType::Class(ref base) => return base.to_cpp_pseudo_code(),
            CppType::FunctionPointer(..) => {
                return self
                    .to_cpp_code(Some(&"FN_PTR".to_string()))
                    .unwrap_or_else(|_| "[?]".to_string())
            }
            _ => {}
        };
        self.to_cpp_code(None).unwrap_or_else(|_| "[?]".to_string())
    }
}

/// Context of usage for a C++ type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CppTypeRole {
    /// This type is used as a function's return type
    ReturnType,
    /// This type is not used as a function's return type
    NotReturnType,
}

impl CppType {
    fn contains_reference(&self) -> bool {
        if let CppType::PointerLike {
            ref kind,
            ref target,
            ..
        } = *self
        {
            match *kind {
                CppPointerLikeTypeKind::Pointer => target.contains_reference(),
                CppPointerLikeTypeKind::Reference | CppPointerLikeTypeKind::RValueReference => true,
            }
        } else {
            false
        }
    }

    /// Converts this C++ type to its adaptation for FFI interface,
    /// removing all features not supported by C ABI
    /// (e.g. references and passing objects by value).
    #[cfg_attr(feature = "clippy", allow(collapsible_if))]
    pub fn to_cpp_ffi_type(&self, role: CppTypeRole) -> Result<CppFfiType> {
        let inner = || -> Result<CppFfiType> {
            if self.is_or_contains_template_parameter() {
                bail!("template parameters cannot be expressed in FFI");
            }
            match self {
                CppType::FunctionPointer(CppFunctionPointerType {
                    ref return_type,
                    ref arguments,
                    ref allows_variadic_arguments,
                }) => {
                    if *allows_variadic_arguments {
                        bail!("function pointers with variadic arguments are not supported");
                    }
                    let mut all_types: Vec<&CppType> = arguments.iter().collect();
                    all_types.push(return_type.as_ref());
                    for arg in all_types {
                        match *arg {
                            CppType::FunctionPointer(..) => {
                                // TODO: also ban pointers to function pointers
                                bail!(
                                    "function pointers containing nested function pointers are \
                                     not supported"
                                );
                            }
                            CppType::Class(..) => {
                                bail!(
                                    "Function pointers containing classes by value are not \
                                     supported"
                                );
                            }
                            _ => {}
                        }
                        if arg.contains_reference() {
                            bail!("Function pointers containing references are not supported");
                        }
                    }
                    return Ok(CppFfiType {
                        ffi_type: self.clone(),
                        conversion: CppTypeConversionToFfi::NoChange,
                        original_type: self.clone(),
                    });
                }
                CppType::Class(ref type1) => {
                    if type1.name == CppName::from_one_part("QFlags") {
                        return Ok(CppFfiType {
                            ffi_type: CppType::BuiltInNumeric(CppBuiltInNumericType::UInt),
                            conversion: CppTypeConversionToFfi::QFlagsToUInt,
                            original_type: self.clone(),
                        });
                    } else {
                        return Ok(CppFfiType {
                            ffi_type: CppType::PointerLike {
                                is_const: role != CppTypeRole::ReturnType,
                                kind: CppPointerLikeTypeKind::Pointer,
                                target: Box::new(self.clone()),
                            },
                            conversion: CppTypeConversionToFfi::ValueToPointer,
                            original_type: self.clone(),
                        });
                    }
                }
                CppType::PointerLike {
                    ref kind,
                    ref is_const,
                    ref target,
                } => {
                    match *kind {
                        CppPointerLikeTypeKind::Pointer => {}
                        CppPointerLikeTypeKind::Reference => {
                            if *is_const {
                                if let CppType::Class(ref type1) = **target {
                                    if type1.name == CppName::from_one_part("QFlags") {
                                        return Ok(CppFfiType {
                                            ffi_type: CppType::BuiltInNumeric(
                                                CppBuiltInNumericType::UInt,
                                            ),
                                            // TODO: use a separate conversion type (QFlagsConstRefToUInt)?
                                            conversion: CppTypeConversionToFfi::QFlagsToUInt,
                                            original_type: self.clone(),
                                        });
                                    }
                                }
                            }
                            return Ok(CppFfiType {
                                ffi_type: CppType::PointerLike {
                                    is_const: *is_const,
                                    kind: CppPointerLikeTypeKind::Pointer,
                                    target: target.clone(),
                                },
                                conversion: CppTypeConversionToFfi::ReferenceToPointer,
                                original_type: self.clone(),
                            });
                        }
                        CppPointerLikeTypeKind::RValueReference => {
                            bail!("rvalue references are not supported");
                        }
                    }
                }
                _ => {}
            }
            Ok(CppFfiType {
                ffi_type: self.clone(),
                conversion: CppTypeConversionToFfi::NoChange,
                original_type: self.clone(),
            })
        };
        Ok(inner().with_context(|_| format!("Can't express type to FFI: {:?}", self))?)
    }

    /// Attempts to replace template types at `nested_level1`
    /// within this type with `template_arguments1`.
    #[cfg_attr(feature = "clippy", allow(if_not_else))]
    pub fn instantiate(
        &self,
        nested_level1: usize,
        template_arguments1: &[CppType],
    ) -> Result<CppType> {
        match self {
            CppType::TemplateParameter {
                nested_level,
                index,
                ..
            } => {
                if *nested_level == nested_level1 {
                    if *index >= template_arguments1.len() {
                        bail!("not enough template arguments");
                    }
                    Ok(template_arguments1[*index].clone())
                } else {
                    Ok(self.clone())
                }
            }
            CppType::Class(ref type1) => Ok(CppType::Class(
                type1.instantiate(nested_level1, template_arguments1)?,
            )),
            CppType::PointerLike {
                ref kind,
                ref is_const,
                ref target,
            } => Ok(CppType::PointerLike {
                kind: kind.clone(),
                is_const: *is_const,
                target: Box::new(target.instantiate(nested_level1, template_arguments1)?),
            }),
            _ => Ok(self.clone()),
        }
    }

    /// Returns true if this type is platform dependent.
    /// Built-in numeric types that can have different size and/or
    /// signedness on different platforms are considered platform dependent.
    /// Any types that refer to a platform dependent type are also
    /// platform dependent.
    ///
    /// Note that most types are platform dependent in "binary" sense,
    /// i.e. their size and memory layout may vary, but this function
    /// does not address this property.
    pub fn is_platform_dependent(&self) -> bool {
        match self {
            CppType::Class(CppClassType {
                ref template_arguments,
                ..
            }) => {
                if let Some(ref template_arguments) = *template_arguments {
                    for arg in template_arguments {
                        if arg.is_platform_dependent() {
                            return true;
                        }
                    }
                }
                false
            }
            CppType::BuiltInNumeric(ref data) => data != &CppBuiltInNumericType::Bool,
            CppType::PointerLike { ref target, .. } => target.is_platform_dependent(),
            _ => false,
        }
    }
}

impl PartialEq for CppSpecificNumericType {
    fn eq(&self, other: &CppSpecificNumericType) -> bool {
        // name field is ignored
        self.bits == other.bits && self.kind == other.kind
    }
}
impl Eq for CppSpecificNumericType {}
