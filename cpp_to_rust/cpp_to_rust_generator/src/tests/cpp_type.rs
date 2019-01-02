use crate::cpp_data::CppName;
use crate::cpp_ffi_data::CppTypeConversionToFfi;
use crate::cpp_type::{
    CppBuiltInNumericType, CppClassType, CppFunctionPointerType, CppSpecificNumericType,
    CppSpecificNumericTypeKind, CppType, CppTypeRole,
};

fn assert_type_to_ffi_unchanged(t: &CppType) {
    for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
        let ffi1 = t.to_cpp_ffi_type(role.clone()).unwrap();
        assert_eq!(&ffi1.original_type, t);
        assert_eq!(&ffi1.ffi_type, t);
        assert_eq!(ffi1.conversion, CppTypeConversionToFfi::NoChange);
    }
}

#[test]
fn void() {
    let type1 = CppType::Void;
    assert_eq!(type1.is_void(), true);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "void");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn void_ptr() {
    let type1 = CppType::new_pointer(false, CppType::Void);
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "void*");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn int() {
    let type1 = CppType::BuiltInNumeric(CppBuiltInNumericType::Int);
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "int");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn bool_ptr() {
    let type1 = CppType::new_pointer(false, CppType::BuiltInNumeric(CppBuiltInNumericType::Bool));
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "bool*");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn char_ptr_ptr() {
    let type1 = CppType::new_pointer(
        false,
        CppType::new_pointer(false, CppType::BuiltInNumeric(CppBuiltInNumericType::Char)),
    );
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "char**");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn qint64() {
    let type1 = CppType::SpecificNumeric(CppSpecificNumericType {
        name: CppName::from_one_part("qint64"),
        bits: 64,
        kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
    });
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "qint64");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn quintptr() {
    let type1 = CppType::PointerSizedInteger {
        name: CppName::from_one_part("quintptr"),
        is_signed: false,
    };
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "quintptr");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn enum1() {
    let type1 = CppType::Enum {
        name: CppName::from_one_part("Qt::CaseSensitivity"),
    };
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "Qt::CaseSensitivity");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn class_value() {
    let type1 = CppType::Class(CppClassType {
        name: CppName::from_one_part("QPoint"),
        template_arguments: None,
    });
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), true);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "QPoint");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());

    let ffi_return_type = type1.to_cpp_ffi_type(CppTypeRole::ReturnType).unwrap();
    assert_eq!(&ffi_return_type.original_type, &type1);
    assert_eq!(
        &ffi_return_type.ffi_type,
        &CppType::new_pointer(
            false,
            CppType::Class(CppClassType {
                name: CppName::from_one_part("QPoint"),
                template_arguments: None,
            })
        ),
    );
    assert_eq!(
        &ffi_return_type.ffi_type.to_cpp_code(None).unwrap(),
        "QPoint*"
    );
    assert_eq!(
        ffi_return_type.conversion,
        CppTypeConversionToFfi::ValueToPointer
    );

    let ffi_arg = type1.to_cpp_ffi_type(CppTypeRole::NotReturnType).unwrap();
    assert_eq!(&ffi_arg.original_type, &type1);
    assert_eq!(
        &ffi_arg.ffi_type,
        &CppType::new_pointer(
            true,
            CppType::Class(CppClassType {
                name: CppName::from_one_part("QPoint"),
                template_arguments: None,
            })
        )
    );
    assert_eq!(
        &ffi_arg.ffi_type.to_cpp_code(None).unwrap(),
        "const QPoint*"
    );
    assert_eq!(ffi_arg.conversion, CppTypeConversionToFfi::ValueToPointer);
}

#[test]
fn class_const_ref() {
    let type1 = CppType::new_reference(
        true,
        CppType::Class(CppClassType {
            name: CppName::from_one_part("QRectF"),
            template_arguments: None,
        }),
    );
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "const QRectF&");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());

    for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
        let ffi1 = type1.to_cpp_ffi_type(role.clone()).unwrap();
        assert_eq!(&ffi1.original_type, &type1);
        assert_eq!(
            &ffi1.ffi_type,
            &CppType::new_pointer(
                true,
                CppType::Class(CppClassType {
                    name: CppName::from_one_part("QRectF"),
                    template_arguments: None,
                })
            )
        );
        assert_eq!(&ffi1.ffi_type.to_cpp_code(None).unwrap(), "const QRectF*");
        assert_eq!(ffi1.conversion, CppTypeConversionToFfi::ReferenceToPointer);
    }
}

#[test]
fn class_mut_ref() {
    let type1 = CppType::new_reference(
        false,
        CppType::Class(CppClassType {
            name: CppName::from_one_part("QRectF"),
            template_arguments: None,
        }),
    );
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "QRectF&");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());

    for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
        let ffi1 = type1.to_cpp_ffi_type(role.clone()).unwrap();
        assert_eq!(&ffi1.original_type, &type1);
        assert_eq!(
            &ffi1.ffi_type,
            &CppType::new_pointer(
                false,
                CppType::Class(CppClassType {
                    name: CppName::from_one_part("QRectF"),
                    template_arguments: None,
                })
            )
        );
        assert_eq!(&ffi1.ffi_type.to_cpp_code(None).unwrap(), "QRectF*");
        assert_eq!(ffi1.conversion, CppTypeConversionToFfi::ReferenceToPointer);
    }
}

#[test]
fn class_mut_ptr() {
    let type1 = CppType::new_pointer(
        false,
        CppType::Class(CppClassType {
            name: CppName::from_one_part("QObject"),
            template_arguments: None,
        }),
    );
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "QObject*");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn class_with_template_args() {
    let args = Some(vec![CppType::Class(CppClassType {
        name: CppName::from_one_part("QString"),
        template_arguments: None,
    })]);
    let type1 = CppType::Class(CppClassType {
        name: CppName::from_one_part("QVector"),
        template_arguments: args.clone(),
    });
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), true);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(type1.to_cpp_code(None).unwrap(), "QVector< QString >");
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());

    let ffi_return_type = type1.to_cpp_ffi_type(CppTypeRole::ReturnType).unwrap();
    assert_eq!(&ffi_return_type.original_type, &type1);
    assert_eq!(
        &ffi_return_type.ffi_type,
        &CppType::new_pointer(
            false,
            CppType::Class(CppClassType {
                name: CppName::from_one_part("QVector"),
                template_arguments: args.clone(),
            })
        ),
    );
    assert_eq!(
        &ffi_return_type.ffi_type.to_cpp_code(None).unwrap(),
        "QVector< QString >*"
    );
    assert_eq!(
        ffi_return_type.conversion,
        CppTypeConversionToFfi::ValueToPointer
    );

    let ffi_arg = type1.to_cpp_ffi_type(CppTypeRole::NotReturnType).unwrap();
    assert_eq!(&ffi_arg.original_type, &type1);
    assert_eq!(
        &ffi_arg.ffi_type,
        &CppType::new_pointer(
            true,
            CppType::Class(CppClassType {
                name: CppName::from_one_part("QVector"),
                template_arguments: args.clone(),
            })
        )
    );
    assert_eq!(
        &ffi_arg.ffi_type.to_cpp_code(None).unwrap(),
        "const QVector< QString >*"
    );
    assert_eq!(ffi_arg.conversion, CppTypeConversionToFfi::ValueToPointer);
}

#[test]
fn nested_template_cpp_code() {
    let type1 = CppType::Class(CppClassType {
        name: CppName::from_one_part("QHash"),
        template_arguments: Some(vec![
            CppType::Class(CppClassType {
                name: CppName::from_one_part("QString"),
                template_arguments: None,
            }),
            CppType::Class(CppClassType {
                name: CppName::from_one_part("QList"),
                template_arguments: Some(vec![CppType::Class(CppClassType {
                    name: CppName::from_one_part("QString"),
                    template_arguments: None,
                })]),
            }),
        ]),
    });
    let code = type1.to_cpp_code(None).unwrap();
    assert_eq!(&code, "QHash< QString, QList< QString > >");
    assert!(!code.contains(">>"));
    assert!(!code.contains("<<"));
}

#[test]
fn qflags() {
    let args = Some(vec![CppType::Class(CppClassType {
        name: CppName::from_one_part("Qt::AlignmentFlag"),
        template_arguments: None,
    })]);
    let type1 = CppType::Class(CppClassType {
        name: CppName::from_one_part("QFlags"),
        template_arguments: args.clone(),
    });
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), true);
    assert_eq!(type1.is_template_parameter(), false);
    assert_eq!(
        type1.to_cpp_code(None).unwrap(),
        "QFlags< Qt::AlignmentFlag >"
    );
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());

    for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
        let ffi_type = type1.to_cpp_ffi_type(role.clone()).unwrap();
        assert_eq!(&ffi_type.original_type, &type1);
        assert_eq!(
            &ffi_type.ffi_type,
            &CppType::BuiltInNumeric(CppBuiltInNumericType::UInt),
        );
        assert_eq!(
            &ffi_type.ffi_type.to_cpp_code(None).unwrap(),
            "unsigned int"
        );
        assert_eq!(ffi_type.conversion, CppTypeConversionToFfi::QFlagsToUInt);
    }
}

#[test]
fn template_parameter() {
    let type1 = CppType::new_pointer(
        false,
        CppType::TemplateParameter {
            nested_level: 0,
            index: 0,
            name: "T".into(),
        },
    );
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert!(type1.to_cpp_code(None).is_err());
    assert!(type1.to_cpp_code(Some(&String::new())).is_err());
    assert!(type1.to_cpp_ffi_type(CppTypeRole::NotReturnType).is_err());
    assert!(type1.to_cpp_ffi_type(CppTypeRole::ReturnType).is_err());
}

#[test]
fn function1() {
    let type1 = CppType::FunctionPointer(CppFunctionPointerType {
        allows_variadic_arguments: false,
        return_type: Box::new(CppType::BuiltInNumeric(CppBuiltInNumericType::Int)),
        arguments: vec![
            CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
            CppType::new_pointer(false, CppType::BuiltInNumeric(CppBuiltInNumericType::Bool)),
        ],
    });
    assert_eq!(type1.is_void(), false);
    assert_eq!(type1.is_class(), false);
    assert_eq!(type1.is_template_parameter(), false);
    assert!(type1.to_cpp_code(None).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn instantiate1() {
    let type1 = CppType::new_reference(
        true,
        CppType::TemplateParameter {
            nested_level: 0,
            index: 0,
            name: "T".into(),
        },
    );
    let type2 = CppType::new_pointer(false, CppType::BuiltInNumeric(CppBuiltInNumericType::Bool));
    let r = type1.instantiate(0, &[type2]).unwrap();
    assert_eq!(
        r,
        CppType::new_reference(
            true,
            CppType::new_pointer(false, CppType::BuiltInNumeric(CppBuiltInNumericType::Bool))
        )
    );
}
