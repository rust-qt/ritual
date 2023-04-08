//! Types for handling information about C++ types.

use crate::cpp_data::CppPath;
use ritual_common::errors::{bail, Result};
use serde_derive::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

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
pub enum CppSpecificNumericTypeKind {
    Integer { is_signed: bool },
    FloatingPoint,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppSpecificNumericType {
    /// Type identifier (most likely a typedef name)
    pub path: CppPath,
    /// Size of type in bits
    pub bits: usize,
    /// Information about the type (float or integer,
    /// signed or unsigned)
    pub kind: CppSpecificNumericTypeKind,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CppTemplateParameter {
    /// Template instantiation level. For example,
    /// if there is a template class and a template method in it,
    /// the class's template parameters will have level = 0 and
    /// the method's template parameters will have level = 1.
    /// If only the class or only the method is a template,
    /// the level will be 0.
    pub nested_level: usize,
    /// Index of the parameter. In `QHash<K, V>` `"K"` has `index = 0`
    /// and `"V"` has `index = 1`.
    pub index: usize,
    /// Declared name of this template parameter
    pub name: String,
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
    PointerSizedInteger { path: CppPath, is_signed: bool },
    /// Enum type
    Enum {
        /// Name, including namespaces and nested classes
        path: CppPath,
    },
    /// Class type
    Class(CppPath),
    /// Template parameter, like `"T"` anywhere inside
    /// `QVector<T>` declaration
    TemplateParameter(CppTemplateParameter),
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
        matches!(self, Float | Double | LongDouble)
    }

    /// Returns true if this type is a signed integer.
    pub fn is_signed_integer(&self) -> bool {
        use self::CppBuiltInNumericType::*;
        matches!(self, SChar | Short | Int | Long | LongLong | Int128)
    }

    /// Returns true if this type is an unsigned integer.
    pub fn is_unsigned_integer(&self) -> bool {
        use self::CppBuiltInNumericType::*;
        matches!(
            self,
            UChar | Char16 | Char32 | UShort | UInt | ULong | ULongLong | UInt128
        )
    }

    /// Returns true if this type is integer but may be signed or
    /// unsigned, depending on the platform.
    pub fn is_integer_with_undefined_signedness(&self) -> bool {
        use self::CppBuiltInNumericType::*;
        matches!(self, Char | WChar)
    }

    /// Returns all supported types.
    pub fn all() -> &'static [CppBuiltInNumericType] {
        use self::CppBuiltInNumericType::*;
        &[
            Bool, Char, SChar, UChar, WChar, Char16, Char32, Short, UShort, Int, UInt, Long, ULong,
            LongLong, ULongLong, Int128, UInt128, Float, Double, LongDouble,
        ]
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

    /// Returns true if this is `void` type.
    pub fn is_void(&self) -> bool {
        matches!(self, CppType::Void)
    }
    /// Returns true if this is a class type.
    pub fn is_class(&self) -> bool {
        matches!(self, CppType::Class(..))
    }
    /// Returns true if this is a template parameter.
    pub fn is_template_parameter(&self) -> bool {
        matches!(self, CppType::TemplateParameter { .. })
    }
    /// Returns true if this is a function pointer.
    pub fn is_function_pointer(&self) -> bool {
        matches!(self, CppType::FunctionPointer(..))
    }

    pub fn is_pointer(&self) -> bool {
        match self {
            CppType::PointerLike { kind, .. } => *kind == CppPointerLikeTypeKind::Pointer,
            _ => false,
        }
    }

    /// Returns true if this is a template parameter or a type that
    /// contains any template parameters.
    pub fn is_or_contains_template_parameter(&self) -> bool {
        match self {
            CppType::TemplateParameter { .. } => true,
            CppType::PointerLike { target, .. } => target.is_or_contains_template_parameter(),
            CppType::FunctionPointer(type1) => {
                type1.return_type.is_or_contains_template_parameter()
                    || type1
                        .arguments
                        .iter()
                        .any(CppType::is_or_contains_template_parameter)
            }
            CppType::Class(path) => path.items().iter().any(|item| {
                if let Some(template_arguments) = &item.template_arguments {
                    template_arguments
                        .iter()
                        .any(CppType::is_or_contains_template_parameter)
                } else {
                    false
                }
            }),
            _ => false,
        }
    }

    pub fn contains_template_parameter(&self, param: &CppTemplateParameter) -> bool {
        match self {
            CppType::TemplateParameter(self_params) => {
                self_params.nested_level == param.nested_level && self_params.index == param.index
            }
            CppType::PointerLike { target, .. } => target.contains_template_parameter(param),
            CppType::FunctionPointer(type1) => {
                type1.return_type.contains_template_parameter(param)
                    || type1
                        .arguments
                        .iter()
                        .any(|t| t.contains_template_parameter(param))
            }
            CppType::Class(path) => path.items().iter().any(|item| {
                if let Some(template_arguments) = &item.template_arguments {
                    template_arguments
                        .iter()
                        .any(|t| t.contains_template_parameter(param))
                } else {
                    false
                }
            }),
            _ => false,
        }
    }

    /// Returns C++ code representing this type.
    pub fn to_cpp_code(&self, function_pointer_inner_text: Option<&str>) -> Result<String> {
        if !self.is_function_pointer() && function_pointer_inner_text.is_some() {
            bail!("unexpected function_pointer_inner_text");
        }
        match self {
            CppType::Void => Ok("void".to_string()),
            CppType::BuiltInNumeric(t) => Ok(t.to_cpp_code().to_string()),
            CppType::Enum { path }
            | CppType::SpecificNumeric(CppSpecificNumericType { path, .. })
            | CppType::PointerSizedInteger { path, .. } => path.to_cpp_code(),
            CppType::Class(path) => path.to_cpp_code(),
            CppType::TemplateParameter { .. } => {
                bail!("template parameters are not allowed in C++ code generator");
            }
            CppType::FunctionPointer(CppFunctionPointerType {
                return_type,
                arguments,
                allows_variadic_arguments,
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
                kind,
                is_const,
                target,
            } => Ok(format!(
                "{}{} {}",
                target.to_cpp_code(function_pointer_inner_text)?,
                if *is_const { " const" } else { "" },
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
        match self {
            CppType::TemplateParameter(param) => {
                return param.name.to_string();
            }
            CppType::Class(base) => return base.to_cpp_pseudo_code(),
            CppType::FunctionPointer(..) => {
                return self
                    .to_cpp_code(Some("FN_PTR"))
                    .unwrap_or_else(|_| "[?]".to_string());
            }
            CppType::PointerLike {
                kind,
                is_const,
                target,
            } => {
                return format!(
                    "{}{}{}",
                    if *is_const { "const " } else { "" },
                    target.to_cpp_pseudo_code(),
                    match *kind {
                        CppPointerLikeTypeKind::Pointer => "*",
                        CppPointerLikeTypeKind::Reference => "&",
                        CppPointerLikeTypeKind::RValueReference => "&&",
                    }
                );
            }
            _ => {}
        };
        self.to_cpp_code(None).unwrap_or_else(|_| "[?]".to_string())
    }

    pub fn ascii_caption(&self) -> String {
        match self {
            CppType::Void | CppType::BuiltInNumeric(_) => {
                self.to_cpp_code(None).unwrap().replace(' ', "_")
            }
            CppType::SpecificNumeric(data) => data.path.ascii_caption(),
            CppType::PointerSizedInteger { path, .. }
            | CppType::Enum { path }
            | CppType::Class(path) => path.ascii_caption(),
            CppType::TemplateParameter(param) => param.name.to_string(),
            CppType::FunctionPointer(_) => "fn".into(),
            CppType::PointerLike {
                kind,
                is_const,
                target,
            } => format!(
                "{}{}{}",
                target.ascii_caption(),
                if *is_const { "_const" } else { "" },
                match *kind {
                    CppPointerLikeTypeKind::Pointer => "_ptr",
                    CppPointerLikeTypeKind::Reference => "_ref",
                    CppPointerLikeTypeKind::RValueReference => "_rref",
                },
            ),
        }
    }

    pub fn pointer_like_to_target(&self) -> Result<&CppType> {
        if let CppType::PointerLike { target, .. } = self {
            Ok(target)
        } else {
            bail!("not a pointer like type");
        }
    }

    pub fn pointer_like_is_const(&self) -> Result<bool> {
        if let CppType::PointerLike { is_const, .. } = self {
            Ok(*is_const)
        } else {
            bail!("not a pointer like type");
        }
    }

    pub fn as_function_pointer(&self) -> Option<&CppFunctionPointerType> {
        if let CppType::FunctionPointer(t) = self {
            Some(t)
        } else {
            None
        }
    }
}

/// Context of usage for a C++ type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CppTypeRole {
    /// This type is used as a function's return type
    ReturnType,
    /// This type is not used as a function's return type
    NotReturnType,
}

pub fn is_qflags(path: &CppPath) -> bool {
    path.last().name == "QFlags"
        && !path.has_parent()
        && path
            .last()
            .template_arguments
            .as_ref()
            .map_or(false, |args| args.len() == 1)
}

impl CppType {
    pub fn contains_reference(&self) -> bool {
        if let CppType::PointerLike { kind, target, .. } = self {
            match *kind {
                CppPointerLikeTypeKind::Pointer => target.contains_reference(),
                CppPointerLikeTypeKind::Reference | CppPointerLikeTypeKind::RValueReference => true,
            }
        } else {
            false
        }
    }

    /// Attempts to replace template types at `nested_level`
    /// within this type with `template_arguments1`.
    #[allow(clippy::if_not_else)]
    pub fn instantiate(
        &self,
        nested_level: usize,
        template_arguments1: &[CppType],
    ) -> Result<CppType> {
        match self {
            CppType::TemplateParameter(param) => {
                if param.nested_level == nested_level {
                    if param.index >= template_arguments1.len() {
                        bail!("not enough template arguments");
                    }
                    Ok(template_arguments1[param.index].clone())
                } else {
                    Ok(self.clone())
                }
            }
            CppType::Class(type1) => Ok(CppType::Class(
                type1.instantiate(nested_level, template_arguments1)?,
            )),
            CppType::PointerLike {
                kind,
                is_const,
                target,
            } => Ok(CppType::PointerLike {
                kind: kind.clone(),
                is_const: *is_const,
                target: Box::new(target.instantiate(nested_level, template_arguments1)?),
            }),
            _ => Ok(self.clone()),
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
impl Hash for CppSpecificNumericType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bits.hash(state);
        self.kind.hash(state);
    }
}
