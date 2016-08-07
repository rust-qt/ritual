use cpp_method::{CppMethod, CppMethodScope, ReturnValueAllocationPlace, CppMethodKind};
use cpp_ffi_function_signature::CppFfiFunctionSignature;
use caption_strategy::{MethodCaptionStrategy, TypeCaptionStrategy};
use cpp_operators::CppOperator;

/// C++ method with arguments and return type
/// processed for FFI but no FFI function name
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppMethodWithFfiSignature {
  /// Original C++ method
  pub cpp_method: CppMethod,
  /// Allocation place method used for converting
  /// the return type of the method
  pub allocation_place: ReturnValueAllocationPlace,
  /// FFI method signature
  pub c_signature: CppFfiFunctionSignature,
}

/// Final result of converting a C++ method
/// to a FFI method
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppAndFfiMethod {
  /// Original C++ method
  pub cpp_method: CppMethod,
  /// Allocation place method used for converting
  /// the return type of the method
  pub allocation_place: ReturnValueAllocationPlace,
  /// FFI method signature
  pub c_signature: CppFfiFunctionSignature,
  /// Final name of FFI method
  pub c_name: String,
}


impl CppMethodWithFfiSignature {
  /// Generates initial FFI method name without any captions
  pub fn c_base_name(&self, include_file: &String) -> Result<String, String> {
    let scope_prefix = match self.cpp_method.scope {
      CppMethodScope::Class(ref class_name) => format!("{}_", class_name.replace("::", "_")),
      CppMethodScope::Global => format!("{}_G_", include_file),
    };
    let method_name = match self.cpp_method.kind {
      CppMethodKind::Constructor => {
        match self.allocation_place {
          ReturnValueAllocationPlace::Stack => "constructor".to_string(),
          ReturnValueAllocationPlace::Heap => "new".to_string(),
          ReturnValueAllocationPlace::NotApplicable => unreachable!(),
        }
      }
      CppMethodKind::Destructor => {
        match self.allocation_place {
          ReturnValueAllocationPlace::Stack => "destructor".to_string(),
          ReturnValueAllocationPlace::Heap => "delete".to_string(),
          ReturnValueAllocationPlace::NotApplicable => unreachable!(),
        }
      }
      CppMethodKind::Operator(ref operator) => {
        match *operator {
          CppOperator::Conversion(ref cpp_type) => {
            format!("operator_{}", cpp_type.caption(TypeCaptionStrategy::Full))
          }
          _ => format!("OP_{}", operator.c_name()),
        }
      }
      CppMethodKind::Regular => self.cpp_method.name.replace("::", "_"),

    };
    Ok(scope_prefix + &method_name)
  }

  /// Generates a caption for this method using specified strategy
  /// to avoid name conflict.
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
  /// Adds FFI method name to a CppMethodWithFfiSignature object.
  pub fn new(data: CppMethodWithFfiSignature,
             c_name: String)
             -> CppAndFfiMethod {

    CppAndFfiMethod {
      cpp_method: data.cpp_method,
      allocation_place: data.allocation_place,
      c_signature: data.c_signature,
      c_name: c_name,
    }
  }

  /// Convenience function to call CppMethod::short_text.
  pub fn short_text(&self) -> String {
    self.cpp_method.short_text()
  }
}
