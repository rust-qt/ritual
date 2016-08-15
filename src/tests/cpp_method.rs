use cpp_method::*;
use cpp_operators::CppOperator;
use cpp_data::CppVisibility;
use cpp_type::*;

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
