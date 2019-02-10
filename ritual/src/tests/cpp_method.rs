use crate::cpp_data::CppPath;
use crate::cpp_data::CppVisibility;
use crate::cpp_ffi_data::CppFfiArgumentMeaning;
use crate::cpp_ffi_data::CppFfiFunction;
use crate::cpp_ffi_data::CppTypeConversionToFfi;
use crate::cpp_function::*;
use crate::cpp_type::*;
use std::collections::HashSet;

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

pub fn empty_membership() -> CppFunctionMemberData {
    CppFunctionMemberData {
        kind: CppFunctionKind::Regular,
        is_virtual: false,
        is_pure_virtual: false,
        is_const: false,
        is_static: false,
        visibility: CppVisibility::Public,
        is_signal: false,
        is_slot: false,
    }
}

pub fn empty_regular_method() -> CppFunction {
    CppFunction {
        path: CppPath::from_str_unchecked("empty"),
        member: None,
        return_type: CppType::Void,
        arguments: vec![],
        doc: None,
        allows_variadic_arguments: false,
        operator: None,
        declaration_code: None,
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
        argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
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
        argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
        name: "arg1".to_string(),
        has_default_value: false,
    });
    method2.arguments.push(CppFunctionArgument {
        argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
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
        argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
        name: "arg1".to_string(),
        has_default_value: false,
    });
    method2.arguments.push(CppFunctionArgument {
        argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
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
        argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
        name: "arg1".to_string(),
        has_default_value: false,
    });
    method2.arguments.push(CppFunctionArgument {
        argument_type: CppType::Enum {
            path: CppPath::from_str_unchecked("Enum1"),
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
        argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
        name: "arg1".to_string(),
        has_default_value: false,
    });
    method2.arguments.push(CppFunctionArgument {
        argument_type: CppType::new_pointer(
            false,
            CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
        ),
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
        argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
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
    method1.return_type = CppType::BuiltInNumeric(CppBuiltInNumericType::Int);
    assert!(method1.argument_types_equal(&method2));
    assert!(method2.argument_types_equal(&method1));
}

fn to_ffi(function: &CppFunction, force_stack: Option<CppPath>) -> CppFfiFunction {
    let movable_types: Vec<_> = force_stack.into_iter().collect();
    crate::cpp_ffi_generator::to_ffi_method(
        function,
        &movable_types,
        &mut crate::cpp_ffi_generator::FfiNameProvider::new(String::new(), HashSet::new()),
    )
    .unwrap()
}

#[test]
fn c_signature_empty() {
    let mut method1 = empty_regular_method();
    method1.return_type = CppType::Void;

    assert!(!method1.is_constructor());
    assert!(!method1.is_destructor());
    assert!(!method1.is_operator());
    assert!(method1.class_type().is_err());

    let r = to_ffi(&method1, None);
    assert!(r.arguments.is_empty());
    assert!(r.return_type.ffi_type.is_void());
}

#[test]
fn c_signature_simple_func() {
    let mut method1 = empty_regular_method();
    method1.return_type = CppType::BuiltInNumeric(CppBuiltInNumericType::Int);
    method1.arguments.push(CppFunctionArgument {
        argument_type: CppType::Enum {
            path: CppPath::from_str_unchecked("Enum1"),
        },
        name: "arg1".to_string(),
        has_default_value: false,
    });
    let r = to_ffi(&method1, None);
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
    method1.path = CppPath::from_str_unchecked("MyClass::empty");
    method1.member = Some(empty_membership());
    method1.return_type = CppType::BuiltInNumeric(CppBuiltInNumericType::Int);
    method1.arguments.push(CppFunctionArgument {
        argument_type: CppType::Class(CppPath::from_str_unchecked("MyClass2")),
        name: "my_arg".to_string(),
        has_default_value: false,
    });

    assert!(!method1.is_constructor());
    assert!(!method1.is_destructor());
    assert!(!method1.is_operator());
    assert_eq!(
        method1.class_type().unwrap(),
        CppPath::from_str_unchecked("MyClass")
    );

    let r = to_ffi(&method1, None);
    assert!(r.arguments.len() == 2);
    assert_eq!(r.arguments[0].name, "this_ptr");
    assert_eq!(
        r.arguments[0].argument_type.ffi_type,
        CppType::new_pointer(false, CppType::Class(method1.class_type().unwrap()))
    );
    assert_eq!(
        r.arguments[0].argument_type.conversion,
        CppTypeConversionToFfi::NoChange
    );
    assert_eq!(r.arguments[0].meaning, CppFfiArgumentMeaning::This);

    assert_eq!(r.arguments[1].name, "my_arg");
    assert_eq!(
        r.arguments[1].argument_type.ffi_type,
        CppType::new_pointer(true, method1.arguments[0].argument_type.clone())
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
    method1.path = CppPath::from_str_unchecked("MyClass::empty");
    method1.member = Some({
        let mut info = empty_membership();
        info.is_static = true;
        info
    });
    method1.return_type = CppType::BuiltInNumeric(CppBuiltInNumericType::Int);
    method1.arguments.push(CppFunctionArgument {
        argument_type: CppType::Enum {
            path: CppPath::from_str_unchecked("Enum1"),
        },
        name: "arg1".to_string(),
        has_default_value: false,
    });
    let r = to_ffi(&method1, None);
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
    method1.path = CppPath::from_str_unchecked("MyClass::empty");
    method1.member = Some({
        let mut info = empty_membership();
        info.kind = CppFunctionKind::Constructor;
        info
    });
    method1.arguments.push(CppFunctionArgument {
        argument_type: CppType::new_reference(
            true,
            CppType::Enum {
                path: CppPath::from_str_unchecked("Enum1"),
            },
        ),
        name: "arg1".to_string(),
        has_default_value: true,
    });

    assert!(method1.is_constructor());
    assert!(!method1.is_destructor());
    assert!(!method1.is_operator());
    assert_eq!(
        method1.class_type().unwrap(),
        CppPath::from_str_unchecked("MyClass")
    );

    let r_stack = to_ffi(&method1, Some(CppPath::from_str_unchecked("MyClass")));

    assert!(r_stack.arguments.len() == 2);
    assert_eq!(r_stack.arguments[0].name, "arg1");
    assert_eq!(
        r_stack.arguments[0].argument_type.ffi_type,
        CppType::new_pointer(
            true,
            CppType::Enum {
                path: CppPath::from_str_unchecked("Enum1"),
            }
        )
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
        CppType::new_pointer(
            false,
            CppType::Class(CppPath::from_str_unchecked("MyClass"))
        ),
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

    let r_heap = to_ffi(&method1, None);
    assert!(r_heap.arguments.len() == 1);
    assert_eq!(r_heap.arguments[0].name, "arg1");
    assert_eq!(
        r_heap.arguments[0].argument_type.ffi_type,
        CppType::new_pointer(
            true,
            CppType::Enum {
                path: CppPath::from_str_unchecked("Enum1"),
            }
        ),
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
        CppType::new_pointer(
            false,
            CppType::Class(CppPath::from_str_unchecked("MyClass"))
        ),
    );
    assert_eq!(
        r_heap.return_type.conversion,
        CppTypeConversionToFfi::ValueToPointer
    );
}

#[test]
fn c_signature_destructor() {
    let mut method1 = empty_regular_method();
    method1.path = CppPath::from_str_unchecked("MyClass::empty");
    method1.member = Some({
        let mut info = empty_membership();
        info.kind = CppFunctionKind::Destructor;
        info
    });

    assert!(!method1.is_constructor());
    assert!(method1.is_destructor());
    assert!(!method1.is_operator());
    assert_eq!(
        method1.class_type().unwrap(),
        CppPath::from_str_unchecked("MyClass")
    );

    let r_stack = to_ffi(&method1, Some(CppPath::from_str_unchecked("MyClass")));
    assert_eq!(r_stack.arguments.len(), 1);
    assert_eq!(r_stack.arguments[0].name, "this_ptr");
    assert_eq!(
        r_stack.arguments[0].argument_type.ffi_type,
        CppType::new_pointer(false, CppType::Class(method1.class_type().unwrap()))
    );
    assert_eq!(
        r_stack.arguments[0].argument_type.conversion,
        CppTypeConversionToFfi::NoChange
    );
    assert_eq!(r_stack.arguments[0].meaning, CppFfiArgumentMeaning::This);

    assert!(r_stack.return_type.ffi_type.is_void());

    let r_heap = to_ffi(&method1, None);
    assert!(r_heap.arguments.len() == 1);
    assert_eq!(r_heap.arguments[0].name, "this_ptr");
    assert_eq!(
        r_heap.arguments[0].argument_type.ffi_type,
        CppType::new_pointer(false, CppType::Class(method1.class_type().unwrap()))
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
    method1.path = CppPath::from_str_unchecked("MyClass::empty");
    method1.member = Some(empty_membership());
    method1.return_type = CppType::Class(CppPath::from_str_unchecked("MyClass3"));
    method1.arguments.push(CppFunctionArgument {
        argument_type: CppType::Class(CppPath::from_str_unchecked("MyClass2")),
        name: "my_arg".to_string(),
        has_default_value: false,
    });
    let r_stack = to_ffi(&method1, Some(CppPath::from_str_unchecked("MyClass3")));
    assert!(r_stack.arguments.len() == 3);
    assert_eq!(r_stack.arguments[0].name, "this_ptr");
    assert_eq!(
        r_stack.arguments[0].argument_type.ffi_type,
        CppType::new_pointer(false, CppType::Class(method1.class_type().unwrap()))
    );
    assert_eq!(
        r_stack.arguments[0].argument_type.conversion,
        CppTypeConversionToFfi::NoChange
    );
    assert_eq!(r_stack.arguments[0].meaning, CppFfiArgumentMeaning::This);

    assert_eq!(r_stack.arguments[1].name, "my_arg");
    assert_eq!(
        r_stack.arguments[1].argument_type.ffi_type,
        CppType::new_pointer(true, method1.arguments[0].argument_type.clone())
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
        CppType::new_pointer(
            false,
            CppType::Class(CppPath::from_str_unchecked("MyClass3"))
        ),
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

    let r_heap = to_ffi(&method1, None);
    assert!(r_heap.arguments.len() == 2);
    assert_eq!(r_heap.arguments[0].name, "this_ptr");
    assert_eq!(
        r_heap.arguments[0].argument_type.ffi_type,
        CppType::new_pointer(false, CppType::Class(method1.class_type().unwrap()))
    );
    assert_eq!(
        r_heap.arguments[0].argument_type.conversion,
        CppTypeConversionToFfi::NoChange
    );
    assert_eq!(r_heap.arguments[0].meaning, CppFfiArgumentMeaning::This);

    assert_eq!(r_heap.arguments[1].name, "my_arg");
    assert_eq!(
        r_heap.arguments[1].argument_type.ffi_type,
        CppType::new_pointer(true, method1.arguments[0].argument_type.clone())
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
        CppType::new_pointer(
            false,
            CppType::Class(CppPath::from_str_unchecked("MyClass3"))
        ),
    );
    assert_eq!(
        r_heap.return_type.conversion,
        CppTypeConversionToFfi::ValueToPointer
    );
}

#[test]
fn full_name_free_function_in_namespace() {
    let mut method1 = empty_regular_method();
    method1.path = CppPath::from_str_unchecked("ns::func1");
    assert!(method1.class_type().is_err());
}

#[test]
fn full_name_method() {
    let mut method1 = empty_regular_method();
    method1.path = CppPath::from_str_unchecked("MyClass::func1");
    method1.member = Some(empty_membership());
    assert_eq!(
        method1.class_type().unwrap(),
        CppPath::from_str_unchecked("MyClass")
    );
}

#[test]
fn full_name_static_method() {
    let mut method1 = empty_regular_method();
    method1.path = CppPath::from_str_unchecked("MyClass::func1");
    method1.member = Some({
        let mut info = empty_membership();
        info.is_static = true;
        info
    });
    assert_eq!(
        method1.class_type().unwrap(),
        CppPath::from_str_unchecked("MyClass")
    );
}

#[test]
fn full_name_nested_class_method() {
    let mut method1 = empty_regular_method();
    method1.path = CppPath::from_str_unchecked("MyClass::Iterator::func1");
    method1.member = Some(empty_membership());
    assert_eq!(
        method1.class_type().unwrap(),
        CppPath::from_str_unchecked("MyClass::Iterator")
    );
}

#[test]
fn short_text1() {
    let method = CppFunction {
        path: CppPath::from_str_unchecked("Class1::method1"),
        member: Some(CppFunctionMemberData {
            kind: CppFunctionKind::Regular,
            is_virtual: false,
            is_pure_virtual: false,
            is_const: true,
            is_static: false,
            visibility: CppVisibility::Protected,
            is_signal: false,
            is_slot: false,
        }),
        operator: None,
        return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
        arguments: vec![
            CppFunctionArgument {
                argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
                name: "arg1".to_string(),
                has_default_value: false,
            },
            CppFunctionArgument {
                argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Double),
                name: "arg2".to_string(),
                has_default_value: true,
            },
        ],
        doc: None,
        allows_variadic_arguments: false,
        declaration_code: None,
    };
    assert_eq!(
        method.short_text(),
        "protected int Class1::method1(int arg1, double arg2 = ?) const"
    );
}
