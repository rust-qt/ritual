use caption_strategy::{ArgumentCaptionStrategy, MethodCaptionStrategy, TypeCaptionStrategy};
use cpp_method::{CppMethod, ReturnValueAllocationPlace};
use cpp_operator::CppOperator;
use cpp_type::{CppType, CppTypeBase};
use errors::Result;
use utils::MapIfOk;

/// Information that indicates how the FFI function argument
/// should be interpreted
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppFfiArgumentMeaning {
  /// This argument contains value for "this" pointer
  /// used to call C++ class member functions
  This,
  /// Value of this argument should be passed as an argument to
  /// the original C++ method. Associated value is index of the
  /// C++ method's argument (counting from 0).
  Argument(i8),
  /// This argument receives pointer to the buffer where
  /// the return value should be transferred to using placement new.
  ReturnValue,
}

impl CppFfiArgumentMeaning {
  /// Checks if this argument coresponds to an original
  /// C++ method's argument
  pub fn is_argument(&self) -> bool {
    match *self {
      CppFfiArgumentMeaning::Argument(..) => true,
      _ => false,
    }
  }
}

/// Representation of an argument of a FFI function
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppFfiFunctionArgument {
  /// Identifier
  pub name: String,
  /// Type
  pub argument_type: CppFfiType,
  /// C++ equivalent
  pub meaning: CppFfiArgumentMeaning,
}

impl CppFfiFunctionArgument {
  /// Generates part of caption string for FFI method.
  /// Used to generate FFI methods with different names
  /// for overloaded functions.
  pub fn caption(&self, strategy: ArgumentCaptionStrategy) -> Result<String> {
    Ok(match strategy {
      ArgumentCaptionStrategy::NameOnly => self.name.clone(),
      ArgumentCaptionStrategy::TypeOnly(type_strategy) => {
        try!(self.argument_type.original_type.caption(type_strategy))
      }
      ArgumentCaptionStrategy::TypeAndName(type_strategy) => {
        format!("{}_{}",
                try!(self.argument_type.original_type.caption(type_strategy)),
                self.name)
      }
    })
  }

  /// Generates C++ code for the part of FFI function signature
  /// corresponding to this argument
  pub fn to_cpp_code(&self) -> Result<String> {
    if let CppTypeBase::FunctionPointer { .. } = self.argument_type.ffi_type.base {
      Ok(try!(self.argument_type.ffi_type.to_cpp_code(Some(&self.name))))
    } else {
      Ok(format!("{} {}",
                 try!(self.argument_type.ffi_type.to_cpp_code(None)),
                 self.name))
    }
  }
}

/// Information about arguments and return type of a FFI function
/// with no final function name
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppFfiFunctionSignature {
  /// List of arguments
  pub arguments: Vec<CppFfiFunctionArgument>,
  /// Return type
  pub return_type: CppFfiType,
}

impl CppFfiFunctionSignature {
  /// Returns true if this signature has const this_ptr argument,
  /// indicating that original C++ method has const attribute.
  /// Returns false if there is no this argument or it's not const.
  pub fn has_const_this(&self) -> bool {
    self.arguments
      .iter()
      .any(|arg| arg.meaning == CppFfiArgumentMeaning::This && arg.argument_type.ffi_type.is_const)
  }

  /// Generates arguments caption string for FFI method.
  /// Used to generate FFI methods with different names
  /// for overloaded functions.
  pub fn arguments_caption(&self, strategy: ArgumentCaptionStrategy) -> Result<String> {
    let r = try!(self.arguments
      .iter()
      .filter(|x| x.meaning.is_argument())
      .map_if_ok(|arg| arg.caption(strategy.clone())));
    Ok(if r.is_empty() {
      "no_args".to_string()
    } else {
      r.join("_")
    })
  }

  /// Generates a caption for this method using specified strategy
  /// to avoid name conflict.
  pub fn caption(&self, strategy: MethodCaptionStrategy) -> Result<String> {
    Ok(match strategy {
      MethodCaptionStrategy::ArgumentsOnly(s) => try!(self.arguments_caption(s)),
      MethodCaptionStrategy::ConstOnly => {
        if self.has_const_this() {
          "const".to_string()
        } else {
          "".to_string()
        }
      }
      MethodCaptionStrategy::ConstAndArguments(s) => {
        let r = if self.has_const_this() {
          "const_".to_string()
        } else {
          "".to_string()
        };
        r + &try!(self.arguments_caption(s))
      }
    })
  }
}

/// Relation between original C++ method's argument value
/// and corresponding FFI function's argument value
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IndirectionChange {
  /// Argument types are identical
  NoChange,
  /// C++ argument is a class value (like QPoint)
  /// and FFI argument is a pointer (like QPoint*)
  ValueToPointer,
  /// C++ argument is a reference (like QPoint&)
  /// and FFI argument is a pointer (like QPoint*)
  ReferenceToPointer,
  /// C++ argument is QFlags<T>
  /// and FFI argument is uint
  QFlagsToUInt,
}

/// FFI function type with attached information about
/// corresponding original C++ type and their relation
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppFfiType {
  /// Original C++ type
  pub original_type: CppType,
  /// FFI function type
  pub ffi_type: CppType,
  /// Relation
  pub conversion: IndirectionChange,
}

impl CppFfiType {
  /// Generates an object representing the void type
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

/// Generates initial FFI method name without any captions
pub fn c_base_name(cpp_method: &CppMethod,
                   allocation_place: &ReturnValueAllocationPlace,
                   include_file: &str)
                   -> Result<String> {
  let scope_prefix = match cpp_method.class_membership {
    Some(ref info) => format!("{}_", try!(info.class_type.caption())),
    None => format!("{}_G_", include_file),
  };

  let add_place_note = |name| {
    match *allocation_place {
      ReturnValueAllocationPlace::Stack => format!("{}_to_output", name),
      ReturnValueAllocationPlace::Heap => format!("{}_as_ptr", name),
      ReturnValueAllocationPlace::NotApplicable => name,
    }
  };

  let method_name = if cpp_method.is_constructor() {
    match *allocation_place {
      ReturnValueAllocationPlace::Stack => "constructor".to_string(),
      ReturnValueAllocationPlace::Heap => "new".to_string(),
      ReturnValueAllocationPlace::NotApplicable => {
        return Err("NotApplicable in constructor".into());
      }
    }
  } else if cpp_method.is_destructor() {
    match *allocation_place {
      ReturnValueAllocationPlace::Stack => "destructor".to_string(),
      ReturnValueAllocationPlace::Heap => "delete".to_string(),
      ReturnValueAllocationPlace::NotApplicable => {
        return Err("NotApplicable in destructor".into());
      }
    }
  } else if let Some(ref operator) = cpp_method.operator {
    add_place_note(match *operator {
      CppOperator::Conversion(ref cpp_type) => {
        format!("convert_to_{}",
                try!(cpp_type.caption(TypeCaptionStrategy::Full)))
      }
      _ => format!("operator_{}", try!(operator.c_name())),
    })
  } else {
    add_place_note(cpp_method.name.replace("::", "_"))
  };
  let template_args_text = match cpp_method.template_arguments_values {
    Some(ref args) => {
      format!("_{}",
              try!(args.iter().map_if_ok(|x| x.caption(TypeCaptionStrategy::Full))).join("_"))
    }
    None => String::new(),
  };
  Ok(scope_prefix + &method_name + &template_args_text)
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
