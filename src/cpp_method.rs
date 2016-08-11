use cpp_type::{CppType, CppTypeIndirection};
use cpp_ffi_type::{CppFfiType, IndirectionChange};
use cpp_ffi_function_signature::CppFfiFunctionSignature;
use cpp_ffi_function_argument::{CppFfiFunctionArgument, CppFfiArgumentMeaning};
use cpp_and_ffi_method::CppMethodWithFfiSignature;
use cpp_data::CppVisibility;
use utils::JoinWithString;
pub use serializable::{CppFunctionArgument, CppMethodScope, CppMethodKind, CppMethod};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ReturnValueAllocationPlace {
  Stack,
  Heap,
  NotApplicable,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AllocationPlaceImportance {
  Important,
  NotImportant,
}





impl CppMethodScope {
  pub fn class_name(&self) -> Option<&String> {
    match *self {
      CppMethodScope::Global => None,
      CppMethodScope::Class(ref s) => Some(s),
    }
  }
}



impl CppMethodKind {
  pub fn is_operator(&self) -> bool {
    match *self {
      CppMethodKind::Operator(..) => true,
      _ => false,
    }
  }
  pub fn is_constructor(&self) -> bool {
    match *self {
      CppMethodKind::Constructor => true,
      _ => false,
    }
  }
  pub fn is_destructor(&self) -> bool {
    match *self {
      CppMethodKind::Destructor => true,
      _ => false,
    }
  }
  #[allow(dead_code)]
  pub fn is_regular(&self) -> bool {
    match *self {
      CppMethodKind::Regular => true,
      _ => false,
    }
  }
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

  pub fn c_signature(&self,
                     allocation_place: ReturnValueAllocationPlace)
                     -> Result<(CppFfiFunctionSignature, AllocationPlaceImportance), String> {

    // no complicated cases support for now
    if self.allows_variable_arguments {
      return Err("Variable arguments are not supported".to_string());
    }
    let mut allocation_place_importance = AllocationPlaceImportance::NotImportant;
    let mut r = CppFfiFunctionSignature {
      arguments: Vec::new(),
      return_type: CppFfiType::void(),
    };
    if let CppMethodScope::Class(..) = self.scope {
      if !self.is_static && self.kind != CppMethodKind::Constructor {
        r.arguments.push(CppFfiFunctionArgument {
          name: "this_ptr".to_string(),
          argument_type: CppType {
                           base: self.class_type.clone().unwrap(),
                           is_const: self.is_const,
                           indirection: CppTypeIndirection::Ptr,
                         }
                         .to_cpp_ffi_type(false)
                         .unwrap(),
          meaning: CppFfiArgumentMeaning::This,
        });
      }
    }
    for (index, arg) in self.arguments.iter().enumerate() {
      match arg.argument_type.to_cpp_ffi_type(false) {
        Ok(c_type) => {
          r.arguments.push(CppFfiFunctionArgument {
            name: arg.name.clone(),
            argument_type: c_type,
            meaning: CppFfiArgumentMeaning::Argument(index as i8),
          });
        }
        Err(msg) => {
          return Err(format!("Can't convert type to C: {:?}: {}", arg.argument_type, msg));
        }
      }
    }
    let real_return_type = if self.kind == CppMethodKind::Constructor {
      Some(CppType {
        is_const: false,
        indirection: CppTypeIndirection::None,
        base: self.class_type.clone().unwrap(),
      })
    } else {
      self.return_type.clone()
    };
    if let Some(return_type) = real_return_type {
      match return_type.to_cpp_ffi_type(true) {
        Ok(c_type) => {
          let is_stack_allocated_struct = return_type.indirection == CppTypeIndirection::None &&
                                          return_type.base.is_class() &&
                                          c_type.conversion.indirection_change !=
                                          IndirectionChange::QFlagsToUInt;
          if is_stack_allocated_struct {
            allocation_place_importance = AllocationPlaceImportance::Important;
            if allocation_place == ReturnValueAllocationPlace::Stack {
              r.arguments.push(CppFfiFunctionArgument {
                name: "output".to_string(),
                argument_type: c_type,
                meaning: CppFfiArgumentMeaning::ReturnValue,
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
    if self.kind == CppMethodKind::Destructor {
      allocation_place_importance = AllocationPlaceImportance::Important;
    }
    Ok((r, allocation_place_importance))
  }

  pub fn add_c_signatures
    (&self)
     -> Result<(CppMethodWithFfiSignature, Option<CppMethodWithFfiSignature>), String> {
    match self.c_signature(ReturnValueAllocationPlace::Heap) {
      Ok((c_signature, importance)) => {
        let result1 = CppMethodWithFfiSignature {
          cpp_method: self.clone(),
          allocation_place: match importance {
            AllocationPlaceImportance::Important => ReturnValueAllocationPlace::Heap,
            AllocationPlaceImportance::NotImportant => ReturnValueAllocationPlace::NotApplicable,
          },
          c_signature: c_signature,
        };
        match importance {
          AllocationPlaceImportance::Important => {
            match self.c_signature(ReturnValueAllocationPlace::Stack) {
              Ok((c_signature2, _)) => {
                Ok((result1,
                    Some(CppMethodWithFfiSignature {
                  cpp_method: self.clone(),
                  allocation_place: ReturnValueAllocationPlace::Stack,
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
    match self.kind {
      CppMethodKind::Constructor => s = format!("{} [constructor]", s),
      CppMethodKind::Destructor => s = format!("{} [destructor]", s),
      CppMethodKind::Operator(ref op) => s = format!("{} [{:?}]", s, op),
      CppMethodKind::Regular => {}
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
    s = format!("{}({})",
                s,
                self.arguments
                    .iter()
                    .map(|arg| {
                      format!("{} {}{}",
                              arg.argument_type.to_cpp_code().unwrap_or("[?]".to_string()),
                              arg.name,
                              if arg.has_default_value {
                                format!(" = ?")
                              } else {
                                String::new()
                              })
                    })
                    .join(", "));
    if self.is_pure_virtual {
      s = format!("{} = 0", s);
    }
    if self.is_const {
      s = format!("{} const", s);
    }
    s.trim().to_string()
  }
}
