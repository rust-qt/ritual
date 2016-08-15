use cpp_method::*;
use cpp_operators::CppOperator;
use cpp_data::CppVisibility;
use cpp_type::*;
use cpp_ffi_function_argument::CppFfiArgumentMeaning;
use cpp_ffi_type::IndirectionChange;

#[test]
fn cpp_method_scope() {
  let a1 = CppMethodScope::Global;
  assert!(a1.class_name().is_none());

  let a2 = CppMethodScope::Class("Class1".to_string());
  assert_eq!(a2.class_name(), Some(&"Class1".to_string()));
}

#[test]
fn cpp_method_kind() {
  assert!(CppMethodKind::Operator(CppOperator::Assignment).is_operator());
  assert!(!CppMethodKind::Operator(CppOperator::Assignment).is_destructor());
  assert!(!CppMethodKind::Operator(CppOperator::Assignment).is_constructor());
  assert!(!CppMethodKind::Operator(CppOperator::Assignment).is_regular());

  assert!(!CppMethodKind::Constructor.is_operator());
  assert!(!CppMethodKind::Constructor.is_destructor());
  assert!(CppMethodKind::Constructor.is_constructor());
  assert!(!CppMethodKind::Constructor.is_regular());

  assert!(!CppMethodKind::Destructor.is_operator());
  assert!(CppMethodKind::Destructor.is_destructor());
  assert!(!CppMethodKind::Destructor.is_constructor());
  assert!(!CppMethodKind::Destructor.is_regular());

  assert!(!CppMethodKind::Regular.is_operator());
  assert!(!CppMethodKind::Regular.is_destructor());
  assert!(!CppMethodKind::Regular.is_constructor());
  assert!(CppMethodKind::Regular.is_regular());
}

fn empty_regular_method() -> CppMethod {
  CppMethod {
    name: String::new(),
    scope: CppMethodScope::Global,
    is_virtual: false,
    is_pure_virtual: false,
    is_const: false,
    is_static: false,
    visibility: CppVisibility::Public,
    is_signal: false,
    return_type: None,
    class_type: None,
    kind: CppMethodKind::Regular,
    arguments: vec![],
    allows_variable_arguments: false,
    include_file: String::new(),
    origin_location: None,
    template_arguments: None,
  }
}

#[test]
fn argument_types_equal1() {
  let method1 = empty_regular_method();
  let method2 = empty_regular_method();
  assert!(method1.argument_types_equal(&method2));
  assert!(method2.argument_types_equal(&method1));
}

#[test]
fn argument_types_equal2() {
  let mut method1 = empty_regular_method();
  let method2 = empty_regular_method();
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  assert!(!method1.argument_types_equal(&method2));
  assert!(!method2.argument_types_equal(&method1));
}

#[test]
fn argument_types_equal3() {
  let mut method1 = empty_regular_method();
  let mut method2 = empty_regular_method();
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  method2.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "x".to_string(),
    has_default_value: false,
  });
  assert!(method1.argument_types_equal(&method2));
  assert!(method2.argument_types_equal(&method1));
}

#[test]
fn argument_types_equal4() {
  let mut method1 = empty_regular_method();
  let mut method2 = empty_regular_method();
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  method2.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "arg1".to_string(),
    has_default_value: true,
  });
  assert!(method1.argument_types_equal(&method2));
  assert!(method2.argument_types_equal(&method1));
}

#[test]
fn argument_types_equal5() {
  let mut method1 = empty_regular_method();
  let mut method2 = empty_regular_method();
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  method2.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      base: CppTypeBase::Enum { name: "Enum1".to_string() },
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  assert!(!method1.argument_types_equal(&method2));
  assert!(!method2.argument_types_equal(&method1));
}

#[test]
fn argument_types_equal6() {
  let mut method1 = empty_regular_method();
  let mut method2 = empty_regular_method();
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  method2.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::Ptr,
      is_const: false,
      base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  assert!(!method1.argument_types_equal(&method2));
  assert!(!method2.argument_types_equal(&method1));
}

#[test]
fn argument_types_equal7() {
  let mut method1 = empty_regular_method();
  let int = CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "arg1".to_string(),
    has_default_value: false,
  };
  let mut method2 = empty_regular_method();
  method1.arguments.push(int.clone());
  method2.arguments.push(int.clone());
  method2.arguments.push(int.clone());
  assert!(!method1.argument_types_equal(&method2));
  assert!(!method2.argument_types_equal(&method1));
}

#[test]
fn argument_types_equal8() {
  let mut method1 = empty_regular_method();
  let method2 = empty_regular_method();
  method1.return_type = Some(CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
  });
  assert!(method1.argument_types_equal(&method2));
  assert!(method2.argument_types_equal(&method1));
}

#[test]
fn needs_allocation_place_variants() {
  let mut method1 = empty_regular_method();
  assert!(!method1.needs_allocation_place_variants());
  method1.kind = CppMethodKind::Constructor;
  assert!(method1.needs_allocation_place_variants());
  method1.kind = CppMethodKind::Destructor;
  assert!(method1.needs_allocation_place_variants());
  method1.kind = CppMethodKind::Operator(CppOperator::Assignment);
  assert!(!method1.needs_allocation_place_variants());
  method1.return_type = Some(CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
  });
  assert!(!method1.needs_allocation_place_variants());
  method1.return_type = Some(CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    base: CppTypeBase::Class {
      name: "QRect".to_string(),
      template_arguments: None,
    },
  });
  assert!(method1.needs_allocation_place_variants());
  method1.return_type = Some(CppType {
    indirection: CppTypeIndirection::Ptr,
    is_const: false,
    base: CppTypeBase::Class {
      name: "QRect".to_string(),
      template_arguments: None,
    },
  });
  assert!(!method1.needs_allocation_place_variants());
  method1.kind = CppMethodKind::Regular;
  method1.return_type = None;
  assert!(!method1.needs_allocation_place_variants());
}

#[test]
fn c_signature_empty() {
  let mut method1 = empty_regular_method();
  method1.return_type = Some(CppType::void());
  let r = method1.c_signature(ReturnValueAllocationPlace::NotApplicable).unwrap();
  assert!(r.arguments.is_empty());
  assert!(r.return_type.ffi_type.is_void());
}

#[test]
fn c_signature_simple_func() {
  let mut method1 = empty_regular_method();
  method1.return_type = Some(CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
  });
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      base: CppTypeBase::Enum { name: "Enum1".to_string() },
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  let r = method1.c_signature(ReturnValueAllocationPlace::NotApplicable).unwrap();
  assert!(r.arguments.len() == 1);
  assert_eq!(r.arguments[0].name, "arg1");
  assert_eq!(r.arguments[0].argument_type.ffi_type,
             method1.arguments[0].argument_type);
  assert_eq!(r.arguments[0].argument_type.conversion,
             IndirectionChange::NoChange);
  assert_eq!(r.arguments[0].meaning, CppFfiArgumentMeaning::Argument(0));
  assert_eq!(r.return_type.ffi_type, method1.return_type.unwrap());
  assert_eq!(r.return_type.conversion, IndirectionChange::NoChange);
}

#[test]
fn c_signature_method_with_this() {
  let mut method1 = empty_regular_method();
  method1.scope = CppMethodScope::Class("MyClass".to_string());
  method1.class_type = Some(CppTypeBase::Class {
    name: "MyClass".to_string(),
    template_arguments: None,
  });
  method1.return_type = Some(CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
  });
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      base: CppTypeBase::Class {
        name: "MyClass2".to_string(),
        template_arguments: None,
      },
    },
    name: "my_arg".to_string(),
    has_default_value: false,
  });
  let r = method1.c_signature(ReturnValueAllocationPlace::NotApplicable).unwrap();
  assert!(r.arguments.len() == 2);
  assert_eq!(r.arguments[0].name, "this_ptr");
  assert_eq!(r.arguments[0].argument_type.ffi_type.base,
             method1.class_type.unwrap());
  assert_eq!(r.arguments[0].argument_type.ffi_type.indirection,
             CppTypeIndirection::Ptr);
  assert_eq!(r.arguments[0].argument_type.conversion,
             IndirectionChange::NoChange);
  assert_eq!(r.arguments[0].meaning, CppFfiArgumentMeaning::This);

  assert_eq!(r.arguments[1].name, "my_arg");
  assert_eq!(r.arguments[1].argument_type.ffi_type.base,
             method1.arguments[0].argument_type.base);
  assert_eq!(r.arguments[1].argument_type.ffi_type.indirection,
             CppTypeIndirection::Ptr);
  assert_eq!(r.arguments[1].argument_type.conversion,
             IndirectionChange::ValueToPointer);
  assert_eq!(r.arguments[1].meaning, CppFfiArgumentMeaning::Argument(0));
  assert_eq!(r.return_type.ffi_type, method1.return_type.unwrap());
}

#[test]
fn c_signature_static_method() {
  let mut method1 = empty_regular_method();
  method1.scope = CppMethodScope::Class("MyClass".to_string());
  method1.class_type = Some(CppTypeBase::Class {
    name: "MyClass".to_string(),
    template_arguments: None,
  });
  method1.is_static = true;
  method1.return_type = Some(CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
  });
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      base: CppTypeBase::Enum { name: "Enum1".to_string() },
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  let r = method1.c_signature(ReturnValueAllocationPlace::NotApplicable).unwrap();
  assert!(r.arguments.len() == 1);
  assert_eq!(r.arguments[0].name, "arg1");
  assert_eq!(r.arguments[0].argument_type.ffi_type,
             method1.arguments[0].argument_type);
  assert_eq!(r.arguments[0].argument_type.conversion,
             IndirectionChange::NoChange);
  assert_eq!(r.arguments[0].meaning, CppFfiArgumentMeaning::Argument(0));
  assert_eq!(r.return_type.ffi_type, method1.return_type.unwrap());
}


#[test]
fn c_signature_constructor() {
  let mut method1 = empty_regular_method();
  method1.kind = CppMethodKind::Constructor;
  method1.scope = CppMethodScope::Class("MyClass".to_string());
  method1.class_type = Some(CppTypeBase::Class {
    name: "MyClass".to_string(),
    template_arguments: None,
  });
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::Ref,
      is_const: true,
      base: CppTypeBase::Enum { name: "Enum1".to_string() },
    },
    name: "arg1".to_string(),
    has_default_value: true,
  });
  let r_stack = method1.c_signature(ReturnValueAllocationPlace::Stack).unwrap();
  assert!(r_stack.arguments.len() == 2);
  assert_eq!(r_stack.arguments[0].name, "arg1");
  assert_eq!(r_stack.arguments[0].argument_type.ffi_type,
             CppType {
               indirection: CppTypeIndirection::Ptr,
               is_const: true,
               base: CppTypeBase::Enum { name: "Enum1".to_string() },
             });
  assert_eq!(r_stack.arguments[0].argument_type.conversion,
             IndirectionChange::ReferenceToPointer);
  assert_eq!(r_stack.arguments[0].meaning, CppFfiArgumentMeaning::Argument(0));

  assert_eq!(r_stack.arguments[1].name, "output");
  assert_eq!(r_stack.arguments[1].argument_type.ffi_type,
             CppType {
               indirection: CppTypeIndirection::Ptr,
               is_const: false,
               base: CppTypeBase::Class {
                 name: "MyClass".to_string(),
                 template_arguments: None,
               },
             });
  assert_eq!(r_stack.arguments[1].argument_type.conversion,
             IndirectionChange::ValueToPointer);
  assert_eq!(r_stack.arguments[1].meaning, CppFfiArgumentMeaning::ReturnValue);

  assert!(r_stack.return_type.ffi_type.is_void());

  let r_heap = method1.c_signature(ReturnValueAllocationPlace::Heap).unwrap();
  assert!(r_heap.arguments.len() == 1);
  assert_eq!(r_heap.arguments[0].name, "arg1");
  assert_eq!(r_heap.arguments[0].argument_type.ffi_type,
             CppType {
               indirection: CppTypeIndirection::Ptr,
               is_const: true,
               base: CppTypeBase::Enum { name: "Enum1".to_string() },
             });
  assert_eq!(r_heap.arguments[0].argument_type.conversion,
             IndirectionChange::ReferenceToPointer);
  assert_eq!(r_heap.arguments[0].meaning, CppFfiArgumentMeaning::Argument(0));
  assert_eq!(r_heap.return_type.ffi_type,
             CppType {
               indirection: CppTypeIndirection::Ptr,
               is_const: false,
               base: CppTypeBase::Class {
                 name: "MyClass".to_string(),
                 template_arguments: None,
               },
             });
  assert_eq!(r_heap.return_type.conversion,
             IndirectionChange::ValueToPointer);
}

// TODO: add tests for:
// - destructor
// - class method returning a class value
// - class method with a class value argument but int return type
