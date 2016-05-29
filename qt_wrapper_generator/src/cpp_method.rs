use cpp_type::{CppType, CppTypeBase, CppFfiType};
use enums::{CppMethodScope, AllocationPlace, AllocationPlaceImportance, CppFfiArgumentMeaning,
            CppTypeIndirection, CppTypeOrigin, CppVisibility};
use cpp_ffi_function_signature::CppFfiFunctionSignature;
use cpp_ffi_function_argument::CppFfiFunctionArgument;
use cpp_and_ffi_method::CppMethodWithFfiSignature;
use utils::JoinWithString;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppFunctionArgument {
  pub name: String,
  pub argument_type: CppType,
  pub default_value: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppMethod {
  pub name: String,
  pub scope: CppMethodScope,
  pub is_virtual: bool,
  pub is_pure_virtual: bool,
  pub is_const: bool,
  pub is_static: bool,
  pub visibility: CppVisibility,
  pub is_signal: bool,
  pub return_type: Option<CppType>,
  pub is_constructor: bool,
  pub is_destructor: bool,
  pub operator: Option<String>,
  pub conversion_operator: Option<CppType>,
  pub is_variable: bool,
  pub arguments: Vec<CppFunctionArgument>,
  pub allows_variable_arguments: bool,
  pub original_index: i32,
  pub origin: CppTypeOrigin,
  pub template_arguments: Option<Vec<String>>,
}

impl CppMethod {
  pub fn argument_types_equal(&self, other: &CppMethod) -> bool {
    if self.arguments.len() != other.arguments.len() {
      return false;
    }
    for i in 0..self.arguments.len() {
      if self.arguments.get(i).unwrap().argument_type !=
         other.arguments.get(i).unwrap().argument_type {
        return false;
      }
    }
    true
  }

  pub fn real_return_type(&self) -> Option<CppType> {
    if self.is_constructor {
      if let CppMethodScope::Class(ref class_name) = self.scope {
        return Some(CppType {
          is_const: false,
          indirection: CppTypeIndirection::None,
          base: CppTypeBase::Class {
            name: class_name.clone(),
            template_arguments: None, // TODO: report template arguments
          },
        });
      } else {
        panic!("constructor encountered with no class scope");
      }
    } else {
      return self.return_type.clone();
    }
  }

  pub fn real_arguments_count(&self) -> i32 {
    // println!("real_arguments_count called for {:?}", self);
    let mut result = self.arguments.len() as i32;
    if let CppMethodScope::Class(..) = self.scope {
      // println!("ok1");
      if !self.is_static {
        // println!("ok2");
        result += 1;
      }
    }
    result
  }

  pub fn c_signature(&self,
                     allocation_place: AllocationPlace)
                     -> Result<(CppFfiFunctionSignature, AllocationPlaceImportance), String> {

    // no complicated cases support for now
    if self.is_variable {
      return Err("Variables are not supported".to_string());
    }
    if self.allows_variable_arguments {
      return Err("Variable arguments are not supported".to_string());
    }
    let mut allocation_place_importance = AllocationPlaceImportance::NotImportant;
    let mut r = CppFfiFunctionSignature {
      arguments: Vec::new(),
      return_type: CppFfiType::void(),
    };
    if let CppMethodScope::Class(ref class_name) = self.scope {
      if !self.is_static && !self.is_constructor {
        r.arguments.push(CppFfiFunctionArgument {
          name: "this_ptr".to_string(),
          argument_type: CppType {
                           base: CppTypeBase::Class {
                             name: class_name.clone(),
                             template_arguments: None, // TODO: report template arguments
                           },
                           is_const: self.is_const,
                           indirection: CppTypeIndirection::Ptr,
                         }
                         .to_cpp_ffi_type()
                         .unwrap(),
          cpp_equivalent: CppFfiArgumentMeaning::This,
        });
      }
    }
    for (index, arg) in self.arguments.iter().enumerate() {
      match arg.argument_type.to_cpp_ffi_type() {
        Ok(c_type) => {
          r.arguments.push(CppFfiFunctionArgument {
            name: arg.name.clone(),
            argument_type: c_type,
            cpp_equivalent: CppFfiArgumentMeaning::Argument(index as i8),
          });
        }
        Err(msg) => {
          return Err(format!("Can't convert type to C: {:?}: {}", arg.argument_type, msg));
        }
      }
    }
    if let Some(return_type) = self.real_return_type() {
      match return_type.to_cpp_ffi_type() {
        Ok(c_type) => {
          let is_stack_allocated_struct = return_type.indirection == CppTypeIndirection::None &&
                                          return_type.base.is_class() &&
                                          !c_type.conversion.qflags_to_uint;
          if is_stack_allocated_struct {
            allocation_place_importance = AllocationPlaceImportance::Important;
            if allocation_place == AllocationPlace::Stack {
              r.arguments.push(CppFfiFunctionArgument {
                name: "output".to_string(),
                argument_type: c_type,
                cpp_equivalent: CppFfiArgumentMeaning::ReturnValue,
              });
            } else {
              r.return_type = c_type;
            }
          } else {
            r.return_type = c_type;
          }
        }
        Err(msg) => {
          return Err(format!("Can't convert type to C: {:?}: {}", return_type, msg));
        }
      }
    }
    if self.is_destructor {
      allocation_place_importance = AllocationPlaceImportance::Important;
    }
    Ok((r, allocation_place_importance))
  }

  pub fn add_c_signatures
    (&self)
     -> Result<(CppMethodWithFfiSignature, Option<CppMethodWithFfiSignature>), String> {
    match self.c_signature(AllocationPlace::Heap) {
      Ok((c_signature, importance)) => {
        let result1 = CppMethodWithFfiSignature {
          cpp_method: self.clone(),
          allocation_place: AllocationPlace::Heap,
          c_signature: c_signature,
        };
        match importance {
          AllocationPlaceImportance::Important => {
            match self.c_signature(AllocationPlace::Stack) {
              Ok((c_signature2, _)) => {
                Ok((result1,
                    Some(CppMethodWithFfiSignature {
                  cpp_method: self.clone(),
                  allocation_place: AllocationPlace::Stack,
                  c_signature: c_signature2,
                })))
              }
              Err(msg) => Err(msg),
            }
          }
          AllocationPlaceImportance::NotImportant => Ok((result1, None)),
        }
      }
      Err(msg) => Err(msg),
    }
  }

  pub fn full_name(&self) -> String {
    if let CppMethodScope::Class(ref name) = self.scope {
      format!("{}::{}", name, self.name)
    } else {
      self.name.clone()
    }
  }

  pub fn short_text(&self) -> String {
    let mut s = String::new();
    if self.is_virtual {
      s = format!("{} virtual", s);
    }
    if self.is_static {
      s = format!("{} static", s);
    }
    if self.visibility == CppVisibility::Protected {
      s = format!("{} protected", s);
    }
    if self.visibility == CppVisibility::Private {
      s = format!("{} private", s);
    }
    if self.is_signal {
      s = format!("{} [signal]", s);
    }
    if self.allows_variable_arguments {
      s = format!("{} [var args]", s);
    }
    if self.is_variable {
      s = format!("{} [variable]", s);
    }
    if self.is_constructor {
      s = format!("{} [constructor]", s);
    }
    if self.is_destructor {
      s = format!("{} [destructor]", s);
    }
    if let Some(ref op) = self.operator {
      s = format!("{} [operator \"{}\"]", s, op);
    }
    if let Some(ref cpp_type) = self.return_type {
      s = format!("{} {}",
                  s,
                  cpp_type.to_cpp_code().unwrap_or("[?]".to_string()));
    }
    if let CppMethodScope::Class(ref name) = self.scope {
      s = format!("{} {}::", s, name);
    }
    s = format!("{}{}", s, self.name);
    if !self.is_variable {
      s = format!("{}({})",
                  s,
                  self.arguments
                      .iter()
                      .map(|arg| {
                        format!("{} {}{}",
                                arg.argument_type.to_cpp_code().unwrap_or("[?]".to_string()),
                                arg.name,
                                if let Some(ref dv) = arg.default_value {
                                  format!(" = {}", dv)
                                } else {
                                  String::new()
                                })
                      })
                      .join(", "));
    }
    if self.is_pure_virtual {
      s = format!("{} = 0", s);
    }
    if self.is_const {
      s = format!("{} const", s);
    }
    s.trim().to_string()
  }
}
