use cpp_ffi_data::*;
use cpp_type::*;

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
            original_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
            ffi_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
            conversion: CppTypeConversionToFfi::NoChange,
        },
        meaning: CppFfiArgumentMeaning::Argument(0),
    };

    assert_eq!(arg.to_cpp_code().unwrap(), "int arg1");
}

#[test]
fn argument_int_ptr() {
    let arg = CppFfiFunctionArgument {
        name: "arg1".to_string(),
        argument_type: CppFfiType {
            original_type: CppType::new_pointer(
                false,
                CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
            ),
            ffi_type: CppType::new_pointer(
                false,
                CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
            ),
            conversion: CppTypeConversionToFfi::NoChange,
        },
        meaning: CppFfiArgumentMeaning::Argument(0),
    };
    assert_eq!(arg.to_cpp_code().unwrap(), "int* arg1");
}

#[test]
fn argument_func() {
    let type1 = CppType::FunctionPointer(CppFunctionPointerType {
        allows_variadic_arguments: false,
        return_type: Box::new(CppType::BuiltInNumeric(CppBuiltInNumericType::Int)),
        arguments: vec![
            CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
            CppType::new_pointer(false, CppType::BuiltInNumeric(CppBuiltInNumericType::Bool)),
        ],
    });

    let arg = CppFfiFunctionArgument {
        name: "arg1".to_string(),
        argument_type: CppFfiType {
            original_type: type1.clone(),
            ffi_type: type1.clone(),
            conversion: CppTypeConversionToFfi::NoChange,
        },
        meaning: CppFfiArgumentMeaning::Argument(0),
    };
    assert_eq!(arg.to_cpp_code().unwrap(), "int (*arg1)(int, bool*)");
}

#[test]
fn cpp_ffi_type_void() {
    let t = CppFfiType::void();
    assert!(t.original_type.is_void());
    assert!(t.ffi_type.is_void());
    assert_eq!(t.conversion, CppTypeConversionToFfi::NoChange);
}
