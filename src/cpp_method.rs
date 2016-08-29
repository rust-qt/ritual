use cpp_type::{CppType, CppTypeIndirection, CppTypeRole, CppTypeBase};
use cpp_ffi_data::CppFfiType;
use cpp_ffi_data::CppFfiFunctionSignature;
use cpp_ffi_data::{CppFfiFunctionArgument, CppFfiArgumentMeaning};
use cpp_ffi_data::CppMethodWithFfiSignature;
use cpp_data::CppVisibility;
use utils::JoinWithString;
pub use serializable::{CppFunctionArgument, CppMethodKind, CppMethod, CppMethodClassMembership};
use cpp_operator::CppOperator;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ReturnValueAllocationPlace {
  /// the method returns a class object by value (or is a constructor), and
  /// it's translated to "output" FFI argument and placement new
  Stack,
  /// the method returns a class object by value (or is a constructor), and
  /// it's translated to pointer FFI return type and plain new
  Heap,
  /// the method does not return a class object by value, so
  /// there is only one FFI wrapper for it
  NotApplicable,
}

fn type_to_cpp_code_permissive(type1: &CppType) -> String {
  let r = match type1.base {
    CppTypeBase::TemplateParameter { ref nested_level, ref index } => {
      let mut fake_type = type1.clone();
      fake_type.base = CppTypeBase::Class {
        name: format!("T_{}_{}", nested_level, index),
        template_arguments: None,
      };
      fake_type.to_cpp_code(None)
    }
    CppTypeBase::Class { ref name, ref template_arguments } => {
      if let &Some(ref template_arguments) = template_arguments {
        Ok(format!("{}<{}>",
                   name,
                   template_arguments.iter()
                     .map(|x| type_to_cpp_code_permissive(x))
                     .join(", ")))
      } else {
        type1.to_cpp_code(None)
      }
    }
    CppTypeBase::FunctionPointer { .. } => type1.to_cpp_code(Some(&"FN_PTR".to_string())),
    _ => type1.to_cpp_code(None),
  };
  r.unwrap_or_else(|_| "[?]".to_string())
}

impl CppMethodKind {
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
  /// Checks if two methods have exactly the same set of input argument types
  pub fn argument_types_equal(&self, other: &CppMethod) -> bool {
    if self.arguments.len() != other.arguments.len() {
      return false;
    }
    if self.allows_variadic_arguments != other.allows_variadic_arguments {
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

  /// Checks if this method would need
  /// to have 2 wrappers with 2 different return value allocation places
  pub fn needs_allocation_place_variants(&self) -> bool {
    if self.is_constructor() || self.is_destructor() {
      return true;
    }
    if self.return_type.needs_allocation_place_variants() {
      return true;
    }
    false
  }

  /// Creates FFI method signature for this method:
  /// - converts all types to FFI types;
  /// - adds "this" argument explicitly if present;
  /// - adds "output" argument for return value if allocation_place is Stack.
  pub fn c_signature(&self,
                     allocation_place: ReturnValueAllocationPlace)
                     -> Result<CppFfiFunctionSignature, String> {
    if self.allows_variadic_arguments {
      return Err("Variable arguments are not supported".to_string());
    }
    let mut r = CppFfiFunctionSignature {
      arguments: Vec::new(),
      return_type: CppFfiType::void(),
    };
    if let Some(ref info) = self.class_membership {
      if !info.is_static && info.kind != CppMethodKind::Constructor {
        r.arguments.push(CppFfiFunctionArgument {
          name: "this_ptr".to_string(),
          argument_type: CppType {
              base: info.class_type.clone(),
              is_const: info.is_const,
              indirection: CppTypeIndirection::Ptr,
            }
            .to_cpp_ffi_type(CppTypeRole::NotReturnType)
            .unwrap(),
          meaning: CppFfiArgumentMeaning::This,
        });
      }
    }
    for (index, arg) in self.arguments.iter().enumerate() {
      match arg.argument_type.to_cpp_ffi_type(CppTypeRole::NotReturnType) {
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
    let real_return_type = if self.is_constructor() {
      CppType {
        is_const: false,
        indirection: CppTypeIndirection::None,
        base: self.class_membership.as_ref().unwrap().class_type.clone(),
      }
    } else {
      self.return_type.clone()
    };
    match real_return_type.to_cpp_ffi_type(CppTypeRole::ReturnType) {
      Ok(c_type) => {
        if real_return_type.needs_allocation_place_variants() {
          match allocation_place {
            ReturnValueAllocationPlace::Stack => {
              r.arguments.push(CppFfiFunctionArgument {
                name: "output".to_string(),
                argument_type: c_type,
                meaning: CppFfiArgumentMeaning::ReturnValue,
              });
            }
            ReturnValueAllocationPlace::Heap => {
              r.return_type = c_type;
            }
            ReturnValueAllocationPlace::NotApplicable => {
              panic!("NotApplicable encountered but return value needs allocation_place variants")
            }
          }
        } else {
          r.return_type = c_type;
        }
      }
      Err(msg) => {
        return Err(format!("Can't convert type to C: {:?}: {}", real_return_type, msg));
      }
    }
    Ok(r)
  }

  /// Generates either one or two FFI signatures for this method,
  /// depending on its return type.
  pub fn to_ffi_signatures(&self) -> Result<Vec<CppMethodWithFfiSignature>, String> {
    let places = if self.needs_allocation_place_variants() {
      vec![ReturnValueAllocationPlace::Heap, ReturnValueAllocationPlace::Stack]
    } else {
      vec![ReturnValueAllocationPlace::NotApplicable]
    };
    let mut results = Vec::new();
    for place in places {
      let c_signature = try!(self.c_signature(place.clone()));
      results.push(CppMethodWithFfiSignature {
        cpp_method: self.clone(),
        allocation_place: place,
        c_signature: c_signature,
      });
    }
    Ok(results)
  }

  pub fn full_name(&self) -> String {
    if let Some(ref info) = self.class_membership {
      format!("{}::{}",
              type_to_cpp_code_permissive(&CppType {
                indirection: CppTypeIndirection::None,
                is_const: false,
                base: info.class_type.clone(),
              }),
              self.name)
    } else {
      self.name.clone()
    }
  }

  /// Returns short text representing values in this method
  /// (only for debug output purposes).
  pub fn short_text(&self) -> String {


    let mut s = String::new();
    if let Some(ref info) = self.class_membership {
      if info.is_virtual {
        s = format!("{} virtual", s);
      }
      if info.is_static {
        s = format!("{} static", s);
      }
      if info.visibility == CppVisibility::Protected {
        s = format!("{} protected", s);
      }
      if info.visibility == CppVisibility::Private {
        s = format!("{} private", s);
      }
      if info.is_signal {
        s = format!("{} [signal]", s);
      }
      match info.kind {
        CppMethodKind::Constructor => s = format!("{} [constructor]", s),
        CppMethodKind::Destructor => s = format!("{} [destructor]", s),
        CppMethodKind::Regular => {}
      }
    }
    if let Some(ref op) = self.operator {
      s = format!("{} [{:?}]", s, op);
    }
    if self.allows_variadic_arguments {
      s = format!("{} [var args]", s);
    }
    s = format!("{} {}", s, type_to_cpp_code_permissive(&self.return_type));
    s = format!("{} {}", s, self.full_name());
    s = format!("{}({})",
                s,
                self.arguments
                  .iter()
                  .map(|arg| {
        format!("{} {}{}",
                type_to_cpp_code_permissive(&arg.argument_type),
                arg.name,
                if arg.has_default_value {
                  format!(" = ?")
                } else {
                  String::new()
                })
      })
                  .join(", "));
    if let Some(ref info) = self.class_membership {
      if info.is_pure_virtual {
        s = format!("{} = 0", s);
      }
      if info.is_const {
        s = format!("{} const", s);
      }
    }
    s.trim().to_string()
  }

  pub fn class_name(&self) -> Option<&String> {
    match self.class_membership {
      Some(ref info) => Some(info.class_type.maybe_name().unwrap()),
      None => None,
    }
  }

  pub fn is_constructor(&self) -> bool {
    match self.class_membership {
      Some(ref info) => info.kind.is_constructor(),
      None => false,
    }
  }
  pub fn is_destructor(&self) -> bool {
    match self.class_membership {
      Some(ref info) => info.kind.is_destructor(),
      None => false,
    }
  }
  pub fn is_operator(&self) -> bool {
    self.operator.is_some()
  }

  pub fn all_involved_types(&self) -> Vec<CppType> {
    let mut result: Vec<CppType> = Vec::new();
    if let Some(ref class_membership) = self.class_membership {
      result.push(CppType {
        base: class_membership.class_type.clone(),
        is_const: class_membership.is_const,
        indirection: CppTypeIndirection::Ptr,
      });
    }
    for t in self.arguments.iter().map(|x| x.argument_type.clone()) {
      result.push(t);
    }
    result.push(self.return_type.clone());
    if let Some(ref operator) = self.operator {
      if let &CppOperator::Conversion(ref cpp_type) = operator {
        result.push(cpp_type.clone());
      }
    }
    result
  }
}
