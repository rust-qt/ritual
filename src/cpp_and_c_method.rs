use cpp_method::CppMethod;
use enums::{AllocationPlace, CppMethodScope};
use c_function_signature::CFunctionSignature;
use utils::operator_c_name;
use caption_strategy::MethodCaptionStrategy;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppMethodWithCSignature {
  pub cpp_method: CppMethod,
  pub allocation_place: AllocationPlace,
  pub c_signature: CFunctionSignature,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppAndCMethod {
  pub cpp_method: CppMethod,
  pub allocation_place: AllocationPlace,
  pub c_signature: CFunctionSignature,
  pub c_name: String,
}


impl CppMethodWithCSignature {

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

impl CppAndCMethod {
  fn new(data: CppMethodWithCSignature, c_name: String) -> CppAndCMethod {
    CppAndCMethod {
      cpp_method: data.cpp_method,
      allocation_place: data.allocation_place,
      c_signature: data.c_signature,
      c_name: c_name,
    }
  }
}
