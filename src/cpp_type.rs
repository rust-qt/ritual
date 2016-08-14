
use cpp_ffi_type::{CppFfiType, IndirectionChange};
use caption_strategy::TypeCaptionStrategy;
pub use serializable::{CppBuiltInNumericType, CppSpecificNumericTypeKind, CppTypeBase, CppType,
                       CppTypeIndirection};
extern crate regex;


impl CppBuiltInNumericType {
  pub fn to_cpp_code(&self) -> &'static str {
    match *self {
      CppBuiltInNumericType::Bool => "bool",
      CppBuiltInNumericType::CharS => "char",
      CppBuiltInNumericType::CharU => "char",
      CppBuiltInNumericType::SChar => "signed char",
      CppBuiltInNumericType::UChar => "unsigned char",
      CppBuiltInNumericType::WChar => "wchar_t",
      CppBuiltInNumericType::Char16 => "char16_t",
      CppBuiltInNumericType::Char32 => "char32_t",
      CppBuiltInNumericType::Short => "short",
      CppBuiltInNumericType::UShort => "unsigned short",
      CppBuiltInNumericType::Int => "int",
      CppBuiltInNumericType::UInt => "unsigned int",
      CppBuiltInNumericType::Long => "long",
      CppBuiltInNumericType::ULong => "unsigned long",
      CppBuiltInNumericType::LongLong => "long long",
      CppBuiltInNumericType::ULongLong => "unsigned long long",
      CppBuiltInNumericType::Int128 => "__int128_t",
      CppBuiltInNumericType::UInt128 => "__uint128_t",
      CppBuiltInNumericType::Float => "float",
      CppBuiltInNumericType::Double => "double",
      CppBuiltInNumericType::LongDouble => "long double",
    }
  }

  pub fn all() -> [CppBuiltInNumericType; 21] {
    [CppBuiltInNumericType::Bool,
     CppBuiltInNumericType::CharS,
     CppBuiltInNumericType::CharU,
     CppBuiltInNumericType::SChar,
     CppBuiltInNumericType::UChar,
     CppBuiltInNumericType::WChar,
     CppBuiltInNumericType::Char16,
     CppBuiltInNumericType::Char32,
     CppBuiltInNumericType::Short,
     CppBuiltInNumericType::UShort,
     CppBuiltInNumericType::Int,
     CppBuiltInNumericType::UInt,
     CppBuiltInNumericType::Long,
     CppBuiltInNumericType::ULong,
     CppBuiltInNumericType::LongLong,
     CppBuiltInNumericType::ULongLong,
     CppBuiltInNumericType::Int128,
     CppBuiltInNumericType::UInt128,
     CppBuiltInNumericType::Float,
     CppBuiltInNumericType::Double,
     CppBuiltInNumericType::LongDouble]
  }
}




impl CppTypeBase {
  #[allow(dead_code)]
  pub fn is_void(&self) -> bool {
    match self {
      &CppTypeBase::Void => true,
      _ => false,
    }
  }
  pub fn is_class(&self) -> bool {
    match self {
      &CppTypeBase::Class { .. } => true,
      _ => false,
    }
  }
  pub fn is_template_parameter(&self) -> bool {
    match self {
      &CppTypeBase::TemplateParameter { .. } => true,
      _ => false,
    }
  }
  pub fn to_cpp_code(&self) -> Result<String, String> {
    match *self {
      CppTypeBase::Void => Ok("void".to_string()),
      CppTypeBase::BuiltInNumeric(ref t) => Ok(t.to_cpp_code().to_string()),
      CppTypeBase::Enum { ref name } => Ok(name.clone()),
      CppTypeBase::SpecificNumeric { ref name, .. } => Ok(name.clone()),
      CppTypeBase::PointerSizedInteger { ref name, .. } => Ok(name.clone()),
      CppTypeBase::Class { ref name, ref template_arguments } => {
        match *template_arguments {
          Some(ref args) => {
            let mut arg_texts = Vec::new();
            for arg in args {
              arg_texts.push(try!(arg.to_cpp_code()));
            }
            Ok(format!("{}< {} >", name, arg_texts.join(", ")))
          }
          None => Ok(name.clone()),
        }
      }
      CppTypeBase::TemplateParameter { .. } => {
        return Err(format!("template parameters are not supported here yet"));
      }
      CppTypeBase::FunctionPointer { ref return_type,
                                     ref arguments,
                                     ref allows_variable_arguments } => {
        if *allows_variable_arguments {
          return Err(format!("Function pointers with variadic arguments are not supported"));
        }
        let mut arg_texts = Vec::new();
        for arg in arguments {
          arg_texts.push(try!(arg.to_cpp_code()));
        }
        Ok(format!("{} (*FN_PTR)({})",
                   try!(return_type.as_ref().to_cpp_code()),
                   arg_texts.join(", ")))
      }
    }
  }
  pub fn caption(&self) -> String {
    match *self {
      CppTypeBase::Void => "void".to_string(),
      CppTypeBase::BuiltInNumeric(ref t) => t.to_cpp_code().to_string().replace(" ", "_"),
      CppTypeBase::SpecificNumeric { ref name, .. } => name.clone(),
      CppTypeBase::PointerSizedInteger { ref name, .. } => name.clone(),
      CppTypeBase::Enum { ref name } => name.replace("::", "_"),
      CppTypeBase::Class { ref name, ref template_arguments } => {
        let name_caption = name.replace("::", "_");
        match *template_arguments {
          Some(ref args) => {
            let mut arg_texts = Vec::new();
            for arg in args {
              arg_texts.push(arg.caption(TypeCaptionStrategy::Full));
            }
            format!("{}_{}", name_caption, arg_texts.join("_"))
          }
          None => name_caption,
        }
      }
      CppTypeBase::TemplateParameter { .. } => {
        panic!("template parameters are not supported here yet");
      }
      CppTypeBase::FunctionPointer { .. } => {
        panic!("function pointers are not supported here yet");
      }
    }
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
      indirection: CppTypeIndirection::None,
      base: CppTypeBase::Void,
    }
  }

  pub fn is_void(&self) -> bool {
    !self.is_const && self.indirection == CppTypeIndirection::None && self.base == CppTypeBase::Void
  }

  pub fn to_cpp_code(&self) -> Result<String, String> {
    let name = try!(self.base.to_cpp_code());
    Ok(format!("{}{}{}",
               if self.is_const {
                 "const "
               } else {
                 ""
               },
               name,
               match self.indirection {
                 CppTypeIndirection::None => "",
                 CppTypeIndirection::Ptr => "*",
                 CppTypeIndirection::Ref => "&",
                 CppTypeIndirection::PtrRef => "*&",
                 CppTypeIndirection::PtrPtr => "**",
                 CppTypeIndirection::RValueRef => "&&",
               }))
  }

  pub fn to_cpp_ffi_type(&self, role: CppTypeRole) -> Result<CppFfiType, String> {
    match self.base {
      CppTypeBase::TemplateParameter { .. } => {
        return Err(format!("Unsupported type"));
      }
      CppTypeBase::FunctionPointer { ref return_type,
                                     ref arguments,
                                     ref allows_variable_arguments } => {
        if *allows_variable_arguments {
          return Err(format!("Function pointers with variadic arguments are not supported"));
        }
        let mut all_types: Vec<&CppType> = arguments.iter().collect();
        all_types.push(return_type.as_ref());
        for arg in all_types {
          match arg.base {
            CppTypeBase::TemplateParameter { .. } => {
              return Err(format!("Function pointers containing template parameters are not \
                                  supported"));
            }
            CppTypeBase::FunctionPointer { .. } => {
              return Err(format!("Function pointers containing nested function pointers are not \
                                  supported"));
            }
            _ => {}
          }
          match arg.indirection {
            CppTypeIndirection::Ref |
            CppTypeIndirection::PtrRef |
            CppTypeIndirection::RValueRef => {
              return Err(format!("Function pointers containing references are not supported"));
            }
            CppTypeIndirection::Ptr | CppTypeIndirection::PtrPtr => {}
            CppTypeIndirection::None => {
              match arg.base {
                CppTypeBase::Class { .. } => {
                  return Err(format!("Function pointers containing classes by value are not \
                                      supported"));
                }
                _ => {}
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
      _ => return Err("Unsupported level of indirection".to_string()),
    }
    if let CppTypeBase::Class { ref name, .. } = self.base {
      if name == "QFlags" {
        assert!(self.indirection == CppTypeIndirection::None);
        conversion = IndirectionChange::QFlagsToUInt;
        result.base = CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::UInt);
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

  pub fn caption(&self, strategy: TypeCaptionStrategy) -> String {
    match strategy {
      TypeCaptionStrategy::Short => self.base.caption(),
      TypeCaptionStrategy::Full => {
        let mut r = self.base.caption();
        match self.indirection {
          CppTypeIndirection::None => {}
          CppTypeIndirection::Ptr => r = format!("{}_ptr", r),
          CppTypeIndirection::Ref => r = format!("{}_ref", r),
          CppTypeIndirection::PtrRef => r = format!("{}_ptr_ref", r),
          CppTypeIndirection::PtrPtr => r = format!("{}_ptr_ptr", r),
          CppTypeIndirection::RValueRef => r = format!("{}_rvalue_ref", r),
        }
        if self.is_const {
          r = format!("const_{}", r);
        }
        r
      }
    }
  }
}


#[cfg(test)]
mod tests {
  use cpp_type::{CppType, CppTypeRole};
  use caption_strategy::TypeCaptionStrategy;
  use cpp_ffi_type::IndirectionChange;

  #[test]
  fn test1() {
    let type1 = CppType::void();
    assert_eq!(type1.is_void(), true);
    assert_eq!(type1.base.is_void(), true);
    assert_eq!(type1.base.is_class(), false);
    assert_eq!(type1.base.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code().unwrap(), "void");
    assert_eq!(type1.base.to_cpp_code().unwrap(), "void");
    assert_eq!(type1.base.caption(), "void");

    let ffi1 = type1.to_cpp_ffi_type(CppTypeRole::NotReturnType).unwrap();
    assert_eq!(ffi1.original_type, type1);
    assert_eq!(ffi1.ffi_type, type1);
    assert_eq!(ffi1.conversion, IndirectionChange::NoChange);

    let ffi2 = type1.to_cpp_ffi_type(CppTypeRole::ReturnType).unwrap();
    assert_eq!(ffi2.original_type, type1);
    assert_eq!(ffi2.ffi_type, type1);
    assert_eq!(ffi2.conversion, IndirectionChange::NoChange);

    assert_eq!(type1.caption(TypeCaptionStrategy::Short), "void");
    assert_eq!(type1.caption(TypeCaptionStrategy::Full), "void");

  }

}

