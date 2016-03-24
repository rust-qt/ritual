use cpp_type::CppType;
use enums::{CppMethodScope, AllocationPlace, AllocationPlaceImportance, IndirectionChange, CFunctionArgumentCppEquivalent};
use c_function_signature::CFunctionSignature;
use c_type::{CType, CTypeExtended, CppToCTypeConversion};
use c_function_argument::CFunctionArgument;
use cpp_and_c_method::CppMethodWithCSignature;
use utils::operator_c_name;
use caption_strategy::MethodCaptionStrategy;

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
          is_template: false, // TODO: is template if base class is template
          is_reference: false,
          is_pointer: false,
          is_const: false,
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
                     allocation_place: AllocationPlace)
                     -> Option<(CFunctionSignature, AllocationPlaceImportance)> {
    if self.is_variable || self.allows_variable_arguments {
      // no complicated cases support for now
      // TODO: return Err
      println!("Variable arguments are not supported");
      return None;
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
            is_primitive: false,
            conversion: CppToCTypeConversion {
              indirection_change: IndirectionChange::NoChange,
              renamed: false,
            },
          },
          cpp_equivalent: CFunctionArgumentCppEquivalent::This,
        });
      }
    }
    for (index, arg) in self.arguments.iter().enumerate() {
      match arg.argument_type.to_c_type() {
        Some(c_type) => {
          r.arguments.push(CFunctionArgument {
            name: arg.name.clone(),
            argument_type: c_type,
            cpp_equivalent: CFunctionArgumentCppEquivalent::Argument(index as i8),
          });
        }
        None => {
          println!("Can't convert type to C: {:?}", arg.argument_type);
          return None;
        }
      }
    }
    if let Some(return_type) = self.real_return_type() {
      match return_type.to_c_type() {
        Some(c_type) => {
          if return_type.is_stack_allocated_struct() {
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
        None => return None,
      }
    }
    if self.is_destructor {
      allocation_place_importance = AllocationPlaceImportance::Important;
    }
    Some((r, allocation_place_importance))
  }

  fn add_c_signatures(&self)
                      -> (Option<CppMethodWithCSignature>,
                          Option<CppMethodWithCSignature>) {
    match self.c_signature(AllocationPlace::Heap) {
      Some((c_signature, importance)) => {
        let result1 = Some(CppMethodWithCSignature {
          cpp_method: self.clone(),
          allocation_place: AllocationPlace::Heap,
          c_signature: c_signature,
        });
        match importance {
          AllocationPlaceImportance::Important => {
            let result2 = match self.c_signature(AllocationPlace::Stack) {
              Some((c_signature2, _)) => {
                Some(CppMethodWithCSignature {
                  cpp_method: self.clone(),
                  allocation_place: AllocationPlace::Stack,
                  c_signature: c_signature2,
                })
              }
              None => None,
            };
            (result1, result2)
          }
          AllocationPlaceImportance::NotImportant => (result1, None),
        }
      }
      None => (None, None),
    }
  }

  pub fn c_base_name(&self) -> String {
    let scope_prefix = match self.cpp_method.scope {
      CppMethodScope::Class(..) => "".to_string(),
      CppMethodScope::Global => "G_".to_string(),
    };
    let method_name = if self.cpp_method.is_constructor {
      match self.allocation_place {
        AllocationPlace::Stack => "constructor".to_string(),
        AllocationPlace::Heap => "new".to_string(),
      }
    } else if self.cpp_method.is_destructor {
      match self.allocation_place {
        AllocationPlace::Stack => "destructor".to_string(),
        AllocationPlace::Heap => "delete".to_string(),
      }
    } else if let Some(ref operator) = self.cpp_method.operator {
      "OP_".to_string() + &operator_c_name(operator, self.cpp_method.real_arguments_count())
    } else {
      self.cpp_method.name.clone()
    };
    scope_prefix + &method_name
  }

  fn caption(&self, strategy: MethodCaptionStrategy) -> String {
    match strategy {
      MethodCaptionStrategy::ArgumentsOnly(s) => self.c_signature.caption(s),
      MethodCaptionStrategy::ConstOnly => {
        if self.cpp_method.is_const {
          "const".to_string()
        } else {
          "".to_string()
        }
      }
      MethodCaptionStrategy::ConstAndArguments(s) => {
        let r = if self.cpp_method.is_const {
          "const_".to_string()
        } else {
          "".to_string()
        };
        r + &self.c_signature.caption(s)
      }
    }



  }
}
