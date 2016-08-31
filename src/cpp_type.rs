
use cpp_ffi_data::{CppFfiType, IndirectionChange};
use caption_strategy::TypeCaptionStrategy;
pub use serializable::{CppBuiltInNumericType, CppSpecificNumericTypeKind, CppTypeBase, CppType,
                       CppTypeIndirection, CppTypeClassBase};
extern crate regex;

impl CppTypeIndirection {
  pub fn combine(left: &CppTypeIndirection,
                 right: &CppTypeIndirection)
                 -> Result<CppTypeIndirection, String> {
    let error_text = || format!("too much indirection: {:?} to {:?}", left, right);
    Ok(match *left {
      CppTypeIndirection::None => right.clone(),
      CppTypeIndirection::Ptr => {
        match *right {
          CppTypeIndirection::None => CppTypeIndirection::Ptr,
          CppTypeIndirection::Ptr => CppTypeIndirection::PtrPtr,
          CppTypeIndirection::Ref => CppTypeIndirection::PtrRef,
          _ => return Err(error_text()),
        }
      }
      CppTypeIndirection::Ref => {
        match *right {
          CppTypeIndirection::None => CppTypeIndirection::Ref,
          _ => return Err(error_text()),
        }
      }
      _ => {
        match *right {
          CppTypeIndirection::None => left.clone(),
          _ => return Err(error_text()),
        }
      }
    })
  }
}

impl CppBuiltInNumericType {
  pub fn to_cpp_code(&self) -> &'static str {
    match *self {
      CppBuiltInNumericType::Bool => "bool",
      CppBuiltInNumericType::Char => "char",
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

  pub fn all() -> [CppBuiltInNumericType; 20] {
    [CppBuiltInNumericType::Bool,
     CppBuiltInNumericType::Char,
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


impl CppTypeClassBase {
  pub fn to_cpp_code(&self) -> Result<String, String> {
    match self.template_arguments {
      Some(ref args) => {
        let mut arg_texts = Vec::new();
        for arg in args {
          arg_texts.push(try!(arg.to_cpp_code(None)));
        }
        Ok(format!("{}< {} >", self.name, arg_texts.join(", ")))
      }
      None => Ok(self.name.clone()),
    }

  }
  pub fn caption(&self) -> String {
    let name_caption = self.name.replace("::", "_");
    match self.template_arguments {
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

  pub fn instantiate_class(&self,
                           nested_level1: i32,
                           template_arguments1: &Vec<CppType>)
                           -> Result<CppTypeClassBase, String> {
    Ok(CppTypeClassBase {
      name: self.name.clone(),
      template_arguments: match self.template_arguments {
        Some(ref template_arguments) => {
          let mut args = Vec::new();
          for arg in template_arguments {
            args.push(try!(arg.instantiate(nested_level1, template_arguments1)));
          }
          Some(args)
        }
        None => None,
      },
    })
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
      &CppTypeBase::Class(..) => true,
      _ => false,
    }
  }
  pub fn is_template_parameter(&self) -> bool {
    match self {
      &CppTypeBase::TemplateParameter { .. } => true,
      _ => false,
    }
  }
  pub fn is_or_contains_template_parameter(&self) -> bool {
    match self {
      &CppTypeBase::TemplateParameter { .. } => true,
      &CppTypeBase::Class(CppTypeClassBase { ref template_arguments, .. }) => {
        if let &Some(ref template_arguments) = template_arguments {
          template_arguments.iter()
            .find(|arg| arg.base.is_or_contains_template_parameter())
            .is_some()
        } else {
          false
        }
      }
      _ => false,
    }
  }

  pub fn to_cpp_code(&self,
                     function_pointer_inner_text: Option<&String>)
                     -> Result<String, String> {
    match *self {
      CppTypeBase::FunctionPointer { .. } => {}
      _ => {
        if function_pointer_inner_text.is_some() {
          return Err("unexpected function_pointer_inner_text".to_string());
        }
      }
    }
    match *self {
      CppTypeBase::Void => Ok("void".to_string()),
      CppTypeBase::BuiltInNumeric(ref t) => Ok(t.to_cpp_code().to_string()),
      CppTypeBase::Enum { ref name } => Ok(name.clone()),
      CppTypeBase::SpecificNumeric { ref name, .. } => Ok(name.clone()),
      CppTypeBase::PointerSizedInteger { ref name, .. } => Ok(name.clone()),
      CppTypeBase::Class(ref info) => info.to_cpp_code(),
      CppTypeBase::TemplateParameter { .. } => {
        return Err(format!("template parameters are not allowed to produce C++ code without \
                            instantiation"));
      }
      CppTypeBase::FunctionPointer { ref return_type,
                                     ref arguments,
                                     ref allows_variadic_arguments } => {
        if *allows_variadic_arguments {
          return Err(format!("Function pointers with variadic arguments are not supported"));
        }
        let mut arg_texts = Vec::new();
        for arg in arguments {
          arg_texts.push(try!(arg.to_cpp_code(None)));
        }
        if function_pointer_inner_text.is_none() {
          return Err("function_pointer_inner_text argument is required".to_string());
        }
        Ok(format!("{} (*{})({})",
                   try!(return_type.as_ref().to_cpp_code(None)),
                   function_pointer_inner_text.unwrap(),
                   arg_texts.join(", ")))
      }
    }
  }

  #[allow(dead_code)]
  pub fn maybe_name(&self) -> Option<&String> {
    match *self {
      CppTypeBase::SpecificNumeric { ref name, .. } => Some(name),
      CppTypeBase::PointerSizedInteger { ref name, .. } => Some(name),
      CppTypeBase::Enum { ref name } => Some(name),
      CppTypeBase::Class(CppTypeClassBase { ref name, .. }) => Some(name),
      _ => None,
    }
  }

  /// Generates alphanumeric representation of self
  /// used to generate FFI function names
  pub fn caption(&self) -> String {
    match *self {
      CppTypeBase::Void => "void".to_string(),
      CppTypeBase::BuiltInNumeric(ref t) => t.to_cpp_code().to_string().replace(" ", "_"),
      CppTypeBase::SpecificNumeric { ref name, .. } => name.clone(),
      CppTypeBase::PointerSizedInteger { ref name, .. } => name.clone(),
      CppTypeBase::Enum { ref name } => name.replace("::", "_"),
      CppTypeBase::Class(ref data) => data.caption(),
      CppTypeBase::TemplateParameter { .. } => {
        panic!("template parameters are not allowed to have captions");
      }
      CppTypeBase::FunctionPointer { .. } => "func".to_string(),
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

  pub fn to_cpp_code(&self,
                     function_pointer_inner_text: Option<&String>)
                     -> Result<String, String> {
    let name = try!(self.base.to_cpp_code(function_pointer_inner_text));
    Ok(format!("{}{}{}",
               if self.is_const { "const " } else { "" },
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

  /// Converts this C++ type to its adaptation for FFI interface,
  /// removing all features not supported by C ABI
  /// (e.g. references and passing objects by value)
  pub fn to_cpp_ffi_type(&self, role: CppTypeRole) -> Result<CppFfiType, String> {
    match self.base {
      CppTypeBase::TemplateParameter { .. } => {
        return Err(format!("Unsupported type"));
      }
      CppTypeBase::FunctionPointer { ref return_type,
                                     ref arguments,
                                     ref allows_variadic_arguments } => {
        if *allows_variadic_arguments {
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
            CppTypeIndirection::Ptr |
            CppTypeIndirection::PtrPtr => {}
            CppTypeIndirection::None => {
              if arg.base.is_class() {
                return Err(format!("Function pointers containing classes by value are not \
                                    supported"));
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
    if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = self.base {
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

  /// Generates alphanumeric representation of self
  /// used to generate FFI function names
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

  /// Checks if a function with this return type would need
  /// to have 2 wrappers with 2 different return value allocation places
  pub fn needs_allocation_place_variants(&self) -> bool {
    if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = self.base {
      if name == "QFlags" {
        return false; // converted to uint in FFI
      }
    }
    return self.indirection == CppTypeIndirection::None && self.base.is_class();
  }

  pub fn instantiate(&self,
                     nested_level1: i32,
                     template_arguments1: &Vec<CppType>)
                     -> Result<CppType, String> {
    if let CppTypeBase::TemplateParameter { nested_level, index } = self.base {
      if nested_level == nested_level1 {
        if index < 0 {
          panic!("CppType::instantiate: index < 0");
        }
        // TODO: Err instead of panic
        if index >= template_arguments1.len() as i32 {
          return Err("CppType::instantiate: too few template arguments".to_string());
        }
        let mut new_type = template_arguments1[index as usize].clone();
        if self.is_const {
          new_type.is_const = true;
        }
        // TODO: Err instead of panic
        new_type.indirection = try!(CppTypeIndirection::combine(&new_type.indirection,
                                                                &self.indirection));
        return Ok(new_type);
      }
    }
    Ok(CppType {
      is_const: self.is_const,
      indirection: self.indirection.clone(),
      base: match self.base {
        CppTypeBase::Class(ref data) => {
          CppTypeBase::Class(try!(data.instantiate_class(nested_level1, template_arguments1)))
        }
        _ => self.base.clone(),
      },
    })

  }
}
