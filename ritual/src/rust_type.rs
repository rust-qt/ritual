use crate::rust_info::RustTypeCaptionStrategy;
use itertools::Itertools;
use ritual_common::errors::{bail, Error, Result};
use ritual_common::string_utils::CaseOperations;
use ritual_common::utils::MapIfOk;
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;

/// Rust identifier. Represented by
/// a vector of name parts. First part is name of the crate,
/// last part is own name of the entity,
/// and intermediate names are module names.
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustPath {
    /// Parts of the name
    pub parts: Vec<String>,
}

impl FromStr for RustPath {
    type Err = Error;

    fn from_str(str: &str) -> Result<Self> {
        let parts = str.split("::").map(String::from).collect_vec();
        if parts.is_empty() {
            bail!("RustPath can't be empty");
        }
        if parts.iter().any(String::is_empty) {
            bail!("RustPath item can't be empty");
        }
        Ok(RustPath { parts })
    }
}

impl PartialEq<&str> for RustPath {
    fn eq(&self, str: &&str) -> bool {
        let parts = str.split("::");
        if self.parts.len() != parts.clone().count() {
            return false;
        }
        self.parts.iter().zip(parts).all(|(a, b)| a == b)
    }
}

impl RustPath {
    /// Creates new `RustPath` consisting of `parts`.
    pub fn from_parts(parts: Vec<String>) -> Self {
        if parts.is_empty() {
            panic!("RustPath can't be empty");
        }
        RustPath { parts }
    }

    pub fn from_good_str(str: &str) -> Self {
        Self::from_str(str).unwrap()
    }

    /// Returns crate name of this name, or `None`
    /// if this name does not contain the crate name (e.g. it's a built-in type).
    pub fn crate_name(&self) -> &str {
        if self.parts.is_empty() {
            panic!("RustPath can't be empty");
        }
        self.parts[0].as_str()
    }

    /// Returns last component of the name.
    pub fn last(&self) -> &str {
        self.parts.last().expect("RustPath can't be empty")
    }
    pub fn last_mut(&mut self) -> &mut String {
        self.parts.last_mut().expect("RustPath can't be empty")
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
            if self.crate_name() == current_crate {
                return format!("crate::{}", self.parts[1..].join("::"));
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

    pub fn parent(&self) -> Result<RustPath> {
        if self.parts.len() > 1 {
            let mut new_path = self.clone();
            new_path.parts.pop().unwrap();
            Ok(new_path)
        } else {
            bail!("failed to get parent path for {:?}", self)
        }
    }

    pub fn parent_parts(&self) -> Result<&[String]> {
        if self.parts.len() > 1 {
            Ok(&self.parts[..self.parts.len() - 1])
        } else {
            bail!("failed to get parent path for {:?}", self)
        }
    }

    pub fn parts(&self) -> &[String] {
        &self.parts
    }
}

/// Conversion from public Rust API type to
/// the corresponding FFI type
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum RustToFfiTypeConversion {
    /// Types are the same
    None,
    /// `&T` to `*const T` (or similar mutable types)
    RefToPtr {
        force_api_is_const: Option<bool>,
        lifetime: Option<String>,
    },
    /// `Ptr<T>` to `*const T` (or similar mutable type)
    UtilsPtrToPtr {
        force_api_is_const: Option<bool>,
    },
    /// `Ref<T>` to `*const T` (or similar mutable types)
    UtilsRefToPtr {
        force_api_is_const: Option<bool>,
    },
    /// `Option<Ref<T>>` to `*const T` (or similar mutable types)
    OptionUtilsRefToPtr {
        force_api_is_const: Option<bool>,
    },
    /// `QPtr<T>` to `Ptr<T>`
    QPtrToPtr,
    /// `T` to `*const T` (or similar mutable type)
    ValueToPtr,
    /// `CppBox<T>` to `*mut T`
    CppBoxToPtr,
    /// `QBox<T>` to `*mut T`
    QBoxToPtr,
    /// `qt_core::flags::Flags<T>` to `c_int`
    QFlagsToUInt {
        api_type: RustType,
    },
    /// `()` to any type
    UnitToAnything,
    /// Primitive to another primitive using `as`
    AsCast {
        api_type: RustType,
    },
    /// Rust public type has an additional reference (`&`)
    RefTo(Box<RustToFfiTypeConversion>),
    ImplCastInto(Box<RustToFfiTypeConversion>),
    ClosureToCallback(Box<RustClosureToCallbackConversion>),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RustClosureToCallbackConversion {
    pub closure_arguments: Vec<RustFinalType>,
    pub closure_return_type: RustFinalType,
}

impl RustToFfiTypeConversion {
    pub fn is_option_utils_ref_to_ptr(&self) -> bool {
        if let RustToFfiTypeConversion::OptionUtilsRefToPtr { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_utils_ref_to_ptr(&self) -> bool {
        if let RustToFfiTypeConversion::UtilsRefToPtr { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn as_callback_ref(&self) -> Option<&RustClosureToCallbackConversion> {
        if let RustToFfiTypeConversion::ClosureToCallback(x) = self {
            Some(x)
        } else {
            None
        }
    }
}

/// Information about a completely processed type
/// including its variations at each processing step.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RustFinalType {
    /// Rust type used in FFI functions
    /// (must be exactly the same as `cpp_ffi_type`)
    ffi_type: RustType,
    /// Type used in public Rust API
    api_type: RustType,
    /// Conversion from `rust_api_type` to `rust_ffi_type`
    conversion: RustToFfiTypeConversion,
}

fn utils_ptr(ffi_type: &RustType, force_api_is_const: Option<bool>) -> Result<RustType> {
    let is_const = if let Some(v) = force_api_is_const {
        v
    } else {
        ffi_type.is_const_pointer_like()?
    };
    let name = if is_const {
        "cpp_core::Ptr"
    } else {
        "cpp_core::MutPtr"
    };

    let target = ffi_type.pointer_like_to_target()?.clone();
    Ok(RustType::Common(RustCommonType {
        path: RustPath::from_good_str(name),
        generic_arguments: Some(vec![target]),
    }))
}

fn utils_ref(ffi_type: &RustType, force_api_is_const: Option<bool>) -> Result<RustType> {
    let is_const = if let Some(v) = force_api_is_const {
        v
    } else {
        ffi_type.is_const_pointer_like()?
    };
    let name = if is_const {
        "cpp_core::Ref"
    } else {
        "cpp_core::MutRef"
    };

    let target = ffi_type.pointer_like_to_target()?.clone();
    Ok(RustType::Common(RustCommonType {
        path: RustPath::from_good_str(name),
        generic_arguments: Some(vec![target]),
    }))
}

fn class_type_to_qt_core_crate_path(t: &RustType) -> Result<RustPath> {
    if let RustType::Common(t) = t {
        let name = if t.path.crate_name().starts_with("moqt") {
            "moqt_core"
        } else {
            "qt_core"
        };
        Ok(RustPath::from_good_str(name))
    } else {
        bail!("expected common type for QBox/QPtr conversion, got {:?}", t);
    }
}

impl RustFinalType {
    pub fn new(ffi_type: RustType, api_to_ffi_conversion: RustToFfiTypeConversion) -> Result<Self> {
        let api_type = match &api_to_ffi_conversion {
            RustToFfiTypeConversion::None => ffi_type.clone(),
            RustToFfiTypeConversion::RefToPtr {
                force_api_is_const,
                lifetime,
            } => {
                if let RustType::PointerLike {
                    is_const, target, ..
                } = &ffi_type
                {
                    let is_const = force_api_is_const.unwrap_or(*is_const);
                    RustType::PointerLike {
                        is_const,
                        kind: RustPointerLikeTypeKind::Reference {
                            lifetime: lifetime.clone(),
                        },
                        target: target.clone(),
                    }
                } else {
                    bail!("not a pointer like type");
                }
            }
            RustToFfiTypeConversion::UtilsPtrToPtr { force_api_is_const } => {
                utils_ptr(&ffi_type, *force_api_is_const)?
            }
            RustToFfiTypeConversion::UtilsRefToPtr { force_api_is_const } => {
                utils_ref(&ffi_type, *force_api_is_const)?
            }
            RustToFfiTypeConversion::OptionUtilsRefToPtr { force_api_is_const } => {
                RustType::new_option(utils_ref(&ffi_type, *force_api_is_const)?)
            }
            RustToFfiTypeConversion::ValueToPtr => ffi_type.pointer_like_to_target()?,
            RustToFfiTypeConversion::CppBoxToPtr => {
                let target = ffi_type.pointer_like_to_target()?;
                RustType::Common(RustCommonType {
                    path: RustPath::from_good_str("cpp_core::CppBox"),
                    generic_arguments: Some(vec![target.clone()]),
                })
            }
            RustToFfiTypeConversion::QBoxToPtr => {
                let target = ffi_type.pointer_like_to_target()?;
                RustType::Common(RustCommonType {
                    path: class_type_to_qt_core_crate_path(&target)?.join("QBox"),
                    generic_arguments: Some(vec![target.clone()]),
                })
            }
            RustToFfiTypeConversion::QPtrToPtr => {
                let target = ffi_type.pointer_like_to_target()?;
                RustType::Common(RustCommonType {
                    path: class_type_to_qt_core_crate_path(&target)?.join("QPtr"),
                    generic_arguments: Some(vec![target.clone()]),
                })
            }
            RustToFfiTypeConversion::QFlagsToUInt { api_type }
            | RustToFfiTypeConversion::AsCast { api_type } => api_type.clone(),
            RustToFfiTypeConversion::UnitToAnything => RustType::unit(),
            RustToFfiTypeConversion::RefTo(conversion) => {
                let intermediate = RustFinalType::new(ffi_type.clone(), (**conversion).clone())?;
                RustType::new_reference(true, intermediate.api_type)
            }
            RustToFfiTypeConversion::ImplCastInto(conversion) => {
                let intermediate = RustFinalType::new(ffi_type.clone(), (**conversion).clone())?;
                let trait_type = RustCommonType {
                    path: RustPath::from_good_str("cpp_core::CastInto"),
                    generic_arguments: Some(vec![intermediate.api_type]),
                };
                RustType::ImplTrait(trait_type)
            }
            RustToFfiTypeConversion::ClosureToCallback { .. } => {
                RustType::GenericParameter("T".into())
            }
        };
        Ok(RustFinalType {
            api_type,
            ffi_type,
            conversion: api_to_ffi_conversion,
        })
    }

    pub fn api_type(&self) -> &RustType {
        &self.api_type
    }

    pub fn ffi_type(&self) -> &RustType {
        &self.ffi_type
    }

    pub fn conversion(&self) -> &RustToFfiTypeConversion {
        &self.conversion
    }

    pub fn with_lifetime(&self, lifetime: String) -> Result<Self> {
        if let RustToFfiTypeConversion::RefToPtr {
            force_api_is_const, ..
        } = &self.conversion
        {
            RustFinalType::new(
                self.ffi_type.clone(),
                RustToFfiTypeConversion::RefToPtr {
                    force_api_is_const: *force_api_is_const,
                    lifetime: Some(lifetime),
                },
            )
        } else {
            bail!("not a RefToPtr type");
        }
    }
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

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustCommonType {
    /// Full name of the base type
    pub path: RustPath,
    /// Generic arguments, if any
    pub generic_arguments: Option<Vec<RustType>>,
}

pub fn paths_can_be_same<T1, T2>(one: &T1, other: &T2) -> bool
where
    T1: for<'a> PartialEq<&'a str> + PartialEq<T2>,
    T2: for<'a> PartialEq<&'a str>,
{
    let colliding: &[(&[&str], &[&str])] = &[
        (
            &[
                "std::os::raw::c_char",
                "std::os::raw::c_schar",
                "std::os::raw::c_short",
                "std::os::raw::c_int",
                "std::os::raw::c_long",
                "std::os::raw::c_longlong",
            ],
            &["u8", "u16", "u32", "u64"],
        ),
        (
            &[
                "std::os::raw::c_char",
                "std::os::raw::c_uchar",
                "std::os::raw::c_ushort",
                "std::os::raw::c_uint",
                "std::os::raw::c_ulong",
                "std::os::raw::c_ulonglong",
            ],
            &["u8", "u16", "u32", "u64"],
        ),
        (
            &["std::os::raw::c_float", "std::os::raw::c_double"],
            &["f32", "f64"],
        ),
    ];

    if one == other {
        return true;
    }

    for (ambiguous, concrete) in colliding {
        if ambiguous.iter().any(|s| one == s) && ambiguous.iter().any(|s| other == s) {
            return true;
        }
        if ambiguous.iter().any(|s| one == s) && concrete.iter().any(|s| other == s) {
            return true;
        }
        if concrete.iter().any(|s| one == s) && ambiguous.iter().any(|s| other == s) {
            return true;
        }
    }

    false
}

impl RustCommonType {
    pub fn can_be_same_as(&self, other: &RustCommonType) -> bool {
        let self_args = self
            .generic_arguments
            .as_ref()
            .into_iter()
            .flat_map(|vec| vec.iter());
        let other_args = other
            .generic_arguments
            .as_ref()
            .into_iter()
            .flat_map(|vec| vec.iter());
        if self_args.clone().count() != other_args.clone().count() {
            return false;
        }
        if !self_args.zip(other_args).all(|(a, b)| a.can_be_same_as(b)) {
            return false;
        }

        paths_can_be_same(&self.path, &other.path)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustFunctionPointerType {
    /// Return type of the function.
    pub return_type: Box<RustType>,
    /// Argument types of the function.
    pub arguments: Vec<RustType>,
}

/// A Rust type
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RustType {
    Tuple(Vec<RustType>),
    /// Primitive built-in type (`i32`, `str`, etc.)
    Primitive(String),
    /// A type specified by path with possible generic arguments
    Common(RustCommonType),
    /// A function pointer type.
    FunctionPointer(RustFunctionPointerType),
    PointerLike {
        kind: RustPointerLikeTypeKind,
        is_const: bool,
        target: Box<RustType>,
    },
    ImplTrait(RustCommonType),
    GenericParameter(String),
}

impl RustType {
    /// Constructs the unit type `()`, used as the replacement of C++'s `void` type.
    pub fn unit() -> Self {
        RustType::Tuple(Vec::new())
    }

    pub fn bool() -> Self {
        RustType::Primitive("bool".into())
    }

    pub fn new_pointer(is_const: bool, target: RustType) -> Self {
        RustType::PointerLike {
            kind: RustPointerLikeTypeKind::Pointer,
            is_const,
            target: Box::new(target),
        }
    }

    pub fn new_reference(is_const: bool, target: RustType) -> Self {
        RustType::PointerLike {
            kind: RustPointerLikeTypeKind::Reference { lifetime: None },
            is_const,
            target: Box::new(target),
        }
    }

    pub fn new_option(target: RustType) -> Self {
        RustType::Common(RustCommonType {
            path: RustPath::from_good_str("std::option::Option"),
            generic_arguments: Some(vec![target]),
        })
    }

    pub fn is_unit(&self) -> bool {
        if let RustType::Tuple(types) = self {
            types.is_empty()
        } else {
            false
        }
    }

    /// Returns alphanumeric description of this type
    /// for purposes of name disambiguation.
    pub fn caption(&self, context: &RustPath, strategy: RustTypeCaptionStrategy) -> Result<String> {
        Ok(match self {
            RustType::Tuple(types) => types
                .iter()
                .map_if_ok(|t| t.caption(context, strategy))?
                .join("_"),
            RustType::Primitive(type1) | RustType::GenericParameter(type1) => type1.to_string(),
            RustType::PointerLike {
                kind,
                is_const,
                target,
            } => {
                let const_text = if *is_const { "_const" } else { "" };
                let kind_text = match *kind {
                    RustPointerLikeTypeKind::Pointer => "_ptr",
                    RustPointerLikeTypeKind::Reference { .. } => "_ref",
                };
                format!(
                    "{}{}{}",
                    target.caption(context, strategy)?,
                    const_text,
                    kind_text
                )
            }
            RustType::Common(RustCommonType {
                path,
                generic_arguments,
            }) => {
                if path == &RustPath::from_good_str("cpp_core::MutPtr")
                    || path == &RustPath::from_good_str("cpp_core::Ptr")
                    || path == &RustPath::from_good_str("cpp_core::Ref")
                    || path == &RustPath::from_good_str("cpp_core::MutRef")
                    || path == &RustPath::from_good_str("cpp_core::CppBox")
                {
                    let arg = &generic_arguments.as_ref().unwrap()[0];
                    return arg.caption(context, strategy);
                }

                let mut name = if path.parts.len() == 1 {
                    path.parts[0].to_snake_case()
                } else if path.crate_name() == "std" {
                    let last = path.last();
                    let last = if last.starts_with("c_") {
                        &last[2..]
                    } else {
                        last
                    };
                    last.to_snake_case()
                } else if strategy == RustTypeCaptionStrategy::LastName {
                    path.last().to_string().to_snake_case()
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
                            }
                        }
                    }
                    if good_parts.is_empty() {
                        path.last().to_string().to_snake_case()
                    } else {
                        good_parts.join("_")
                    }
                };
                if let Some(args) = generic_arguments {
                    name = format!(
                        "{}_{}",
                        name,
                        args.iter()
                            .map_if_ok(|x| x.caption(context, strategy))?
                            .join("_")
                    );
                }
                name
            }
            RustType::FunctionPointer { .. } => "fn".to_string(),
            RustType::ImplTrait(trait_type) => {
                if trait_type.path == RustPath::from_good_str("cpp_core::CastInto") {
                    trait_type
                        .generic_arguments
                        .iter()
                        .flatten()
                        .map_if_ok(|x| x.caption(context, strategy))?
                        .join("_")
                } else {
                    RustType::Common(trait_type.clone()).caption(context, strategy)?
                }
            }
        })
    }

    /// Returns true if this type is a reference.
    pub fn is_ref(&self) -> bool {
        match self {
            RustType::PointerLike { kind, .. } => kind.is_ref(),
            _ => false,
        }
    }

    /// Returns a copy of this type with `new_lifetime` added, if possible.
    pub fn with_lifetime(&self, new_lifetime: String) -> RustType {
        let mut r = self.clone();
        if let RustType::PointerLike { kind, .. } = &mut r {
            match kind {
                RustPointerLikeTypeKind::Pointer => {}
                RustPointerLikeTypeKind::Reference { lifetime } => {
                    *lifetime = Some(new_lifetime);
                }
            }
        }
        r
    }

    /// Returns name of the lifetime of this type,
    /// or `None` if there isn't any lifetime in this type.
    pub fn lifetime(&self) -> Option<&str> {
        if let RustType::PointerLike { kind, .. } = self {
            if let RustPointerLikeTypeKind::Reference { lifetime } = kind {
                return lifetime.as_ref().map(String::as_str);
            }
        }
        None
    }
    /// Returns true if indirection that is applied last has const qualifier.
    pub fn is_const_pointer_like(&self) -> Result<bool> {
        if let RustType::PointerLike { is_const, .. } = self {
            Ok(*is_const)
        } else {
            bail!("not a PointerLike type");
        }
    }

    /// Sets value of `is_const` for a `PointerLike` type.
    pub fn set_const(&mut self, value: bool) -> Result<()> {
        match self {
            RustType::PointerLike { is_const, .. } => {
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
        match self {
            RustType::PointerLike { kind, target, .. } => {
                kind.is_pointer() || target.is_unsafe_argument()
            }
            RustType::Primitive(_) | RustType::GenericParameter(_) => false,
            RustType::Common(RustCommonType {
                generic_arguments, ..
            }) => {
                if let Some(args) = generic_arguments {
                    if args.iter().any(RustType::is_unsafe_argument) {
                        return true;
                    }
                }
                false
            }
            RustType::Tuple(types) => types.iter().any(RustType::is_unsafe_argument),
            RustType::FunctionPointer(function) => {
                function.return_type.is_unsafe_argument()
                    || function.arguments.iter().any(RustType::is_unsafe_argument)
            }
            RustType::ImplTrait(_) => true,
        }
    }

    pub fn pointer_like_to_target(&self) -> Result<RustType> {
        if let RustType::PointerLike { target, .. } = self {
            Ok((**target).clone())
        } else {
            bail!("not a pointer like type");
        }
    }

    pub fn ptr_to_ref(&self, is_const1: bool) -> Result<Self> {
        let mut r = self.clone();
        if let RustType::PointerLike { is_const, kind, .. } = &mut r {
            if !kind.is_pointer() {
                bail!("not a pointer type");
            }
            *kind = RustPointerLikeTypeKind::Reference { lifetime: None };
            *is_const = is_const1;
        } else {
            bail!("not a PointerLike type");
        }
        Ok(r)
    }

    pub fn as_common(&self) -> Result<&RustCommonType> {
        if let RustType::Common(r) = self {
            Ok(r)
        } else {
            bail!("expected common type, got {:?}", self)
        }
    }

    pub fn can_be_same_as(&self, other: &RustType) -> bool {
        match self {
            RustType::Tuple(self_args) => {
                if let RustType::Tuple(other_args) = other {
                    self_args.len() == other_args.len()
                        && self_args
                            .iter()
                            .zip(other_args)
                            .all(|(a, b)| a.can_be_same_as(b))
                } else {
                    false
                }
            }
            RustType::Common(self_type) => match other {
                RustType::Common(other_type) => self_type.can_be_same_as(other_type),
                RustType::Primitive(other_type) => {
                    self_type.generic_arguments.is_none()
                        && paths_can_be_same(&self_type.path, &other_type.as_str())
                }
                _ => false,
            },
            RustType::Primitive(type1) => match other {
                RustType::Primitive(other_type) => type1 == other_type,
                RustType::Common(other_type) => {
                    other_type.generic_arguments.is_none()
                        && paths_can_be_same(&other_type.path, &type1.as_str())
                }
                _ => false,
            },
            RustType::FunctionPointer(function) => {
                if let RustType::FunctionPointer(other) = other {
                    function.return_type.can_be_same_as(&other.return_type)
                        && function.arguments.len() == other.arguments.len()
                        && function
                            .arguments
                            .iter()
                            .zip(&other.arguments)
                            .all(|(a, b)| a.can_be_same_as(b))
                } else {
                    false
                }
            }
            RustType::PointerLike {
                kind: self_kind,
                is_const: self_is_const,
                target: self_target,
            } => {
                if let RustType::PointerLike {
                    kind,
                    is_const,
                    target,
                } = other
                {
                    self_kind == kind
                        && self_is_const == is_const
                        && self_target.can_be_same_as(target)
                } else {
                    false
                }
            }
            RustType::ImplTrait(trait_type) => {
                if let RustType::ImplTrait(other_trait_type) = other {
                    trait_type.can_be_same_as(other_trait_type)
                } else {
                    false
                }
            }
            RustType::GenericParameter(_) => self == other,
        }
    }
}
