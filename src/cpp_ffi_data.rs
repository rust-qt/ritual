use caption_strategy::ArgumentCaptionStrategy;
use utils::JoinWithString;
use cpp_type::CppTypeBase;
use cpp_type::CppType;
use cpp_method::{CppMethod, ReturnValueAllocationPlace};
use caption_strategy::{MethodCaptionStrategy, TypeCaptionStrategy};
use cpp_operator::CppOperator;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppFfiArgumentMeaning {
  This,
  Argument(i8),
  ReturnValue,
}

impl CppFfiArgumentMeaning {
  pub fn is_argument(&self) -> bool {
    match self {
      &CppFfiArgumentMeaning::Argument(..) => true,
      _ => false,
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppFfiFunctionArgument {
  pub name: String,
  pub argument_type: CppFfiType,
  pub meaning: CppFfiArgumentMeaning,
}

impl CppFfiFunctionArgument {
  pub fn caption(&self, strategy: ArgumentCaptionStrategy) -> String {
    match strategy {
      ArgumentCaptionStrategy::NameOnly => self.name.clone(),
      ArgumentCaptionStrategy::TypeOnly(type_strategy) => {
        self.argument_type.original_type.caption(type_strategy)
      }
      ArgumentCaptionStrategy::TypeAndName(type_strategy) => {
        format!("{}_{}",
                self.argument_type.original_type.caption(type_strategy),
                self.name)
      }
    }
  }

  pub fn to_cpp_code(&self) -> Result<String, String> {
    match self.argument_type.ffi_type.base {
      CppTypeBase::FunctionPointer { .. } => {
        Ok(try!(self.argument_type.ffi_type.to_cpp_code(Some(&self.name))))
      }
      _ => {
        Ok(format!("{} {}",
                   try!(self.argument_type.ffi_type.to_cpp_code(None)),
                   self.name))
      }
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppFfiFunctionSignature {
  pub arguments: Vec<CppFfiFunctionArgument>,
  pub return_type: CppFfiType,
}

impl CppFfiFunctionSignature {
  pub fn caption(&self, strategy: ArgumentCaptionStrategy) -> String {
    let r = self.arguments
      .iter()
      .filter(|x| x.meaning.is_argument())
      .map(|x| x.caption(strategy.clone()))
      .join("_");
    if r.len() == 0 {
      "no_args".to_string()
    } else {
      r
    }
  }

  pub fn arguments_to_cpp_code(&self) -> Result<String, String> {
    let mut code = Vec::new();
    for arg in &self.arguments {
      match arg.to_cpp_code() {
        Ok(c) => code.push(c),
        Err(msg) => return Err(msg),
      }
    }
    Ok(code.join(", "))
  }
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IndirectionChange {
  NoChange,
  ValueToPointer,
  ReferenceToPointer,
  QFlagsToUInt,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppFfiType {
  pub original_type: CppType,
  pub ffi_type: CppType,
  pub conversion: IndirectionChange,
}

impl CppFfiType {
  pub fn void() -> Self {
    CppFfiType {
      original_type: CppType::void(),
      ffi_type: CppType::void(),
      conversion: IndirectionChange::NoChange,
    }
  }
}


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
    let scope_prefix = match self.cpp_method.class_membership {
      Some(ref info) => format!("{}_", info.class_type.caption()),
      None => format!("{}_G_", include_file),
    };

    let add_place_note = |name| {
      match self.allocation_place {
        ReturnValueAllocationPlace::Stack => format!("{}_to_output", name),
        ReturnValueAllocationPlace::Heap => format!("{}_as_ptr", name),
        ReturnValueAllocationPlace::NotApplicable => name,
      }
    };

    let method_name = if self.cpp_method.is_constructor() {
      match self.allocation_place {
        ReturnValueAllocationPlace::Stack => "constructor".to_string(),
        ReturnValueAllocationPlace::Heap => "new".to_string(),
        ReturnValueAllocationPlace::NotApplicable => unreachable!(),
      }
    } else if self.cpp_method.is_destructor() {
      match self.allocation_place {
        ReturnValueAllocationPlace::Stack => "destructor".to_string(),
        ReturnValueAllocationPlace::Heap => "delete".to_string(),
        ReturnValueAllocationPlace::NotApplicable => unreachable!(),
      }
    } else if let Some(ref operator) = self.cpp_method.operator {
      add_place_note(match *operator {
        CppOperator::Conversion(ref cpp_type) => {
          format!("operator_{}", cpp_type.caption(TypeCaptionStrategy::Full))
        }
        _ => format!("OP_{}", operator.c_name()),
      })
    } else {
      add_place_note(self.cpp_method.name.replace("::", "_"))
    };
    Ok(scope_prefix + &method_name)
  }

  /// Generates a caption for this method using specified strategy
  /// to avoid name conflict.
  pub fn caption(&self, strategy: MethodCaptionStrategy) -> String {
    match strategy {
      MethodCaptionStrategy::ArgumentsOnly(s) => self.c_signature.caption(s),
      MethodCaptionStrategy::ConstOnly => {
        if self.cpp_method.class_membership.as_ref().map(|x| x.is_const).unwrap_or(false) {
          "const".to_string()
        } else {
          "".to_string()
        }
      }
      MethodCaptionStrategy::ConstAndArguments(s) => {
        let r = if self.cpp_method.class_membership.as_ref().map(|x| x.is_const).unwrap_or(false) {
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
  pub fn new(data: CppMethodWithFfiSignature, c_name: String) -> CppAndFfiMethod {
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
