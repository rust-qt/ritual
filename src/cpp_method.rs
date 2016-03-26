use cpp_type::CppType;
use enums::{CppMethodScope, AllocationPlace, AllocationPlaceImportance, IndirectionChange,
            CFunctionArgumentCppEquivalent, CppTypeIndirection, CppTypeKind};
use c_function_signature::CFunctionSignature;
use c_type::{CType, CTypeExtended, CppToCTypeConversion};
use c_function_argument::CFunctionArgument;
use cpp_and_c_method::CppMethodWithCSignature;
use utils::operator_c_name;
use caption_strategy::MethodCaptionStrategy;
use cpp_type_map::CppTypeMap;

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
  pub is_const: bool,
  pub is_static: bool,
  pub return_type: Option<CppType>,
  pub is_constructor: bool,
  pub is_destructor: bool,
  pub operator: Option<String>,
  pub is_variable: bool,
  pub arguments: Vec<CppFunctionArgument>,
  pub allows_variable_arguments: bool,
  pub original_index: i32,
}

impl CppMethod {
  fn real_return_type(&self) -> Option<CppType> {
    if self.is_constructor {
      if let CppMethodScope::Class(ref class_name) = self.scope {
        return Some(CppType {
          template_arguments: None, // TODO: figure out template arguments
          is_const: false,
          indirection: CppTypeIndirection::None,
          base: class_name.clone(),
        });
      } else {
        panic!("constructor encountered with no class scope");
      }
    } else {
      return self.return_type.clone();
    }
  }

  fn real_arguments_count(&self) -> i32 {
    let mut result = self.arguments.len() as i32;
    if let CppMethodScope::Class(..) = self.scope {
      if !self.is_static {
        result += 1;
      }
    }
    result
  }

  pub fn c_signature(&self,
                     cpp_type_map: &CppTypeMap,
                     allocation_place: AllocationPlace)
                     -> Result<(CFunctionSignature, AllocationPlaceImportance), String> {

    // no complicated cases support for now
    if self.is_variable {
      return Err("Variables are not supported".to_string());
    }
    if self.allows_variable_arguments {
      return Err("Variable arguments are not supported".to_string());
    }
    let mut allocation_place_importance = AllocationPlaceImportance::NotImportant;
    let mut r = CFunctionSignature {
      arguments: Vec::new(),
      return_type: CTypeExtended::void(),
    };
    if let CppMethodScope::Class(ref class_name) = self.scope {
      if !self.is_static && !self.is_constructor {
        r.arguments.push(CFunctionArgument {
          name: "self".to_string(),
          argument_type: CTypeExtended {
            c_type: CType {
              base: class_name.clone(),
              is_pointer: true,
              is_const: self.is_const,
            },
            cpp_type: CppType {
              base: class_name.clone(),
              template_arguments: None, // TODO: figure out template arguments
              is_const: self.is_const,
              indirection: CppTypeIndirection::Ptr,
            },
            conversion: CppToCTypeConversion {
              indirection_change: IndirectionChange::NoChange,
              renamed: false,
              qflags_to_uint: false,
            },
          },
          cpp_equivalent: CFunctionArgumentCppEquivalent::This,
        });
      }
    }
    for (index, arg) in self.arguments.iter().enumerate() {
      match arg.argument_type.to_c_type(cpp_type_map) {
        Ok(c_type) => {
          r.arguments.push(CFunctionArgument {
            name: arg.name.clone(),
            argument_type: c_type,
            cpp_equivalent: CFunctionArgumentCppEquivalent::Argument(index as i8),
          });
        }
        Err(msg) => {
          return Err(format!("Can't convert type to C: {:?}: {}", arg.argument_type, msg));
        }
      }
    }
    if let Some(return_type) = self.real_return_type() {
      match return_type.to_c_type(cpp_type_map) {
        Ok(c_type) => {
          let is_stack_allocated_struct = if return_type.indirection == CppTypeIndirection::None {
            match cpp_type_map.get_info(&return_type.base).unwrap().kind {
              CppTypeKind::Class { .. } => true,
              _ => false
            }
          } else {
            false
          };
          if is_stack_allocated_struct {
            allocation_place_importance = AllocationPlaceImportance::Important;
            if allocation_place == AllocationPlace::Stack {
              r.arguments.push(CFunctionArgument {
                name: "output".to_string(),
                argument_type: c_type,
                cpp_equivalent: CFunctionArgumentCppEquivalent::ReturnValue,
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

  fn add_c_signatures
    (&self,
     cpp_type_map: &CppTypeMap)
     -> Result<(CppMethodWithCSignature, Option<CppMethodWithCSignature>), String> {
    match self.c_signature(cpp_type_map, AllocationPlace::Heap) {
      Ok((c_signature, importance)) => {
        let result1 = CppMethodWithCSignature {
          cpp_method: self.clone(),
          allocation_place: AllocationPlace::Heap,
          c_signature: c_signature,
        };
        match importance {
          AllocationPlaceImportance::Important => {
            match self.c_signature(cpp_type_map, AllocationPlace::Stack) {
              Ok((c_signature2, _)) => {
                Ok((result1,
                    Some(CppMethodWithCSignature {
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
}
