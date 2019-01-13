#![allow(dead_code)]

use crate::cpp_ffi_data::CppTypeConversionToFfi;
use crate::cpp_type::CppType;
use ritual_common::errors::{bail, Result};
use ritual_common::string_utils::CaseOperations;
use ritual_common::utils::MapIfOk;
use serde_derive::{Deserialize, Serialize};

/// Rust identifier. Represented by
/// a vector of name parts. For a regular name,
/// first part is name of the crate,
/// last part is own name of the entity,
/// and intermediate names are module names.
/// Built-in types are represented
/// by a single vector item, like `vec!["i32"]`.
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustPath {
    /// Parts of the name
    pub parts: Vec<String>,
}

impl RustPath {
    /// Creates new `RustPath` consisting of `parts`.
    pub fn from_parts(parts: Vec<String>) -> RustPath {
        if parts.is_empty() {
            panic!("RustPath can't be empty");
        }
        RustPath { parts }
    }

    /// Returns crate name of this name, or `None`
    /// if this name does not contain the crate name (e.g. it's a built-in type).
    pub fn crate_name(&self) -> Option<&str> {
        if self.parts.is_empty() {
            panic!("RustPath can't be empty");
        }
        if self.parts.len() > 1 {
            Some(self.parts[0].as_str())
        } else {
            None
        }
    }

    /// Returns last component of the name.
    pub fn last(&self) -> &str {
        self.parts.last().expect("RustPath can't be empty")
    }

    pub fn join(&self, name: impl Into<String>) -> RustPath {
        let mut new_path = self.clone();
        new_path.parts.push(name.into());
        new_path
    }

    /// Returns formatted name for using within `current_crate`.
    /// If `current_crate` is `None`, it's assumed that the formatted name
    /// will be used outside of the crate it belongs to.
    pub fn full_name(&self, current_crate: Option<&str>) -> String {
        if let Some(current_crate) = current_crate {
            if let Some(self_crate) = self.crate_name() {
                if self_crate == current_crate {
                    return format!("crate::{}", self.parts[1..].join("::"));
                }
            }
        }
        format!("::{}", self.parts.join("::"))
    }

    /// Returns true if `other` is nested within `self`.
    pub fn includes(&self, other: &RustPath) -> bool {
        let extra_modules_count = other.parts.len() as isize - self.parts.len() as isize;
        extra_modules_count > 0 && other.parts[0..self.parts.len()] == self.parts[..]
    }

    /// Returns true if `other` is a direct child of `self`.
    pub fn includes_directly(&self, other: &RustPath) -> bool {
        let extra_modules_count = other.parts.len() as isize - self.parts.len() as isize;
        self.includes(other) && extra_modules_count == 1
    }

    pub fn is_child_of(&self, parent: &RustPath) -> bool {
        parent.includes_directly(self)
    }

    pub fn parent(&self) -> Option<RustPath> {
        if self.parts.len() > 1 {
            let mut new_path = self.clone();
            new_path.parts.pop().unwrap();
            Some(new_path)
        } else {
            None
        }
    }
}

/// Conversion from public Rust API type to
/// the corresponding FFI type
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum RustToFfiTypeConversion {
    /// Types are the same
    None,
    /// `&T` to `*const T` (or similar mutable types)
    RefToPtr,
    /// `Option<&T>` to `*const T` (or similar mutable types)
    OptionRefToPtr,
    /// `T` to `*const T` (or similar mutable type)
    ValueToPtr,
    /// `CppBox<T>` to `*const T` (or similar mutable type)
    CppBoxToPtr,
    /// `qt_core::flags::Flags<T>` to `libc::c_uint`
    QFlagsToUInt,
}

/// Information about a completely processed type
/// including its variations at each processing step.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompleteType {
    /// Original C++ type used in the C++ library's API
    pub original_cpp_type: CppType,
    /// C++ type used in the C++ wrapper library's API
    pub cpp_ffi_type: CppType,
    /// Conversion from `cpp_type` to `cpp_ffi_type`
    pub cpp_to_ffi_conversion: CppTypeConversionToFfi,
    /// Rust type used in FFI functions
    /// (must be exactly the same as `cpp_ffi_type`)
    pub rust_ffi_type: RustType,
    /// Type used in public Rust API
    pub rust_api_type: RustType,
    /// Conversion from `rust_api_type` to `rust_ffi_type`
    pub rust_api_to_ffi_conversion: RustToFfiTypeConversion,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RustPointerLikeTypeKind {
    // `*mut T` or `*const T`
    Pointer,
    // `&'lifetime T` or `&'lifetime mut T`
    Reference { lifetime: Option<String> },
}

impl RustPointerLikeTypeKind {
    pub fn is_pointer(&self) -> bool {
        match *self {
            RustPointerLikeTypeKind::Pointer => true,
            _ => false,
        }
    }

    pub fn is_ref(&self) -> bool {
        match *self {
            RustPointerLikeTypeKind::Reference { .. } => true,
            _ => false,
        }
    }
}

/// A Rust type
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RustType {
    /// Empty tuple `()`, used as the replacement of C++'s `void` type.
    EmptyTuple,
    /// A numeric, enum or struct type with some indirection
    Common {
        /// Full name of the base type
        path: RustPath,
        /// Generic arguments, if any
        generic_arguments: Option<Vec<RustType>>,
    },
    /// A function pointer type.
    FunctionPointer {
        /// Return type of the function.
        return_type: Box<RustType>,
        /// Argument types of the function.
        arguments: Vec<RustType>,
    },
    PointerLike {
        kind: RustPointerLikeTypeKind,
        is_const: bool,
        target: Box<RustType>,
    },
}

impl RustType {
    /// Returns alphanumeric description of this type
    /// for purposes of name disambiguation.
    #[allow(dead_code)]
    pub fn caption(&self, context: &RustPath) -> Result<String> {
        Ok(match *self {
            RustType::EmptyTuple => "empty".to_string(),
            RustType::PointerLike {
                ref kind,
                ref is_const,
                ref target,
            } => {
                let kind_text = match *kind {
                    RustPointerLikeTypeKind::Pointer => "ptr_",
                    RustPointerLikeTypeKind::Reference { .. } => "ref_",
                };
                let const_text = if *is_const { "" } else { "mut_" };
                format!("{}{}{}", kind_text, const_text, target.caption(context)?)
            }
            RustType::Common {
                ref path,
                ref generic_arguments,
            } => {
                let mut name = if path.parts.len() == 1 {
                    path.parts[0].to_snake_case()
                } else {
                    let mut remaining_context: &[String] = &context.parts;
                    let parts: &[String] = &path.parts;
                    let mut good_parts = Vec::new();
                    for part in parts {
                        if !remaining_context.is_empty() && part == &remaining_context[0] {
                            remaining_context = &remaining_context[1..];
                        } else {
                            remaining_context = &[];
                            let snake_part = part.to_snake_case();
                            if good_parts.last() != Some(&snake_part) {
                                good_parts.push(snake_part);
                            } else {
                            }
                        }
                    }
                    if good_parts.is_empty() {
                        path.last().to_string()
                    } else {
                        good_parts.join("_")
                    }
                };
                if let Some(ref args) = *generic_arguments {
                    name = format!(
                        "{}_{}",
                        name,
                        args.iter().map_if_ok(|x| x.caption(context))?.join("_")
                    );
                }
                name
            }
            RustType::FunctionPointer { .. } => "fn".to_string(),
        })
    }

    /// Returns true if this type is a reference.
    #[allow(dead_code)]
    pub fn is_ref(&self) -> bool {
        match *self {
            RustType::PointerLike { ref kind, .. } => kind.is_ref(),
            _ => false,
        }
    }

    /// Returns a copy of this type with `new_lifetime` added, if possible.
    pub fn with_lifetime(&self, new_lifetime: String) -> RustType {
        let mut r = self.clone();
        if let RustType::PointerLike { ref mut kind, .. } = r {
            match *kind {
                RustPointerLikeTypeKind::Pointer => {}
                RustPointerLikeTypeKind::Reference { ref mut lifetime } => {
                    *lifetime = Some(new_lifetime);
                }
            }
        }
        r
    }

    /// Returns name of the lifetime of this type,
    /// or `None` if there isn't any lifetime in this type.
    pub fn lifetime(&self) -> Option<&str> {
        if let RustType::PointerLike { ref kind, .. } = *self {
            if let RustPointerLikeTypeKind::Reference { ref lifetime } = *kind {
                return lifetime.as_ref().map(|s| s.as_str());
            }
        }
        None
    }
    /// Returns true if indirection that is applied last has const qualifier.
    pub fn last_is_const(&self) -> Result<bool> {
        if let RustType::PointerLike { ref is_const, .. } = *self {
            Ok(*is_const)
        } else {
            bail!("not a PointerLike type");
        }
    }

    /// Returns true if the first indirection of the type is const.
    pub fn is_const(&self) -> Result<bool> {
        match *self {
            RustType::PointerLike { ref is_const, .. } => Ok(*is_const),
            _ => bail!("not a PointerLike type"),
        }
    }

    /// Sets value of `is_const` for a `PointerLike` type.
    pub fn set_const(&mut self, value: bool) -> Result<()> {
        match *self {
            RustType::PointerLike {
                ref mut is_const, ..
            } => {
                *is_const = value;
                Ok(())
            }
            _ => bail!("not a PointerLike type"),
        }
    }

    /// Returns true if function with an argument of this type
    /// should be assumed unsafe. Currently returns true if this type
    /// is or contains a raw pointer.
    pub fn is_unsafe_argument(&self) -> bool {
        match *self {
            RustType::PointerLike {
                ref kind,
                ref target,
                ..
            } => kind.is_pointer() || target.is_unsafe_argument(),
            RustType::Common {
                ref generic_arguments,
                ..
            } => {
                if let Some(ref args) = *generic_arguments {
                    if args.iter().any(|arg| arg.is_unsafe_argument()) {
                        return true;
                    }
                }
                false
            }
            RustType::EmptyTuple => false,
            RustType::FunctionPointer {
                ref return_type,
                ref arguments,
            } => {
                return_type.is_unsafe_argument()
                    || arguments.iter().any(|arg| arg.is_unsafe_argument())
            }
        }
    }
}

impl CompleteType {
    /// Converts Rust API type from pointer to reference
    /// and modifies `rust_api_to_c_conversion` accordingly.
    /// `is_const1` specifies new constness of the created reference.
    pub fn ptr_to_ref(&self, is_const1: bool) -> Result<CompleteType> {
        let mut r = self.clone();
        if let RustType::PointerLike {
            ref mut is_const,
            ref mut kind,
            ..
        } = r.rust_api_type
        {
            if !kind.is_pointer() {
                bail!("not a pointer type");
            }
            *kind = RustPointerLikeTypeKind::Reference { lifetime: None };
            *is_const = is_const1;
        } else {
            bail!("not a PointerLike type");
        }
        if r.rust_api_to_ffi_conversion != RustToFfiTypeConversion::None {
            bail!("rust_api_to_ffi_conversion is not None");
        }
        r.rust_api_to_ffi_conversion = RustToFfiTypeConversion::RefToPtr;
        Ok(r)
    }

    /// Converts Rust API type from pointer to value
    /// and modifies `rust_api_to_c_conversion` accordingly.
    pub fn ptr_to_value(&self) -> Result<CompleteType> {
        let mut r = self.clone();
        r.rust_api_type = if let RustType::PointerLike {
            ref kind,
            ref target,
            ..
        } = r.rust_api_type
        {
            if !kind.is_pointer() {
                bail!("not a pointer type");
            }
            (**target).clone()
        } else {
            bail!("not a RustType::Common");
        };
        if r.rust_api_to_ffi_conversion != RustToFfiTypeConversion::None {
            bail!("rust_api_to_ffi_conversion is not None");
        }
        r.rust_api_to_ffi_conversion = RustToFfiTypeConversion::ValueToPtr;
        Ok(r)
    }
}
