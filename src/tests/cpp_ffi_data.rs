use cpp_ffi_data::*;
use cpp_type::*;
use caption_strategy::*;
use tests::cpp_method::{empty_regular_method, empty_membership};
use cpp_method::{CppMethodKind, ReturnValueAllocationPlace};
use cpp_operator::CppOperator;

#[test]
fn argument_meaning() {
  let a1 = CppFfiArgumentMeaning::This;
  assert!(!a1.is_argument());

  let a2 = CppFfiArgumentMeaning::Argument(2);
  assert!(a2.is_argument());

  let a3 = CppFfiArgumentMeaning::ReturnValue;
  assert!(!a3.is_argument());
}

#[test]
fn argument_int() {
  let arg = CppFfiFunctionArgument {
    name: "arg1".to_string(),
    argument_type: CppFfiType {
      original_type: CppType {
        indirection: CppTypeIndirection::None,
        is_const: false,
        is_const2: false,
        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
      },
      ffi_type: CppType {
        indirection: CppTypeIndirection::None,
        is_const: false,
        is_const2: false,
        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
      },
      conversion: IndirectionChange::NoChange,
    },
    meaning: CppFfiArgumentMeaning::Argument(0),
  };
  assert_eq!(arg.caption(ArgumentCaptionStrategy::NameOnly), "arg1");
  assert_eq!(arg.caption(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Short)),
             "int");
  assert_eq!(arg.caption(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Full)),
             "int");
  assert_eq!(arg.caption(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Short)),
             "int_arg1");
  assert_eq!(arg.caption(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Full)),
             "int_arg1");
  assert_eq!(arg.to_cpp_code().unwrap(), "int arg1");
}

#[test]
fn argument_int_ptr() {
  let arg = CppFfiFunctionArgument {
    name: "arg1".to_string(),
    argument_type: CppFfiType {
      original_type: CppType {
        indirection: CppTypeIndirection::Ptr,
        is_const: false,
        is_const2: false,
        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
      },
      ffi_type: CppType {
        indirection: CppTypeIndirection::Ptr,
        is_const: false,
        is_const2: false,
        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
      },
      conversion: IndirectionChange::NoChange,
    },
    meaning: CppFfiArgumentMeaning::Argument(0),
  };
  assert_eq!(arg.caption(ArgumentCaptionStrategy::NameOnly), "arg1");
  assert_eq!(arg.caption(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Short)),
             "int");
  assert_eq!(arg.caption(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Full)),
             "int_ptr");
  assert_eq!(arg.caption(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Short)),
             "int_arg1");
  assert_eq!(arg.caption(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Full)),
             "int_ptr_arg1");
  assert_eq!(arg.to_cpp_code().unwrap(), "int* arg1");
}

#[test]
fn argument_func() {
  let type1 = CppType {
    is_const: false,
    is_const2: false,
    indirection: CppTypeIndirection::None,
    base: CppTypeBase::FunctionPointer {
      allows_variadic_arguments: false,
      return_type: Box::new(CppType {
        indirection: CppTypeIndirection::None,
        is_const: false,
        is_const2: false,
        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
      }),
      arguments: vec![CppType {
                        indirection: CppTypeIndirection::None,
                        is_const: false,
                        is_const2: false,
                        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
                      },
                      CppType {
                        indirection: CppTypeIndirection::Ptr,
                        is_const: false,
                        is_const2: false,
                        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
                      }],
    },
  };

  let arg = CppFfiFunctionArgument {
    name: "arg1".to_string(),
    argument_type: CppFfiType {
      original_type: type1.clone(),
      ffi_type: type1.clone(),
      conversion: IndirectionChange::NoChange,
    },
    meaning: CppFfiArgumentMeaning::Argument(0),
  };
  assert_eq!(arg.caption(ArgumentCaptionStrategy::NameOnly), "arg1");
  assert_eq!(arg.caption(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Short)),
             "func");
  assert_eq!(arg.caption(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Full)),
             "int_func_int_bool_ptr");
  assert_eq!(arg.caption(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Short)),
             "func_arg1");
  assert_eq!(arg.caption(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Full)),
             "int_func_int_bool_ptr_arg1");
  assert_eq!(arg.to_cpp_code().unwrap(), "int (*arg1)(int, bool*)");
}

#[test]
fn signature_two_numbers() {
  let sig = CppFfiFunctionSignature {
    arguments: vec![CppFfiFunctionArgument {
                      name: "arg1".to_string(),
                      argument_type: CppFfiType {
                        original_type: CppType {
                          indirection: CppTypeIndirection::None,
                          is_const: false,
                          is_const2: false,
                          base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
                        },
                        ffi_type: CppType {
                          indirection: CppTypeIndirection::None,
                          is_const: false,
                          is_const2: false,
                          base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
                        },
                        conversion: IndirectionChange::NoChange,
                      },
                      meaning: CppFfiArgumentMeaning::Argument(0),
                    },
                    CppFfiFunctionArgument {
                      name: "arg2".to_string(),
                      argument_type: CppFfiType {
                        original_type: CppType {
                          indirection: CppTypeIndirection::None,
                          is_const: false,
                          is_const2: false,
                          base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Double),
                        },
                        ffi_type: CppType {
                          indirection: CppTypeIndirection::None,
                          is_const: false,
                          is_const2: false,
                          base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Double),
                        },
                        conversion: IndirectionChange::NoChange,
                      },
                      meaning: CppFfiArgumentMeaning::Argument(0),
                    }],
    return_type: CppFfiType::void(),
  };

  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::NameOnly),
             "arg1_arg2");
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Short)),
             "int_double");
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Full)),
             "int_double");
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Short)),
             "int_arg1_double_arg2");
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Full)),
             "int_arg1_double_arg2");

  assert_eq!(sig.caption(MethodCaptionStrategy::ArgumentsOnly(ArgumentCaptionStrategy::NameOnly)),
             "arg1_arg2");
  assert_eq!(sig.caption(MethodCaptionStrategy::ArgumentsOnly(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Short))),
             "int_double");
  assert_eq!(sig.caption(MethodCaptionStrategy::ArgumentsOnly(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Full))),
             "int_double");
  assert_eq!(sig.caption(MethodCaptionStrategy::ArgumentsOnly(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Short))),
             "int_arg1_double_arg2");
  assert_eq!(sig.caption(MethodCaptionStrategy::ArgumentsOnly(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Full))),
             "int_arg1_double_arg2");

  assert_eq!(sig.caption(MethodCaptionStrategy::ConstOnly), "");

  assert_eq!(sig.arguments_to_cpp_code().unwrap(),
             "int arg1, double arg2");

  assert!(!sig.has_const_this());
}

#[test]
fn signature_class_method() {
  let sig = CppFfiFunctionSignature {
    arguments: vec![CppFfiFunctionArgument {
                      name: "this_ptr".to_string(),
                      argument_type: CppFfiType {
                        original_type: CppType {
                          indirection: CppTypeIndirection::Ptr,
                          is_const: false,
                          is_const2: false,
                          base: CppTypeBase::Class(CppTypeClassBase {
                            name: "Class1".to_string(),
                            template_arguments: None,
                          }),
                        },
                        ffi_type: CppType {
                          indirection: CppTypeIndirection::Ptr,
                          is_const: false,
                          is_const2: false,
                          base: CppTypeBase::Class(CppTypeClassBase {
                            name: "Class1".to_string(),
                            template_arguments: None,
                          }),
                        },
                        conversion: IndirectionChange::NoChange,
                      },
                      meaning: CppFfiArgumentMeaning::This,
                    },
                    CppFfiFunctionArgument {
                      name: "arg1".to_string(),
                      argument_type: CppFfiType {
                        original_type: CppType {
                          indirection: CppTypeIndirection::None,
                          is_const: false,
                          is_const2: false,
                          base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Double),
                        },
                        ffi_type: CppType {
                          indirection: CppTypeIndirection::None,
                          is_const: false,
                          is_const2: false,
                          base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Double),
                        },
                        conversion: IndirectionChange::NoChange,
                      },
                      meaning: CppFfiArgumentMeaning::Argument(0),
                    }],
    return_type: CppFfiType::void(),
  };
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::NameOnly),
             "arg1");
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Short)),
             "double");
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Full)),
             "double");
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Short)),
             "double_arg1");
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Full)),
             "double_arg1");

  assert_eq!(sig.caption(MethodCaptionStrategy::ConstOnly), "");

  assert_eq!(sig.arguments_to_cpp_code().unwrap(),
             "Class1* this_ptr, double arg1");

  assert!(!sig.has_const_this());
}

#[test]
fn signature_class_method_const() {
  let sig = CppFfiFunctionSignature {
    arguments: vec![CppFfiFunctionArgument {
                      name: "this_ptr".to_string(),
                      argument_type: CppFfiType {
                        original_type: CppType {
                          indirection: CppTypeIndirection::Ptr,
                          is_const: true,
                          is_const2: false,
                          base: CppTypeBase::Class(CppTypeClassBase {
                            name: "Class1".to_string(),
                            template_arguments: None,
                          }),
                        },
                        ffi_type: CppType {
                          indirection: CppTypeIndirection::Ptr,
                          is_const: true,
                          is_const2: false,
                          base: CppTypeBase::Class(CppTypeClassBase {
                            name: "Class1".to_string(),
                            template_arguments: None,
                          }),
                        },
                        conversion: IndirectionChange::NoChange,
                      },
                      meaning: CppFfiArgumentMeaning::This,
                    },
                    CppFfiFunctionArgument {
                      name: "arg1".to_string(),
                      argument_type: CppFfiType {
                        original_type: CppType {
                          indirection: CppTypeIndirection::None,
                          is_const: false,
                          is_const2: false,
                          base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Double),
                        },
                        ffi_type: CppType {
                          indirection: CppTypeIndirection::None,
                          is_const: false,
                          is_const2: false,
                          base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Double),
                        },
                        conversion: IndirectionChange::NoChange,
                      },
                      meaning: CppFfiArgumentMeaning::Argument(0),
                    }],
    return_type: CppFfiType::void(),
  };

  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::NameOnly),
             "arg1");
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Short)),
             "double");
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Full)),
             "double");
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Short)),
             "double_arg1");
  assert_eq!(sig.arguments_caption(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Full)),
             "double_arg1");

  assert_eq!(sig.caption(MethodCaptionStrategy::ConstOnly), "const");

  assert_eq!(sig.caption(MethodCaptionStrategy::ArgumentsOnly(ArgumentCaptionStrategy::NameOnly)),
             "arg1");
  assert_eq!(sig.caption(MethodCaptionStrategy::ArgumentsOnly(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Short))),
             "double");
  assert_eq!(sig.caption(MethodCaptionStrategy::ArgumentsOnly(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Full))),
             "double");
  assert_eq!(sig.caption(MethodCaptionStrategy::ArgumentsOnly(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Short))),
             "double_arg1");
  assert_eq!(sig.caption(MethodCaptionStrategy::ArgumentsOnly(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Full))),
             "double_arg1");

  assert_eq!(sig.caption(MethodCaptionStrategy::ConstAndArguments(ArgumentCaptionStrategy::NameOnly)),
             "const_arg1");
  assert_eq!(sig.caption(MethodCaptionStrategy::ConstAndArguments(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Short))),
             "const_double");
  assert_eq!(sig.caption(MethodCaptionStrategy::ConstAndArguments(ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Full))),
             "const_double");
  assert_eq!(sig.caption(MethodCaptionStrategy::ConstAndArguments(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Short))),
             "const_double_arg1");
  assert_eq!(sig.caption(MethodCaptionStrategy::ConstAndArguments(ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Full))),
             "const_double_arg1");

  assert_eq!(sig.arguments_to_cpp_code().unwrap(),
             "const Class1* this_ptr, double arg1");

  assert!(sig.has_const_this());
}




#[test]
fn cpp_ffi_type_void() {
  let t = CppFfiType::void();
  assert!(t.original_type.is_void());
  assert!(t.ffi_type.is_void());
  assert_eq!(t.conversion, IndirectionChange::NoChange);
}

#[test]
fn c_base_name_free_func() {
  let mut method = empty_regular_method();
  method.name = "func1".to_string();
  let include_file = "QRect".to_string();
  assert_eq!(c_base_name(&method,
                         &ReturnValueAllocationPlace::NotApplicable,
                         &include_file)
               .unwrap(),
             "QRect_G_func1");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Stack, &include_file).unwrap(),
             "QRect_G_func1_to_output");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Heap, &include_file).unwrap(),
             "QRect_G_func1_as_ptr");
}

#[test]
fn c_base_name_free_func_in_namespace() {
  let mut method = empty_regular_method();
  method.name = "ns::func1".to_string();
  let include_file = "QRect".to_string();
  assert_eq!(c_base_name(&method,
                         &ReturnValueAllocationPlace::NotApplicable,
                         &include_file)
               .unwrap(),
             "QRect_G_ns_func1");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Stack, &include_file).unwrap(),
             "QRect_G_ns_func1_to_output");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Heap, &include_file).unwrap(),
             "QRect_G_ns_func1_as_ptr");
}

#[test]
fn c_base_name_class_method() {
  let mut method = empty_regular_method();
  method.name = "func1".to_string();
  method.class_membership = Some(empty_membership("MyClass"));
  let include_file = "QRect".to_string();
  assert_eq!(c_base_name(&method,
                         &ReturnValueAllocationPlace::NotApplicable,
                         &include_file)
               .unwrap(),
             "MyClass_func1");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Stack, &include_file).unwrap(),
             "MyClass_func1_to_output");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Heap, &include_file).unwrap(),
             "MyClass_func1_as_ptr");
}

#[test]
fn c_base_name_class_method_in_namespace() {
  let mut method = empty_regular_method();
  method.name = "func1".to_string();
  method.class_membership = Some(empty_membership("ns1::MyClass"));
  let include_file = "QRect".to_string();
  assert_eq!(c_base_name(&method,
                         &ReturnValueAllocationPlace::NotApplicable,
                         &include_file)
               .unwrap(),
             "ns1_MyClass_func1");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Stack, &include_file).unwrap(),
             "ns1_MyClass_func1_to_output");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Heap, &include_file).unwrap(),
             "ns1_MyClass_func1_as_ptr");
}

#[test]
fn c_base_name_constructor() {
  let mut method = empty_regular_method();
  method.name = "QRect".to_string();
  method.class_membership = Some({
    let mut info = empty_membership("QRect");
    info.kind = CppMethodKind::Constructor;
    info
  });
  let include_file = "QtCore".to_string();
  assert!(c_base_name(&method,
                      &ReturnValueAllocationPlace::NotApplicable,
                      &include_file)
    .is_err());
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Stack, &include_file).unwrap(),
             "QRect_constructor");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Heap, &include_file).unwrap(),
             "QRect_new");
}

#[test]
fn c_base_name_destructor() {
  let mut method = empty_regular_method();
  method.name = "QRect".to_string();
  method.class_membership = Some({
    let mut info = empty_membership("QRect");
    info.kind = CppMethodKind::Destructor;
    info
  });
  let include_file = "QtCore".to_string();
  assert!(c_base_name(&method,
                      &ReturnValueAllocationPlace::NotApplicable,
                      &include_file)
    .is_err());
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Stack, &include_file).unwrap(),
             "QRect_destructor");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Heap, &include_file).unwrap(),
             "QRect_delete");
}

#[test]
fn c_base_name_class_method_operator() {
  let mut method = empty_regular_method();
  method.name = "operator>".to_string();
  method.class_membership = Some(empty_membership("MyClass"));
  method.operator = Some(CppOperator::GreaterThan);
  let include_file = "QRect".to_string();
  assert_eq!(c_base_name(&method,
                         &ReturnValueAllocationPlace::NotApplicable,
                         &include_file)
               .unwrap(),
             "MyClass_operator_gt");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Stack, &include_file).unwrap(),
             "MyClass_operator_gt_to_output");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Heap, &include_file).unwrap(),
             "MyClass_operator_gt_as_ptr");
}

#[test]
fn c_base_name_free_func_operator() {
  let mut method = empty_regular_method();
  method.name = "operator>".to_string();
  method.operator = Some(CppOperator::GreaterThan);
  let include_file = "QRect".to_string();
  assert_eq!(c_base_name(&method,
                         &ReturnValueAllocationPlace::NotApplicable,
                         &include_file)
               .unwrap(),
             "QRect_G_operator_gt");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Stack, &include_file).unwrap(),
             "QRect_G_operator_gt_to_output");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Heap, &include_file).unwrap(),
             "QRect_G_operator_gt_as_ptr");
}

#[test]
fn c_base_name_conversion_operator() {
  let mut method = empty_regular_method();
  method.name = "operator const QPoint&".to_string();
  method.class_membership = Some(empty_membership("MyClass"));
  method.operator = Some(CppOperator::Conversion(CppType {
    is_const: true,
    is_const2: false,
    base: CppTypeBase::Class(CppTypeClassBase {
      name: "QPoint".to_string(),
      template_arguments: None,
    }),
    indirection: CppTypeIndirection::Ref,
  }));
  let include_file = "QRect".to_string();
  assert_eq!(c_base_name(&method,
                         &ReturnValueAllocationPlace::NotApplicable,
                         &include_file)
               .unwrap(),
             "MyClass_convert_to_const_QPoint_ref");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Stack, &include_file).unwrap(),
             "MyClass_convert_to_const_QPoint_ref_to_output");
  assert_eq!(c_base_name(&method, &ReturnValueAllocationPlace::Heap, &include_file).unwrap(),
             "MyClass_convert_to_const_QPoint_ref_as_ptr");
}
