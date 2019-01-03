//use common::errors::{unexpected, ResultExt, Result};
//use common::string_utils::CaseOperations;
//use common::utils::MapIfOk;
//use cpp_ffi_data::CppIndirectionChange;
//use cpp_type::CppType;
use serde_derive::{Deserialize, Serialize};

/// Rust identifier. Represented by
/// a vector of name parts. For a regular name,
/// first part is name of the crate,
/// last part is own name of the entity,
/// and intermediate names are module names.
/// Built-in types are represented
/// by a single vector item, like `vec!["i32"]`.
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustName {
    /// Parts of the name
    pub parts: Vec<String>,
}

/*
/// Conversion from public Rust API type to
/// the corresponding FFI type
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum RustToCTypeConversion {
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
  pub cpp_type: CppType,
  /// C++ type used in the C++ wrapper library's API
  pub cpp_ffi_type: CppType,
  /// Conversion from `cpp_type` to `cpp_ffi_type`
  pub cpp_to_ffi_conversion: CppIndirectionChange,
  /// Rust type used in FFI functions
  /// (must be exactly the same as `cpp_ffi_type`)
  pub rust_ffi_type: RustType,
  /// Type used in public Rust API
  pub rust_api_type: RustType,
  /// Conversion from `rust_api_type` to `rust_ffi_type`
  pub rust_api_to_c_conversion: RustToCTypeConversion,
}

/// Indirection of a Rust type
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RustTypeIndirection {
  /// No indirection
  None,
  /// Raw pointer
  Ptr,
  /// Reference with a lifetime
  Ref { lifetime: Option<String> },
  /// Raw pointer to raw pointer
  PtrPtr,
  /// Raw pointer to reference
  PtrRef { lifetime: Option<String> },
}

/// A Rust type
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RustType {
  /// Empty tuple `()`, used as the replacement of C++'s `void` type.
  EmptyTuple,
  /// A numeric, enum or struct type with some indirection
  Common {
    /// Full name of the base type
    base: RustName,
    /// Generic arguments, if any
    generic_arguments: Option<Vec<RustType>>,
    /// If the type has no indirection, `is_const`
    /// indicates constness of the type itself (e.g. `i32` vs `mut i32`).
    /// If the type has one level of indirection, `is_const`
    /// indicates constness of that indirection, i.e. if the pointer or the reference
    /// is const. If the type has two levels of indirection,
    /// `is_const` indicates constness of indirection that is applied first.
    is_const: bool,
    /// If the type has two levels of indirection,
    /// `is_const2` indicates constness of indirection that is applied second.
    /// In other cases it is `false`.
    is_const2: bool,
    /// Indirection of this type.
    indirection: RustTypeIndirection,
  },
  /// A function pointer type.
  FunctionPointer {
    /// Return type of the function.
    return_type: Box<RustType>,
    /// Argument types of the function.
    arguments: Vec<RustType>,
  },
}

impl RustName {
  /// Creates new `RustName` consisting of `parts`.
  pub fn new(parts: Vec<String>) -> Result<RustName> {
    if parts.is_empty() {
      return Err(unexpected("RustName can't be empty").into());
    }
    Ok(RustName { parts: parts })
  }

  /// Returns crate name of this name, or `None`
  /// if this name does not contain the crate name.
  pub fn crate_name(&self) -> Option<&String> {
    assert!(self.parts.len() > 0);
    if self.parts.len() > 1 {
      Some(&self.parts[0])
    } else {
      None
    }
  }

  /// Returns last component of the name.
  pub fn last_name(&self) -> Result<&String> {
    self
      .parts
      .last()
      .with_context(|| unexpected("RustName can't be empty"))
  }

  /// Returns formatted name for using within `current_crate`.
  /// If `current_crate` is `None`, it's assumed that the formatted name
  /// will be used outside of the crate it belongs to.
  pub fn full_name(&self, current_crate: Option<&str>) -> String {
    if let Some(current_crate) = current_crate {
      if let Some(self_crate) = self.crate_name() {
        if self_crate == current_crate {
          return format!("::{}", self.parts[1..].join("::"));
        }
      }
    }
    if self.parts.len() == 1 {
      self.parts[0].clone()
    } else {
      format!("::{}", self.parts.join("::"))
    }
  }

  /// Returns true if `other` is nested within `self`.
  pub fn includes(&self, other: &RustName) -> bool {
    let extra_modules_count = other.parts.len() as isize - self.parts.len() as isize;
    extra_modules_count > 0 && other.parts[0..self.parts.len()] == self.parts[..]
  }

  /// Returns true if `other` is a direct child of `self`.
  pub fn includes_directly(&self, other: &RustName) -> bool {
    let extra_modules_count = other.parts.len() as isize - self.parts.len() as isize;
    self.includes(other) && extra_modules_count == 1
  }
}

impl RustType {
  /// Returns alphanumeric description of this type
  /// for purposes of name disambiguation.
  #[allow(dead_code)]
  pub fn caption(&self, context: &RustName) -> Result<String> {
    Ok(match *self {
      RustType::EmptyTuple => "empty".to_string(),
      RustType::Common {
        ref base,
        ref generic_arguments,
        ref is_const,
        ref is_const2,
        ref indirection,
      } => {
        let mut name = if base.parts.len() == 1 {
          base.parts[0].to_snake_case()
        } else {
          let mut remaining_context: &[String] = &context.parts;
          let mut parts: &[String] = &base.parts;
          if &parts[0] == "libc" {
            parts = &parts[1..];
          };
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
            base.last_name()?.clone()
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
        let mut_text = if *is_const { "" } else { "_mut" };
        match *indirection {
          RustTypeIndirection::None => {}
          RustTypeIndirection::Ref { .. } => {
            name = format!("{}{}_ref", name, mut_text);
          }
          RustTypeIndirection::Ptr => {
            name = format!("{}{}_ptr", name, mut_text);
          }
          RustTypeIndirection::PtrPtr => {
            let mut_text2 = if *is_const2 { "" } else { "_mut" };
            name = format!("{}{}_ptr{}_ptr", name, mut_text, mut_text2);
          }
          RustTypeIndirection::PtrRef { .. } => {
            let mut_text2 = if *is_const2 { "" } else { "_mut" };
            name = format!("{}{}_ptr{}_ref", name, mut_text, mut_text2);
          }
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
      RustType::Common {
        ref indirection, ..
      } => match *indirection {
        RustTypeIndirection::Ref { .. } | RustTypeIndirection::PtrRef { .. } => true,
        _ => false,
      },
      RustType::EmptyTuple | RustType::FunctionPointer { .. } => false,
    }
  }

  /// Returns a copy of this type with `new_lifetime` added, if possible.
  pub fn with_lifetime(&self, new_lifetime: String) -> RustType {
    let mut r = self.clone();
    if let RustType::Common {
      ref mut indirection,
      ..
    } = r
    {
      match *indirection {
        RustTypeIndirection::Ref { ref mut lifetime }
        | RustTypeIndirection::PtrRef { ref mut lifetime } => *lifetime = Some(new_lifetime),
        _ => {}
      }
    }
    r
  }

  /// Returns name of the lifetime of this type,
  /// or `None` if there isn't any lifetime in this type.
  pub fn lifetime(&self) -> Option<&String> {
    match *self {
      RustType::Common {
        ref indirection, ..
      } => match *indirection {
        RustTypeIndirection::Ref { ref lifetime }
        | RustTypeIndirection::PtrRef { ref lifetime } => lifetime.as_ref(),
        _ => None,
      },
      _ => None,
    }
  }
  /// Returns true if indirection that is applied last has const qualifier.
  pub fn last_is_const(&self) -> Result<bool> {
    if let RustType::Common {
      ref is_const,
      ref is_const2,
      ref indirection,
      ..
    } = *self
    {
      match *indirection {
        RustTypeIndirection::PtrPtr { .. } | RustTypeIndirection::PtrRef { .. } => Ok(*is_const2),
        _ => Ok(*is_const),
      }
    } else {
      Err("not a Common type".into())
    }
  }

  /// Returns true if this type (or first indirection of the type) is const.
  pub fn is_const(&self) -> Result<bool> {
    match *self {
      RustType::Common { ref is_const, .. } => Ok(*is_const),
      _ => Err("not a Common type".into()),
    }
  }

  /// Sets value of `is_const` for a common type.
  pub fn set_const(&mut self, value: bool) -> Result<()> {
    match *self {
      RustType::Common {
        ref mut is_const, ..
      } => {
        *is_const = value;
        Ok(())
      }
      _ => Err("not a Common type".into()),
    }
  }

  /// Returns true if function with an argument of type `self`
  /// should be assumed unsafe. Currently returns true if this type
  /// is or contains a raw pointer.
  pub fn is_unsafe_argument(&self) -> bool {
    match *self {
      RustType::Common {
        ref indirection,
        ref base,
        ref generic_arguments,
        ..
      } => {
        match *indirection {
          RustTypeIndirection::None | RustTypeIndirection::Ref { .. } => {}
          RustTypeIndirection::Ptr
          | RustTypeIndirection::PtrPtr
          | RustTypeIndirection::PtrRef { .. } => {
            return true;
          }
        }
        if base.full_name(None) == "std::option::Option" {
          if let Some(ref args) = *generic_arguments {
            if let Some(ref arg) = args.get(0) {
              if arg.is_unsafe_argument() {
                return true;
              }
            }
          }
        }
        false
      }
      RustType::EmptyTuple => false,
      RustType::FunctionPointer { .. } => true,
    }
  }
}

impl CompleteType {
  /// Converts Rust API type from pointer to reference
  /// and modifies `rust_api_to_c_conversion` accordingly.
  /// `is_const1` specifies new constness of the created reference.
  pub fn ptr_to_ref(&self, is_const1: bool) -> Result<CompleteType> {
    let mut r = self.clone();
    if let RustType::Common {
      ref mut is_const,
      ref mut indirection,
      ..
    } = r.rust_api_type
    {
      if *indirection != RustTypeIndirection::Ptr {
        return Err("not a pointer type".into());
      }
      *indirection = RustTypeIndirection::Ref { lifetime: None };
      *is_const = is_const1;
    } else {
      return Err("not a RustType::Common".into());
    }
    if r.rust_api_to_c_conversion != RustToCTypeConversion::None {
      return Err("rust_api_to_c_conversion is not none".into());
    }
    r.rust_api_to_c_conversion = RustToCTypeConversion::RefToPtr;
    Ok(r)
  }

  /// Converts Rust API type from pointer to value
  /// and modifies `rust_api_to_c_conversion` accordingly.
  pub fn ptr_to_value(&self) -> Result<CompleteType> {
    let mut r = self.clone();
    if let RustType::Common {
      ref mut is_const,
      ref mut indirection,
      ..
    } = r.rust_api_type
    {
      if *indirection != RustTypeIndirection::Ptr {
        return Err("not a pointer type".into());
      }
      *indirection = RustTypeIndirection::None;
      *is_const = true;
    } else {
      return Err("not a RustType::Common".into());
    }
    if r.rust_api_to_c_conversion != RustToCTypeConversion::None {
      return Err("rust_api_to_c_conversion is not none".into());
    }
    r.rust_api_to_c_conversion = RustToCTypeConversion::ValueToPtr;
    Ok(r)
  }
}
*/
