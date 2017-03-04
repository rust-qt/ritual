use caption_strategy::TypeCaptionStrategy;
use cpp_data::CppFunctionPointerType;
use cpp_ffi_data::{CppFfiType, IndirectionChange};
use common::errors::{Result, ChainErr, Error, unexpected};
use common::string_utils::JoinWithString;
use common::utils::MapIfOk;

extern crate regex;

pub use serializable::{CppBuiltInNumericType, CppSpecificNumericTypeKind, CppTypeBase, CppType,
                       CppTypeIndirection, CppTypeClassBase};

impl CppTypeIndirection {
  pub fn combine(left: &CppTypeIndirection,
                 right: &CppTypeIndirection)
                 -> Result<CppTypeIndirection> {
    let err = || format!("too much indirection: {:?} to {:?}", left, right).into();
    Ok(match *left {
      CppTypeIndirection::None => right.clone(),
      CppTypeIndirection::Ptr => {
        match *right {
          CppTypeIndirection::None => CppTypeIndirection::Ptr,
          CppTypeIndirection::Ptr => CppTypeIndirection::PtrPtr,
          CppTypeIndirection::Ref => CppTypeIndirection::PtrRef,
          _ => return Err(err()),
        }
      }
      CppTypeIndirection::Ref => {
        match *right {
          CppTypeIndirection::None => CppTypeIndirection::Ref,
          _ => return Err(err()),
        }
      }
      _ => {
        match *right {
          CppTypeIndirection::None => left.clone(),
          _ => return Err(err()),
        }
      }
    })
  }
}

impl CppBuiltInNumericType {
  pub fn to_cpp_code(&self) -> &'static str {
    use serializable::CppBuiltInNumericType::*;
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

  pub fn is_float(&self) -> bool {
    use serializable::CppBuiltInNumericType::*;
    match *self {
      Float | Double | LongDouble => true,
      _ => false,
    }
  }

  pub fn is_signed_integer(&self) -> bool {
    use serializable::CppBuiltInNumericType::*;
    match *self {
      SChar | Short | Int | Long | LongLong | Int128 => true,
      _ => false,
    }
  }

  pub fn is_unsigned_integer(&self) -> bool {
    use serializable::CppBuiltInNumericType::*;
    match *self {
      UChar | Char16 | Char32 | UShort | UInt | ULong | ULongLong | UInt128 => true,
      _ => false,
    }
  }

  pub fn is_integer_with_undefined_signedness(&self) -> bool {
    use serializable::CppBuiltInNumericType::*;
    match *self {
      Char | WChar => true,
      _ => false,
    }
  }


  pub fn all() -> [CppBuiltInNumericType; 20] {
    use serializable::CppBuiltInNumericType::*;
    [Bool, Char, SChar, UChar, WChar, Char16, Char32, Short, UShort, Int, UInt, Long, ULong,
     LongLong, ULongLong, Int128, UInt128, Float, Double, LongDouble]
  }
}


impl CppTypeClassBase {
  pub fn to_cpp_code(&self) -> Result<String> {
    match self.template_arguments {
      Some(ref args) => {
        let mut arg_texts = Vec::new();
        for arg in args {
          arg_texts.push(arg.to_cpp_code(None)?);
        }
        Ok(format!("{}< {} >", self.name, arg_texts.join(", ")))
      }
      None => Ok(self.name.clone()),
    }

  }
  pub fn caption(&self) -> Result<String> {
    let name_caption = self.name.replace("::", "_");
    Ok(match self.template_arguments {
      Some(ref args) => {
        format!("{}_{}",
                name_caption,
                args.iter().map_if_ok(|arg| arg.caption(TypeCaptionStrategy::Full))?.join("_"))
      }
      None => name_caption,
    })
  }

  pub fn instantiate_class(&self,
                           nested_level1: i32,
                           template_arguments1: &[CppType])
                           -> Result<CppTypeClassBase> {
    Ok(CppTypeClassBase {
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

  pub fn to_cpp_pseudo_code(&self) -> String {
    if let Some(ref template_arguments) = self.template_arguments {
      format!("{}<{}>",
              self.name,
              template_arguments.iter()
                .map(|x| x.to_cpp_pseudo_code())
                .join(", "))
    } else {
      self.name.clone()
    }
  }
}

impl CppTypeBase {
  #[allow(dead_code)]
  pub fn is_void(&self) -> bool {
    match *self {
      CppTypeBase::Void => true,
      _ => false,
    }
  }
  pub fn is_class(&self) -> bool {
    match *self {
      CppTypeBase::Class(..) => true,
      _ => false,
    }
  }
  pub fn is_template_parameter(&self) -> bool {
    match *self {
      CppTypeBase::TemplateParameter { .. } => true,
      _ => false,
    }
  }
  pub fn is_function_pointer(&self) -> bool {
    match *self {
      CppTypeBase::FunctionPointer(..) => true,
      _ => false,
    }
  }
  pub fn is_or_contains_template_parameter(&self) -> bool {
    match *self {
      CppTypeBase::TemplateParameter { .. } => true,
      CppTypeBase::Class(CppTypeClassBase { ref template_arguments, .. }) => {
        if let Some(ref template_arguments) = *template_arguments {
          template_arguments.iter()
            .any(|arg| arg.base.is_or_contains_template_parameter())
        } else {
          false
        }
      }
      _ => false,
    }
  }

  pub fn to_cpp_code(&self, function_pointer_inner_text: Option<&str>) -> Result<String> {
    if !self.is_function_pointer() && function_pointer_inner_text.is_some() {
      return Err("unexpected function_pointer_inner_text".into());
    }
    match *self {
      CppTypeBase::Void => Ok("void".to_string()),
      CppTypeBase::BuiltInNumeric(ref t) => Ok(t.to_cpp_code().to_string()),
      CppTypeBase::Enum { ref name } |
      CppTypeBase::SpecificNumeric { ref name, .. } |
      CppTypeBase::PointerSizedInteger { ref name, .. } => Ok(name.clone()),
      //      CppTypeBase::SpecificNumeric { ref name, .. } => Ok(name.clone()),
      //      CppTypeBase::PointerSizedInteger { ref name, .. } => Ok(name.clone()),
      CppTypeBase::Class(ref info) => info.to_cpp_code(),
      CppTypeBase::TemplateParameter { .. } => {
        Err("template parameters are not allowed in C++ code generator".into())
      }
      CppTypeBase::FunctionPointer(CppFunctionPointerType { ref return_type,
                                                            ref arguments,
                                                            ref allows_variadic_arguments }) => {
        if *allows_variadic_arguments {
          return Err("function pointers with variadic arguments are not supported".into());
        }
        let mut arg_texts = Vec::new();
        for arg in arguments {
          arg_texts.push(arg.to_cpp_code(None)?);
        }
        if let Some(function_pointer_inner_text) = function_pointer_inner_text {
          Ok(format!("{} (*{})({})",
                     return_type.as_ref().to_cpp_code(None)?,
                     function_pointer_inner_text,
                     arg_texts.join(", ")))
        } else {
          return Err("function_pointer_inner_text argument is missing".into());
        }
      }
    }
  }

  #[allow(dead_code)]
  pub fn maybe_name(&self) -> Option<&String> {
    match *self {
      CppTypeBase::SpecificNumeric { ref name, .. } |
      CppTypeBase::PointerSizedInteger { ref name, .. } |
      CppTypeBase::Enum { ref name } |
      CppTypeBase::Class(CppTypeClassBase { ref name, .. }) => Some(name),
      _ => None,
    }
  }

  /// Generates alphanumeric representation of self
  /// used to generate FFI function names
  pub fn caption(&self, strategy: TypeCaptionStrategy) -> Result<String> {
    Ok(match *self {
      CppTypeBase::Void => "void".to_string(),
      CppTypeBase::BuiltInNumeric(ref t) => t.to_cpp_code().to_string().replace(" ", "_"),
      CppTypeBase::SpecificNumeric { ref name, .. } |
      CppTypeBase::PointerSizedInteger { ref name, .. } => name.clone(),
      CppTypeBase::Enum { ref name } => name.replace("::", "_"),
      CppTypeBase::Class(ref data) => data.caption()?,
      CppTypeBase::TemplateParameter { .. } => {
        return Err("template parameters are not allowed to have captions".into());
      }
      CppTypeBase::FunctionPointer(CppFunctionPointerType { ref return_type,
                                                            ref arguments,
                                                            .. }) => {
        match strategy {
          TypeCaptionStrategy::Short => "func".to_string(),
          TypeCaptionStrategy::Full => {
            format!("{}_func_{}",
                    return_type.caption(strategy.clone())?,
                    arguments.iter().map_if_ok(|x| x.caption(strategy.clone()))?.join("_"))
          }
        }
      }

    })
  }

  pub fn to_cpp_pseudo_code(&self) -> String {
    match *self {
      CppTypeBase::TemplateParameter { ref nested_level, ref index } => {
        return format!("T_{}_{}", nested_level, index);
      }
      CppTypeBase::Class(ref base) => return base.to_cpp_pseudo_code(),
      CppTypeBase::FunctionPointer(..) => {
        return self.to_cpp_code(Some(&"FN_PTR".to_string())).unwrap_or_else(|_| "[?]".to_string())
      }
      _ => {}
    };
    self.to_cpp_code(None).unwrap_or_else(|_| "[?]".to_string())
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CppTypeRole {
  ReturnType,
  NotReturnType,
}


impl CppType {
  pub fn void() -> Self {
    CppType {
      is_const: false,
      is_const2: false,
      indirection: CppTypeIndirection::None,
      base: CppTypeBase::Void,
    }
  }

  pub fn is_void(&self) -> bool {
    !self.is_const && self.indirection == CppTypeIndirection::None && self.base == CppTypeBase::Void
  }

  fn to_cpp_code_intermediate(&self, base_code: &str) -> String {
    format!("{}{}{}",
            if self.is_const { "const " } else { "" },
            base_code,
            match self.indirection {
              CppTypeIndirection::None => "",
              CppTypeIndirection::Ptr => "*",
              CppTypeIndirection::Ref => "&",
              CppTypeIndirection::PtrRef => if self.is_const2 { "* const &" } else { "*&" },
              CppTypeIndirection::PtrPtr => if self.is_const2 { "* const *" } else { "**" },
              CppTypeIndirection::RValueRef => "&&",
            })
  }

  pub fn to_cpp_code(&self, function_pointer_inner_text: Option<&str>) -> Result<String> {
    let base_code = self.base.to_cpp_code(function_pointer_inner_text)?;
    Ok(self.to_cpp_code_intermediate(&base_code))
  }

  pub fn to_cpp_pseudo_code(&self) -> String {
    let base_code = self.base.to_cpp_pseudo_code();
    self.to_cpp_code_intermediate(&base_code)
  }

  /// Converts this C++ type to its adaptation for FFI interface,
  /// removing all features not supported by C ABI
  /// (e.g. references and passing objects by value)
  #[cfg_attr(feature="clippy", allow(collapsible_if))]
  pub fn to_cpp_ffi_type(&self, role: CppTypeRole) -> Result<CppFfiType> {
    let err = || format!("Can't express type to FFI: {:?}", self);
    match self.base {
      CppTypeBase::TemplateParameter { .. } => {
        return Err(Error::from("template parameters cannot be expressed in FFI")).chain_err(&err);
      }
      CppTypeBase::FunctionPointer(CppFunctionPointerType { ref return_type,
                                                            ref arguments,
                                                            ref allows_variadic_arguments }) => {
        if *allows_variadic_arguments {
          return Err(Error::from("function pointers with variadic arguments are not supported"))
            .chain_err(&err);
        }
        let mut all_types: Vec<&CppType> = arguments.iter().collect();
        all_types.push(return_type.as_ref());
        for arg in all_types {
          match arg.base {
            CppTypeBase::TemplateParameter { .. } => {
              return Err(Error::from("function pointers containing template parameters are not \
                                      supported"))
                .chain_err(&err);
            }
            CppTypeBase::FunctionPointer(..) => {
              return Err(Error::from("function pointers containing nested function pointers are \
                                      not supported"))
                .chain_err(&err);
            }
            _ => {}
          }
          match arg.indirection {
            CppTypeIndirection::Ref |
            CppTypeIndirection::PtrRef |
            CppTypeIndirection::RValueRef => {
              return Err(Error::from("Function pointers containing references are not supported"))
                .chain_err(&err);
            }
            CppTypeIndirection::Ptr |
            CppTypeIndirection::PtrPtr => {}
            CppTypeIndirection::None => {
              if arg.base.is_class() {
                return Err(Error::from("Function pointers containing classes by value are not \
                                        supported"))
                  .chain_err(&err);
              }
            }
          }
        }
        return Ok(CppFfiType {
          ffi_type: self.clone(),
          conversion: IndirectionChange::NoChange,
          original_type: self.clone(),
        });
      }
      _ => {}
    }
    let mut result = self.clone();
    let mut conversion = IndirectionChange::NoChange;
    match self.indirection {
      CppTypeIndirection::None |
      CppTypeIndirection::Ptr |
      CppTypeIndirection::PtrPtr => {
        // no change needed
      }
      CppTypeIndirection::Ref => {
        result.indirection = CppTypeIndirection::Ptr;
        conversion = IndirectionChange::ReferenceToPointer;
      }
      CppTypeIndirection::PtrRef => {
        result.indirection = CppTypeIndirection::PtrPtr;
        conversion = IndirectionChange::ReferenceToPointer;
      }
      CppTypeIndirection::RValueRef => {
        return Err(Error::from("rvalue references are not supported")).chain_err(&err);
      }
    }
    if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = self.base {
      if name == "QFlags" {
        if !(self.indirection == CppTypeIndirection::None ||
             (self.indirection == CppTypeIndirection::Ref && self.is_const)) {
          return Err(Error::from(format!("QFlags type can only be values or const references: \
                                          {:?}",
                                         self)))
            .chain_err(&err);
        }
        conversion = IndirectionChange::QFlagsToUInt;
        result.base = CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::UInt);
        result.is_const = false;
        result.indirection = CppTypeIndirection::None;
      } else {
        // structs can't be passed by value
        if self.indirection == CppTypeIndirection::None {
          result.indirection = CppTypeIndirection::Ptr;
          conversion = IndirectionChange::ValueToPointer;

          // "const Rect" return type should not be translated to const pointer
          result.is_const = role != CppTypeRole::ReturnType;
        }
      }
    }
    Ok(CppFfiType {
      ffi_type: result,
      conversion: conversion,
      original_type: self.clone(),
    })
  }

  /// Generates alphanumeric representation of self
  /// used to generate FFI function names
  pub fn caption(&self, strategy: TypeCaptionStrategy) -> Result<String> {
    Ok(match strategy {
      TypeCaptionStrategy::Short => self.base.caption(strategy.clone())?,
      TypeCaptionStrategy::Full => {
        let mut r = self.base.caption(strategy.clone())?;
        match self.indirection {
          CppTypeIndirection::None => {}
          CppTypeIndirection::Ptr => r = format!("{}_ptr", r),
          CppTypeIndirection::Ref => r = format!("{}_ref", r),
          CppTypeIndirection::PtrRef => r = format!("{}_ptr_ref", r),
          CppTypeIndirection::PtrPtr => {
            if self.is_const2 {
              r = format!("{}_ptr_const_ptr", r);
            } else {
              r = format!("{}_ptr_ptr", r);
            }
          }
          CppTypeIndirection::RValueRef => r = format!("{}_rvalue_ref", r),
        }
        if self.is_const {
          r = format!("const_{}", r);
        }
        r
      }
    })
  }

  /// Checks if a function with this return type would need
  /// to have 2 wrappers with 2 different return value allocation places
  pub fn needs_allocation_place_variants(&self) -> bool {
    if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = self.base {
      if name == "QFlags" {
        return false; // converted to uint in FFI
      }
    }
    self.indirection == CppTypeIndirection::None && self.base.is_class()
  }

  #[cfg_attr(feature="clippy", allow(if_not_else))]
  pub fn instantiate(&self,
                     nested_level1: i32,
                     template_arguments1: &[CppType])
                     -> Result<CppType> {
    if let CppTypeBase::TemplateParameter { nested_level, index } = self.base {
      if nested_level == nested_level1 {
        if index < 0 {
          return Err(unexpected("CppType::instantiate: index < 0").into());
        }
        if index >= template_arguments1.len() as i32 {
          return Err("not enough template arguments".into());
        }
        let arg = &template_arguments1[index as usize];
        let mut new_type = CppType::void();
        new_type.base = arg.base.clone();
        match CppTypeIndirection::combine(&arg.indirection, &self.indirection) {
          Err(msg) => return Err(msg),
          Ok(r) => new_type.indirection = r,
        }
        match new_type.indirection {
          CppTypeIndirection::None => {
            new_type.is_const = self.is_const || arg.is_const;
          }
          CppTypeIndirection::Ptr |
          CppTypeIndirection::Ref |
          CppTypeIndirection::RValueRef => {
            if self.indirection != CppTypeIndirection::None {
              new_type.is_const = self.is_const;
            } else if arg.indirection != CppTypeIndirection::None {
              new_type.is_const = arg.is_const;
            } else {
              return Err(unexpected("CppType::instantiate: one of types must be ptr or ref!")
                .into());
            }
          }
          CppTypeIndirection::PtrPtr |
          CppTypeIndirection::PtrRef => {
            if self.indirection == new_type.indirection {
              new_type.is_const = self.is_const;
              new_type.is_const2 = self.is_const2;
            } else if arg.indirection == new_type.indirection {
              new_type.is_const = arg.is_const;
              new_type.is_const2 = arg.is_const2;
            } else {
              new_type.is_const = arg.is_const;
              new_type.is_const2 = self.is_const;
            }
          }
        }
        return Ok(new_type);
      }
    }
    Ok(CppType {
      is_const: self.is_const,
      is_const2: self.is_const2,
      indirection: self.indirection.clone(),
      base: match self.base {
        CppTypeBase::Class(ref data) => {
          CppTypeBase::Class(data.instantiate_class(nested_level1, template_arguments1)?)
        }
        _ => self.base.clone(),
      },
    })

  }

  pub fn is_platform_dependent(&self) -> bool {
    if let CppTypeBase::Class(CppTypeClassBase { ref template_arguments, .. }) = self.base {
      if let Some(ref template_arguments) = *template_arguments {
        for arg in template_arguments {
          if arg.is_platform_dependent() {
            return true;
          }
        }
      }
    }
    if let CppTypeBase::BuiltInNumeric(ref data) = self.base {
      if data != &CppBuiltInNumericType::Bool {
        return true;
      }
    }
    false
  }

  pub fn can_be_the_same_as(&self, other_type: &CppType) -> bool {
    if self == other_type {
      return true;
    }
    if self.indirection != other_type.indirection || self.is_const != other_type.is_const ||
       self.is_const2 != other_type.is_const2 {
      return false;
    }
    if let CppTypeBase::Class(CppTypeClassBase { ref name, ref template_arguments, .. }) =
           self.base {
      if let Some(ref template_arguments) = *template_arguments {
        let name1 = name;
        let args1 = template_arguments;
        if let CppTypeBase::Class(CppTypeClassBase { ref name, ref template_arguments, .. }) =
               other_type.base {
          if let Some(ref template_arguments) = *template_arguments {
            return name1 == name && args1.len() == template_arguments.len() &&
                   args1.iter()
              .zip(template_arguments.iter())
              .all(|(a1, a2)| a1.can_be_the_same_as(a2));
          }
        }
      }
    }
    if let CppTypeBase::BuiltInNumeric(ref data) = self.base {
      let data1 = data;
      if let CppTypeBase::BuiltInNumeric(ref data) = other_type.base {
        if data1.is_float() {
          return data.is_float();
        } else if data1.is_signed_integer() {
          return data.is_signed_integer() || data.is_integer_with_undefined_signedness();
        } else if data1.is_unsigned_integer() {
          return data.is_unsigned_integer() || data.is_integer_with_undefined_signedness();
        } else if data1.is_integer_with_undefined_signedness() {
          return data.is_signed_integer() || data.is_unsigned_integer() ||
                 data.is_integer_with_undefined_signedness();
        } else {
          return false;
        }
      }
      if let CppTypeBase::SpecificNumeric { ref kind, .. } = other_type.base {
        if data1.is_float() {
          return kind == &CppSpecificNumericTypeKind::FloatingPoint;
        } else if data1.is_signed_integer() {
          return kind == &CppSpecificNumericTypeKind::Integer { is_signed: true };
        } else if data1.is_unsigned_integer() {
          return kind == &CppSpecificNumericTypeKind::Integer { is_signed: false };
        } else if data1.is_integer_with_undefined_signedness() {
          if let CppSpecificNumericTypeKind::Integer { .. } = *kind {
            return true;
          } else {
            return false;
          }
        } else {
          return false;
        }
      }
      return false;
    }
    if let CppTypeBase::BuiltInNumeric(..) = other_type.base {
      if let CppTypeBase::SpecificNumeric { .. } = self.base {
        return other_type.can_be_the_same_as(self);
      }
    }
    false
  }
}
