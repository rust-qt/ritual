use cpp_method::CppMethod;
use enums::{AllocationPlace, CppMethodScope};
use cpp_ffi_function_signature::CppFfiFunctionSignature;
use utils::operator_c_name;
use caption_strategy::{MethodCaptionStrategy, TypeCaptionStrategy};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppMethodWithFfiSignature {
  pub cpp_method: CppMethod,
  pub allocation_place: AllocationPlace,
  pub c_signature: CppFfiFunctionSignature,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppAndFfiMethod {
  pub cpp_method: CppMethod,
  pub allocation_place: AllocationPlace,
  pub c_signature: CppFfiFunctionSignature,
  pub c_name: String,
}


impl CppMethodWithFfiSignature {
  pub fn c_base_name(&self, include_file: &String) -> Result<String, String> {
    let scope_prefix = match self.cpp_method.scope {
      CppMethodScope::Class(ref class_name) => format!("{}_", class_name.replace("::", "_")),
      CppMethodScope::Global => format!("{}_G_", include_file),
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
      match operator_c_name(operator, self.cpp_method.real_arguments_count()) {
        Ok(op) => format!("OP_{}", op),
        Err(msg) => return Err(msg),
      }
    } else if let Some(ref operator_type) = self.cpp_method.conversion_operator {
      //TODO: support conversion operators in rust
      format!("operator_{}", operator_type.caption(TypeCaptionStrategy::Full))
    } else {
      self.cpp_method.name.clone() //.replace("operator ", "operator_")
    };
    Ok(scope_prefix + &method_name)
  }

  pub fn caption(&self, strategy: MethodCaptionStrategy) -> String {
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

impl CppAndFfiMethod {
  pub fn new(data: CppMethodWithFfiSignature, c_name: String) -> CppAndFfiMethod {
    CppAndFfiMethod {
      cpp_method: data.cpp_method,
      allocation_place: data.allocation_place,
      c_signature: data.c_signature,
      c_name: c_name,
    }
  }

  pub fn short_text(&self) -> String {
    self.cpp_method.short_text()
  }
}
