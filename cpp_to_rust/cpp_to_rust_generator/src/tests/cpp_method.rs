use cpp_data::CppVisibility;
use cpp_ffi_data::CppFfiArgumentMeaning;
use cpp_ffi_data::CppTypeConversionToFfi;
use cpp_function::*;
use cpp_type::*;

#[test]
fn cpp_method_kind() {
  assert!(!CppFunctionKind::Constructor.is_destructor());
  assert!(CppFunctionKind::Constructor.is_constructor());
  assert!(!CppFunctionKind::Constructor.is_regular());

  assert!(CppFunctionKind::Destructor.is_destructor());
  assert!(!CppFunctionKind::Destructor.is_constructor());
  assert!(!CppFunctionKind::Destructor.is_regular());

  assert!(!CppFunctionKind::Regular.is_destructor());
  assert!(!CppFunctionKind::Regular.is_constructor());
  assert!(CppFunctionKind::Regular.is_regular());
}

pub fn empty_membership(class_name: &'static str) -> CppFunctionMemberData {
  CppFunctionMemberData {
    kind: CppFunctionKind::Regular,
    is_virtual: false,
    is_pure_virtual: false,
    is_const: false,
    is_static: false,
    visibility: CppVisibility::Public,
    is_signal: false,
    is_slot: false,
    class_type: CppClassType {
      name: class_name.to_string(),
      template_arguments: None,
    },
  }
}

pub fn empty_regular_method() -> CppFunction {
  CppFunction {
    name: String::new(),
    member: None,
    return_type: CppType::void(),
    arguments: vec![],
    doc: None,
    inheritance_chain: Vec::new(),
    allows_variadic_arguments: false,
    include_file: String::new(),
    origin_location: None,
    template_arguments: None,
    template_arguments_values: None,
    operator: None,
    declaration_code: None,
    is_ffi_whitelisted: false,
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
      is_const2: false,
      base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
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
      is_const2: false,
      base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  method2.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
      base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
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
      is_const2: false,
      base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  method2.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
      base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
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
      is_const2: false,
      base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  method2.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
      base: CppType::Enum {
        name: "Enum1".to_string(),
      },
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
      is_const2: false,
      base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  method2.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::Ptr,
      is_const: false,
      is_const2: false,
      base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
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
      is_const2: false,
      base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
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
  method1.return_type = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
  };
  assert!(method1.argument_types_equal(&method2));
  assert!(method2.argument_types_equal(&method1));
}

#[test]
fn c_signature_empty() {
  let mut method1 = empty_regular_method();
  method1.return_type = CppType::void();

  assert!(!method1.is_constructor());
  assert!(!method1.is_destructor());
  assert!(!method1.is_operator());
  assert_eq!(method1.class_name(), None);

  let r = method1
    .c_signature(ReturnValueAllocationPlace::NotApplicable)
    .unwrap();
  assert!(r.arguments.is_empty());
  assert!(r.return_type.ffi_type.is_void());
}

#[test]
fn c_signature_simple_func() {
  let mut method1 = empty_regular_method();
  method1.return_type = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
  };
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
      base: CppType::Enum {
        name: "Enum1".to_string(),
      },
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  let r = method1
    .c_signature(ReturnValueAllocationPlace::NotApplicable)
    .unwrap();
  assert!(r.arguments.len() == 1);
  assert_eq!(r.arguments[0].name, "arg1");
  assert_eq!(
    r.arguments[0].argument_type.ffi_type,
    method1.arguments[0].argument_type
  );
  assert_eq!(
    r.arguments[0].argument_type.conversion,
    CppTypeConversionToFfi::NoChange
  );
  assert_eq!(r.arguments[0].meaning, CppFfiArgumentMeaning::Argument(0));
  assert_eq!(r.return_type.ffi_type, method1.return_type);
  assert_eq!(r.return_type.conversion, CppTypeConversionToFfi::NoChange);
}

#[test]
fn c_signature_method_with_this() {
  let mut method1 = empty_regular_method();
  method1.member = Some(empty_membership("MyClass"));
  method1.return_type = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
  };
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
      base: CppType::Class(CppClassType {
        name: "MyClass2".to_string(),
        template_arguments: None,
      }),
    },
    name: "my_arg".to_string(),
    has_default_value: false,
  });

  assert!(!method1.is_constructor());
  assert!(!method1.is_destructor());
  assert!(!method1.is_operator());
  assert_eq!(method1.class_name(), Some(&"MyClass".to_string()));

  let r = method1
    .c_signature(ReturnValueAllocationPlace::NotApplicable)
    .unwrap();
  assert!(r.arguments.len() == 2);
  assert_eq!(r.arguments[0].name, "this_ptr");
  assert_eq!(
    r.arguments[0].argument_type.ffi_type.base,
    CppType::Class(method1.member.as_ref().unwrap().class_type.clone(),)
  );
  assert_eq!(
    r.arguments[0].argument_type.ffi_type.indirection,
    CppTypeIndirection::Ptr
  );
  assert_eq!(
    r.arguments[0].argument_type.conversion,
    CppTypeConversionToFfi::NoChange
  );
  assert_eq!(r.arguments[0].meaning, CppFfiArgumentMeaning::This);

  assert_eq!(r.arguments[1].name, "my_arg");
  assert_eq!(
    r.arguments[1].argument_type.ffi_type.base,
    method1.arguments[0].argument_type.base
  );
  assert_eq!(
    r.arguments[1].argument_type.ffi_type.indirection,
    CppTypeIndirection::Ptr
  );
  assert_eq!(
    r.arguments[1].argument_type.conversion,
    CppTypeConversionToFfi::ValueToPointer
  );
  assert_eq!(r.arguments[1].meaning, CppFfiArgumentMeaning::Argument(0));
  assert_eq!(r.return_type.ffi_type, method1.return_type);
}

#[test]
fn c_signature_static_method() {
  let mut method1 = empty_regular_method();
  method1.member = Some({
    let mut info = empty_membership("MyClass");
    info.is_static = true;
    info
  });
  method1.return_type = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
  };
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
      base: CppType::Enum {
        name: "Enum1".to_string(),
      },
    },
    name: "arg1".to_string(),
    has_default_value: false,
  });
  let r = method1
    .c_signature(ReturnValueAllocationPlace::NotApplicable)
    .unwrap();
  assert!(r.arguments.len() == 1);
  assert_eq!(r.arguments[0].name, "arg1");
  assert_eq!(
    r.arguments[0].argument_type.ffi_type,
    method1.arguments[0].argument_type
  );
  assert_eq!(
    r.arguments[0].argument_type.conversion,
    CppTypeConversionToFfi::NoChange
  );
  assert_eq!(r.arguments[0].meaning, CppFfiArgumentMeaning::Argument(0));
  assert_eq!(r.return_type.ffi_type, method1.return_type);
}

#[test]
fn c_signature_constructor() {
  let mut method1 = empty_regular_method();
  method1.member = Some({
    let mut info = empty_membership("MyClass");
    info.kind = CppFunctionKind::Constructor;
    info
  });
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::Ref,
      is_const: true,
      is_const2: false,
      base: CppType::Enum {
        name: "Enum1".to_string(),
      },
    },
    name: "arg1".to_string(),
    has_default_value: true,
  });

  assert!(method1.is_constructor());
  assert!(!method1.is_destructor());
  assert!(!method1.is_operator());
  assert_eq!(method1.class_name(), Some(&"MyClass".to_string()));

  let r_stack = method1
    .c_signature(ReturnValueAllocationPlace::Stack)
    .unwrap();
  assert!(r_stack.arguments.len() == 2);
  assert_eq!(r_stack.arguments[0].name, "arg1");
  assert_eq!(
    r_stack.arguments[0].argument_type.ffi_type,
    CppType {
      indirection: CppTypeIndirection::Ptr,
      is_const: true,
      is_const2: false,
      base: CppType::Enum {
        name: "Enum1".to_string(),
      },
    }
  );
  assert_eq!(
    r_stack.arguments[0].argument_type.conversion,
    CppTypeConversionToFfi::ReferenceToPointer
  );
  assert_eq!(
    r_stack.arguments[0].meaning,
    CppFfiArgumentMeaning::Argument(0)
  );

  assert_eq!(r_stack.arguments[1].name, "output");
  assert_eq!(
    r_stack.arguments[1].argument_type.ffi_type,
    CppType {
      indirection: CppTypeIndirection::Ptr,
      is_const: false,
      is_const2: false,
      base: CppType::Class(CppClassType {
        name: "MyClass".to_string(),
        template_arguments: None,
      }),
    }
  );
  assert_eq!(
    r_stack.arguments[1].argument_type.conversion,
    CppTypeConversionToFfi::ValueToPointer
  );
  assert_eq!(
    r_stack.arguments[1].meaning,
    CppFfiArgumentMeaning::ReturnValue
  );

  assert!(r_stack.return_type.ffi_type.is_void());

  let r_heap = method1
    .c_signature(ReturnValueAllocationPlace::Heap)
    .unwrap();
  assert!(r_heap.arguments.len() == 1);
  assert_eq!(r_heap.arguments[0].name, "arg1");
  assert_eq!(
    r_heap.arguments[0].argument_type.ffi_type,
    CppType {
      indirection: CppTypeIndirection::Ptr,
      is_const: true,
      is_const2: false,
      base: CppType::Enum {
        name: "Enum1".to_string(),
      },
    }
  );
  assert_eq!(
    r_heap.arguments[0].argument_type.conversion,
    CppTypeConversionToFfi::ReferenceToPointer
  );
  assert_eq!(
    r_heap.arguments[0].meaning,
    CppFfiArgumentMeaning::Argument(0)
  );
  assert_eq!(
    r_heap.return_type.ffi_type,
    CppType {
      indirection: CppTypeIndirection::Ptr,
      is_const: false,
      is_const2: false,
      base: CppType::Class(CppClassType {
        name: "MyClass".to_string(),
        template_arguments: None,
      }),
    }
  );
  assert_eq!(
    r_heap.return_type.conversion,
    CppTypeConversionToFfi::ValueToPointer
  );
}

#[test]
fn c_signature_destructor() {
  let mut method1 = empty_regular_method();
  method1.member = Some({
    let mut info = empty_membership("MyClass");
    info.kind = CppFunctionKind::Destructor;
    info
  });

  assert!(!method1.is_constructor());
  assert!(method1.is_destructor());
  assert!(!method1.is_operator());
  assert_eq!(method1.class_name(), Some(&"MyClass".to_string()));

  let r_stack = method1
    .c_signature(ReturnValueAllocationPlace::Stack)
    .unwrap();
  assert!(r_stack.arguments.len() == 1);
  assert_eq!(r_stack.arguments[0].name, "this_ptr");
  assert_eq!(
    &r_stack.arguments[0].argument_type.ffi_type.base,
    &CppType::Class(method1.member.as_ref().unwrap().class_type.clone(),)
  );
  assert_eq!(
    r_stack.arguments[0].argument_type.ffi_type.indirection,
    CppTypeIndirection::Ptr
  );
  assert_eq!(
    r_stack.arguments[0].argument_type.conversion,
    CppTypeConversionToFfi::NoChange
  );
  assert_eq!(r_stack.arguments[0].meaning, CppFfiArgumentMeaning::This);

  assert!(r_stack.return_type.ffi_type.is_void());

  let r_heap = method1
    .c_signature(ReturnValueAllocationPlace::Heap)
    .unwrap();
  assert!(r_heap.arguments.len() == 1);
  assert_eq!(r_heap.arguments[0].name, "this_ptr");
  assert_eq!(
    r_heap.arguments[0].argument_type.ffi_type.base,
    CppType::Class(method1.member.as_ref().unwrap().class_type.clone(),)
  );
  assert_eq!(
    r_heap.arguments[0].argument_type.ffi_type.indirection,
    CppTypeIndirection::Ptr
  );
  assert_eq!(
    r_heap.arguments[0].argument_type.conversion,
    CppTypeConversionToFfi::NoChange
  );
  assert_eq!(r_heap.arguments[0].meaning, CppFfiArgumentMeaning::This);

  assert!(r_heap.return_type.ffi_type.is_void());
}

#[test]
fn c_signature_method_returning_class() {
  let mut method1 = empty_regular_method();
  method1.member = Some(empty_membership("MyClass"));
  method1.return_type = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppType::Class(CppClassType {
      name: "MyClass3".to_string(),
      template_arguments: None,
    }),
  };
  method1.arguments.push(CppFunctionArgument {
    argument_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
      base: CppType::Class(CppClassType {
        name: "MyClass2".to_string(),
        template_arguments: None,
      }),
    },
    name: "my_arg".to_string(),
    has_default_value: false,
  });
  let r_stack = method1
    .c_signature(ReturnValueAllocationPlace::Stack)
    .unwrap();
  assert!(r_stack.arguments.len() == 3);
  assert_eq!(r_stack.arguments[0].name, "this_ptr");
  assert_eq!(
    &r_stack.arguments[0].argument_type.ffi_type.base,
    &CppType::Class(method1.member.as_ref().unwrap().class_type.clone(),)
  );
  assert_eq!(
    r_stack.arguments[0].argument_type.ffi_type.indirection,
    CppTypeIndirection::Ptr
  );
  assert_eq!(
    r_stack.arguments[0].argument_type.conversion,
    CppTypeConversionToFfi::NoChange
  );
  assert_eq!(r_stack.arguments[0].meaning, CppFfiArgumentMeaning::This);

  assert_eq!(r_stack.arguments[1].name, "my_arg");
  assert_eq!(
    r_stack.arguments[1].argument_type.ffi_type.base,
    method1.arguments[0].argument_type.base
  );
  assert_eq!(
    r_stack.arguments[1].argument_type.ffi_type.indirection,
    CppTypeIndirection::Ptr
  );
  assert_eq!(
    r_stack.arguments[1].argument_type.conversion,
    CppTypeConversionToFfi::ValueToPointer
  );
  assert_eq!(
    r_stack.arguments[1].meaning,
    CppFfiArgumentMeaning::Argument(0)
  );

  assert_eq!(r_stack.arguments[2].name, "output");
  assert_eq!(
    r_stack.arguments[2].argument_type.ffi_type,
    CppType {
      indirection: CppTypeIndirection::Ptr,
      is_const: false,
      is_const2: false,
      base: CppType::Class(CppClassType {
        name: "MyClass3".to_string(),
        template_arguments: None,
      }),
    }
  );
  assert_eq!(
    r_stack.arguments[2].argument_type.conversion,
    CppTypeConversionToFfi::ValueToPointer
  );
  assert_eq!(
    r_stack.arguments[2].meaning,
    CppFfiArgumentMeaning::ReturnValue
  );

  assert!(r_stack.return_type.ffi_type.is_void());

  let r_heap = method1
    .c_signature(ReturnValueAllocationPlace::Heap)
    .unwrap();
  assert!(r_heap.arguments.len() == 2);
  assert_eq!(r_heap.arguments[0].name, "this_ptr");
  assert_eq!(
    r_heap.arguments[0].argument_type.ffi_type.base,
    CppType::Class(method1.member.as_ref().unwrap().class_type.clone(),)
  );
  assert_eq!(
    r_heap.arguments[0].argument_type.ffi_type.indirection,
    CppTypeIndirection::Ptr
  );
  assert_eq!(
    r_heap.arguments[0].argument_type.conversion,
    CppTypeConversionToFfi::NoChange
  );
  assert_eq!(r_heap.arguments[0].meaning, CppFfiArgumentMeaning::This);

  assert_eq!(r_heap.arguments[1].name, "my_arg");
  assert_eq!(
    r_heap.arguments[1].argument_type.ffi_type.base,
    method1.arguments[0].argument_type.base
  );
  assert_eq!(
    r_heap.arguments[1].argument_type.ffi_type.indirection,
    CppTypeIndirection::Ptr
  );
  assert_eq!(
    r_heap.arguments[1].argument_type.conversion,
    CppTypeConversionToFfi::ValueToPointer
  );
  assert_eq!(
    r_heap.arguments[1].meaning,
    CppFfiArgumentMeaning::Argument(0)
  );

  assert_eq!(
    r_heap.return_type.ffi_type,
    CppType {
      indirection: CppTypeIndirection::Ptr,
      is_const: false,
      is_const2: false,
      base: CppType::Class(CppClassType {
        name: "MyClass3".to_string(),
        template_arguments: None,
      }),
    }
  );
  assert_eq!(
    r_heap.return_type.conversion,
    CppTypeConversionToFfi::ValueToPointer
  );
}

#[test]
fn full_name_free_function_in_namespace() {
  let mut method1 = empty_regular_method();
  method1.name = "ns::func1".to_string();
  assert_eq!(method1.class_name(), None);
}

#[test]
fn full_name_method() {
  let mut method1 = empty_regular_method();
  method1.name = "func1".to_string();
  method1.member = Some(empty_membership("MyClass"));
  assert_eq!(method1.class_name(), Some(&"MyClass".to_string()));
}

#[test]
fn full_name_static_method() {
  let mut method1 = empty_regular_method();
  method1.name = "func1".to_string();
  method1.member = Some({
    let mut info = empty_membership("MyClass");
    info.is_static = true;
    info
  });
  assert_eq!(method1.class_name(), Some(&"MyClass".to_string()));
}

#[test]
fn full_name_nested_class_method() {
  let mut method1 = empty_regular_method();
  method1.name = "func1".to_string();
  method1.member = Some(empty_membership("MyClass::Iterator"));
  assert_eq!(method1.class_name(), Some(&"MyClass::Iterator".to_string()));
}

#[test]
fn short_text1() {
  let method = CppFunction {
    name: "method1".to_string(),
    member: Some(CppFunctionMemberData {
      kind: CppFunctionKind::Regular,
      is_virtual: false,
      is_pure_virtual: false,
      is_const: true,
      is_static: false,
      visibility: CppVisibility::Protected,
      is_signal: false,
      is_slot: false,
      class_type: CppClassType {
        name: "Class1".to_string(),
        template_arguments: None,
      },
    }),
    operator: None,
    return_type: CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
      base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
    },
    arguments: vec![
      CppFunctionArgument {
        argument_type: CppType {
          indirection: CppTypeIndirection::None,
          is_const: false,
          is_const2: false,
          base: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
        },
        name: "arg1".to_string(),
        has_default_value: false,
      },
      CppFunctionArgument {
        argument_type: CppType {
          indirection: CppTypeIndirection::None,
          is_const: false,
          is_const2: false,
          base: CppType::BuiltInNumeric(CppBuiltInNumericType::Double),
        },
        name: "arg2".to_string(),
        has_default_value: true,
      },
    ],
    doc: None,
    inheritance_chain: Vec::new(),
    allows_variadic_arguments: false,
    include_file: String::new(),
    origin_location: None,
    template_arguments: None,
    template_arguments_values: None,
    declaration_code: None,
    is_ffi_whitelisted: false,
  };
  assert_eq!(
    method.short_text(),
    "protected int Class1::method1(int arg1, double arg2 = ?) const"
  );
}
