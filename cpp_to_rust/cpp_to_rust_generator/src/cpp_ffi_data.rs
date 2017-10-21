use caption_strategy::{ArgumentCaptionStrategy, MethodCaptionStrategy, TypeCaptionStrategy};
use cpp_method::{CppMethod, ReturnValueAllocationPlace, CppMethodArgument};
use cpp_operator::CppOperator;
use cpp_type::{CppType, CppTypeBase, CppFunctionPointerType};
use common::errors::Result;
use common::utils::MapIfOk;

/// Variation of a field accessor method
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
#[derive(Serialize, Deserialize)]
pub enum CppFieldAccessorType {
  /// Returns copy of the field
  CopyGetter,
  /// Returns const reference to the field
  ConstRefGetter,
  /// Returns mutable reference to the field
  MutRefGetter,
  /// Copies value from its argument to the field
  Setter,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppCast {
  Static {
    /// If true, this is an unsafe (from base to derived) `static_cast` wrapper.
    is_unsafe: bool,
    /// If true, this is a wrapper of `static_cast` between a class and its
    /// direct base.
    is_direct: bool,
  },
  Dynamic,
  #[allow(unused)]
  QObject,
}

impl CppCast {
  pub fn cpp_method_name(&self) -> &'static str {
    match *self {
      CppCast::Static { .. } => "static_cast",
      CppCast::Dynamic => "dynamic_cast",
      CppCast::QObject => "qobject_cast",
    }
  }

  pub fn is_unsafe_static_cast(&self) -> bool {
    match *self {
      CppCast::Static { ref is_unsafe, .. } => *is_unsafe,
      _ => false,
    }
  }
  pub fn is_direct_static_cast(&self) -> bool {
    match *self {
      CppCast::Static { ref is_direct, .. } => *is_direct,
      _ => false,
    }

  }
}

/// Information about real nature of a C++ FFI method.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppFfiMethodKind {
  /// This is a real C++ method.
  Real,
  RealWithOmittedArguments {
    /// If Some, the method is derived from another method by omitting arguments,
    /// and this field contains all arguments of the original method.
    arguments_before_omitting: Option<Vec<CppMethodArgument>>,
  },
  /// This is a field accessor, i.e. a non-existing getter or setter
  /// method for a public field.
  FieldAccessor {
    /// Type of the accessor
    accessor_type: CppFieldAccessorType,
    /// Name of the C++ field
    field_name: String,
  },
  /// This is an instance of `static_cast`, `dynamic_cast` or
  /// `qobject_cast` function call.
  Cast(CppCast),
}

/// Relation between original C++ method's argument value
/// and corresponding FFI function's argument value
#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub enum CppIndirectionChange {
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

/// Information that indicates how an FFI function argument
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
  /// Checks if this argument corresponds to an original
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
pub struct CppFfiMethodArgument {
  /// Identifier
  pub name: String,
  /// Type
  pub argument_type: CppFfiType,
  /// C++ equivalent
  pub meaning: CppFfiArgumentMeaning,
}

impl CppFfiMethodArgument {
  /// Generates part of caption string for FFI method.
  /// Used to generate FFI methods with different names
  /// for overloaded functions.
  pub fn caption(&self, strategy: ArgumentCaptionStrategy) -> Result<String> {
    Ok(match strategy {
      ArgumentCaptionStrategy::NameOnly => self.name.clone(),
      ArgumentCaptionStrategy::TypeOnly(type_strategy) => {
        self.argument_type.original_type.caption(type_strategy)?
      }
      ArgumentCaptionStrategy::TypeAndName(type_strategy) => {
        format!(
          "{}_{}",
          self.argument_type.original_type.caption(type_strategy)?,
          self.name
        )
      }
    })
  }

  /// Generates C++ code for the part of FFI function signature
  /// corresponding to this argument
  pub fn to_cpp_code(&self) -> Result<String> {
    if let CppTypeBase::FunctionPointer(..) = self.argument_type.ffi_type.base {
      Ok(self.argument_type.ffi_type.to_cpp_code(Some(&self.name))?)
    } else {
      Ok(format!(
        "{} {}",
        self.argument_type.ffi_type.to_cpp_code(None)?,
        self.name
      ))
    }
  }
}

/// Information about arguments and return type of a FFI function
/// with no final function name
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppFfiMethodSignature {
  /// List of arguments
  pub arguments: Vec<CppFfiMethodArgument>,
  /// Return type
  pub return_type: CppFfiType,
}

impl CppFfiMethodSignature {
  /// Returns true if this signature has const this_ptr argument,
  /// indicating that original C++ method has const attribute.
  /// Returns false if there is no this argument or it's not const.
  pub fn has_const_this(&self) -> bool {
    self.arguments.iter().any(|arg| {
      arg.meaning == CppFfiArgumentMeaning::This && arg.argument_type.ffi_type.is_const
    })
  }

  /// Generates arguments caption string for FFI method.
  /// Used to generate FFI methods with different names
  /// for overloaded functions.
  pub fn arguments_caption(&self, strategy: ArgumentCaptionStrategy) -> Result<String> {
    let r = self
      .arguments
      .iter()
      .filter(|x| x.meaning.is_argument())
      .map_if_ok(|arg| arg.caption(strategy.clone()))?;
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
      MethodCaptionStrategy::ArgumentsOnly(s) => self.arguments_caption(s)?,
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
        r + &self.arguments_caption(s)?
      }
    })
  }
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
  pub conversion: CppIndirectionChange,
}

impl CppFfiType {
  /// Generates an object representing the void type
  pub fn void() -> Self {
    CppFfiType {
      original_type: CppType::void(),
      ffi_type: CppType::void(),
      conversion: CppIndirectionChange::NoChange,
    }
  }
}

/// C++ method with arguments and return type
/// processed for FFI but no FFI function name
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppMethodWithFfiSignature {
  /// Original C++ method
  pub cpp_method: CppMethod,
  /// For fake C++ methods, this field describes how they were generated
  pub kind: CppFfiMethodKind,
  /// Allocation place method used for converting
  /// the return type of the method
  pub allocation_place: ReturnValueAllocationPlace,
  /// FFI method signature
  pub c_signature: CppFfiMethodSignature,
}

/// Final result of converting a C++ method
/// to a FFI method
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppAndFfiMethod {
  /// Original C++ method
  pub cpp_method: CppMethod,
  /// For fake C++ methods, this field describes how they were generated
  pub kind: CppFfiMethodKind,
  /// Allocation place method used for converting
  /// the return type of the method
  pub allocation_place: ReturnValueAllocationPlace,
  /// FFI method signature
  pub c_signature: CppFfiMethodSignature,
  /// Final name of FFI method
  pub c_name: String,
}

/// Generates initial FFI method name without any captions
pub fn c_base_name(
  cpp_method: &CppMethod,
  allocation_place: &ReturnValueAllocationPlace,
  include_file: &str,
) -> Result<String> {
  let scope_prefix = match cpp_method.class_membership {
    Some(ref info) => format!("{}_", info.class_type.caption()?),
    None => format!("{}_G_", include_file),
  };

  let add_place_note = |name| match *allocation_place {
    ReturnValueAllocationPlace::Stack => format!("{}_to_output", name),
    ReturnValueAllocationPlace::Heap => format!("{}_as_ptr", name),
    ReturnValueAllocationPlace::NotApplicable => name,
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
        format!(
          "convert_to_{}",
          cpp_type.caption(TypeCaptionStrategy::Full)?
        )
      }
      _ => format!("operator_{}", operator.c_name()?),
    })
  } else {
    add_place_note(cpp_method.name.replace("::", "_"))
  };
  let template_args_text = match cpp_method.template_arguments_values {
    Some(ref args) => {
      format!(
        "_{}",
        args
          .iter()
          .map_if_ok(|x| x.caption(TypeCaptionStrategy::Full))?
          .join("_")
      )
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
      kind: data.kind,
      allocation_place: data.allocation_place,
      c_signature: data.c_signature,
      c_name: c_name,
    }
  }

  /// Convenience function to call `CppMethod::short_text`.
  pub fn short_text(&self) -> String {
    self.cpp_method.short_text()
  }
}

/// Information about a Qt slot wrapper with
/// certain slot arguments
#[derive(Debug, Clone)]
pub struct QtSlotWrapper {
  /// Generated name of the wrapper class
  pub class_name: String,
  /// Arguments of the slot.
  pub arguments: Vec<CppFfiType>,
  /// The function pointer type accepted by this wrapper
  pub function_type: CppFunctionPointerType,
  /// String identifier passed to `QObject::connect` function to
  /// specify the object's slot.
  pub receiver_id: String,
}

/// Information about a header of the generated C++ wrapper library
#[derive(Debug, Clone)]
pub struct CppFfiHeaderData {
  /// Name of the original include file without extension
  pub include_file_base_name: String,
  /// Processed methods
  pub methods: Vec<CppAndFfiMethod>,
  /// Generated Qt slot wrappers
  pub qt_slot_wrappers: Vec<QtSlotWrapper>,
}
