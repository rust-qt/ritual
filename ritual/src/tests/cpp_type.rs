use crate::cpp_data::{CppPath, CppPathItem};
use crate::cpp_ffi_data::CppToFfiTypeConversion;
use crate::cpp_ffi_generator::ffi_type;
use crate::cpp_type::{
    CppBuiltInNumericType, CppFunctionPointerType, CppSpecificNumericType,
    CppSpecificNumericTypeKind, CppTemplateParameter, CppType, CppTypeRole,
};

fn assert_type_to_ffi_unchanged(t: &CppType) {
    for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
        let ffi1 = ffi_type(t, *role).unwrap();
        assert_eq!(ffi1.original_type(), t);
        assert_eq!(ffi1.ffi_type(), t);
        assert_eq!(ffi1.conversion(), &CppToFfiTypeConversion::NoChange);
    }
}

#[test]
fn void() {
    let type1 = CppType::Void;
    assert!(type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "void");
    assert!(type1.to_cpp_code(Some("")).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn void_ptr() {
    let type1 = CppType::new_pointer(false, CppType::Void);
    assert!(!type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "void *");
    assert!(type1.to_cpp_code(Some("")).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn int() {
    let type1 = CppType::BuiltInNumeric(CppBuiltInNumericType::Int);
    assert!(!type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "int");
    assert!(type1.to_cpp_code(Some("")).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn bool_ptr() {
    let type1 = CppType::new_pointer(false, CppType::BuiltInNumeric(CppBuiltInNumericType::Bool));
    assert!(!type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "bool *");
    assert!(type1.to_cpp_code(Some("")).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn char_ptr_ptr() {
    let type1 = CppType::new_pointer(
        false,
        CppType::new_pointer(false, CppType::BuiltInNumeric(CppBuiltInNumericType::Char)),
    );
    assert!(!type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "char * *");
    assert!(type1.to_cpp_code(Some("")).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn qint64() {
    let type1 = CppType::SpecificNumeric(CppSpecificNumericType {
        path: CppPath::from_good_str("qint64"),
        bits: 64,
        kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
    });
    assert!(!type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "qint64");
    assert!(type1.to_cpp_code(Some("")).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn quintptr() {
    let type1 = CppType::PointerSizedInteger {
        path: CppPath::from_good_str("quintptr"),
        is_signed: false,
    };
    assert!(!type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "quintptr");
    assert!(type1.to_cpp_code(Some("")).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn enum1() {
    let type1 = CppType::Enum {
        path: CppPath::from_good_str("Qt::CaseSensitivity"),
    };
    assert!(!type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "Qt::CaseSensitivity");
    assert!(type1.to_cpp_code(Some("")).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn class_value() {
    let type1 = CppType::Class(CppPath::from_good_str("QPoint"));
    assert!(!type1.is_void());
    assert!(type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "QPoint");
    assert!(type1.to_cpp_code(Some("")).is_err());

    let ffi_return_type = ffi_type(&type1, CppTypeRole::ReturnType).unwrap();
    assert_eq!(ffi_return_type.original_type(), &type1);
    assert_eq!(
        ffi_return_type.ffi_type(),
        &CppType::new_pointer(false, CppType::Class(CppPath::from_good_str("QPoint"))),
    );
    assert_eq!(
        ffi_return_type.ffi_type().to_cpp_code(None).unwrap(),
        "QPoint *"
    );
    assert_eq!(
        ffi_return_type.conversion(),
        &CppToFfiTypeConversion::ValueToPointer {
            is_ffi_const: false
        }
    );

    let ffi_arg = ffi_type(&type1, CppTypeRole::NotReturnType).unwrap();
    assert_eq!(ffi_arg.original_type(), &type1);
    assert_eq!(
        ffi_arg.ffi_type(),
        &CppType::new_pointer(true, CppType::Class(CppPath::from_good_str("QPoint")))
    );
    assert_eq!(
        ffi_arg.ffi_type().to_cpp_code(None).unwrap(),
        "QPoint const *"
    );
    assert_eq!(
        ffi_arg.conversion(),
        &CppToFfiTypeConversion::ValueToPointer { is_ffi_const: true }
    );
}

#[test]
fn class_const_ref() {
    let type1 = CppType::new_reference(true, CppType::Class(CppPath::from_good_str("QRectF")));
    assert!(!type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "QRectF const &");
    assert!(type1.to_cpp_code(Some("")).is_err());

    for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
        let ffi1 = ffi_type(&type1, *role).unwrap();
        assert_eq!(ffi1.original_type(), &type1);
        assert_eq!(
            ffi1.ffi_type(),
            &CppType::new_pointer(true, CppType::Class(CppPath::from_good_str("QRectF")))
        );
        assert_eq!(ffi1.ffi_type().to_cpp_code(None).unwrap(), "QRectF const *");
        assert_eq!(
            ffi1.conversion(),
            &CppToFfiTypeConversion::ReferenceToPointer
        );
    }
}

#[test]
fn class_mut_ref() {
    let type1 = CppType::new_reference(false, CppType::Class(CppPath::from_good_str("QRectF")));
    assert!(!type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "QRectF &");
    assert!(type1.to_cpp_code(Some("")).is_err());

    for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
        let ffi1 = ffi_type(&type1, *role).unwrap();
        assert_eq!(ffi1.original_type(), &type1);
        assert_eq!(
            ffi1.ffi_type(),
            &CppType::new_pointer(false, CppType::Class(CppPath::from_good_str("QRectF")))
        );
        assert_eq!(ffi1.ffi_type().to_cpp_code(None).unwrap(), "QRectF *");
        assert_eq!(
            ffi1.conversion(),
            &CppToFfiTypeConversion::ReferenceToPointer
        );
    }
}

#[test]
fn class_mut_ptr() {
    let type1 = CppType::new_pointer(false, CppType::Class(CppPath::from_good_str("QObject")));
    assert!(!type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "QObject *");
    assert!(type1.to_cpp_code(Some("")).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn class_with_template_args() {
    let args = Some(vec![CppType::Class(CppPath::from_good_str("QString"))]);
    let type1 = CppType::Class(CppPath::from_item(CppPathItem {
        name: "QVector".into(),
        template_arguments: args.clone(),
    }));
    assert!(!type1.is_void());
    assert!(type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(type1.to_cpp_code(None).unwrap(), "QVector< QString >");
    assert!(type1.to_cpp_code(Some("")).is_err());

    let ffi_return_type = ffi_type(&type1, CppTypeRole::ReturnType).unwrap();
    assert_eq!(ffi_return_type.original_type(), &type1);
    assert_eq!(
        ffi_return_type.ffi_type(),
        &CppType::new_pointer(
            false,
            CppType::Class(CppPath::from_item(CppPathItem {
                name: "QVector".into(),
                template_arguments: args.clone()
            }))
        ),
    );
    assert_eq!(
        ffi_return_type.ffi_type().to_cpp_code(None).unwrap(),
        "QVector< QString > *"
    );
    assert_eq!(
        ffi_return_type.conversion(),
        &CppToFfiTypeConversion::ValueToPointer {
            is_ffi_const: false
        }
    );

    let ffi_arg = ffi_type(&type1, CppTypeRole::NotReturnType).unwrap();
    assert_eq!(ffi_arg.original_type(), &type1);
    assert_eq!(
        ffi_arg.ffi_type(),
        &CppType::new_pointer(
            true,
            CppType::Class(CppPath::from_item(CppPathItem {
                name: "QVector".into(),
                template_arguments: args,
            }))
        )
    );
    assert_eq!(
        ffi_arg.ffi_type().to_cpp_code(None).unwrap(),
        "QVector< QString > const *"
    );
    assert_eq!(
        ffi_arg.conversion(),
        &CppToFfiTypeConversion::ValueToPointer { is_ffi_const: true }
    );
}

#[test]
fn nested_template_cpp_code() {
    let qlist_args = Some(vec![CppType::Class(CppPath::from_good_str("QString"))]);
    let qhash_args = Some(vec![
        CppType::Class(CppPath::from_good_str("QString")),
        CppType::Class(CppPath::from_item(CppPathItem {
            name: "QList".into(),
            template_arguments: qlist_args,
        })),
    ]);
    let type1 = CppType::Class(CppPath::from_item(CppPathItem {
        name: "QHash".into(),
        template_arguments: qhash_args,
    }));
    let code = type1.to_cpp_code(None).unwrap();
    assert_eq!(&code, "QHash< QString, QList< QString > >");
    assert!(!code.contains(">>"));
    assert!(!code.contains("<<"));
}

#[test]
fn qflags() {
    let args = Some(vec![CppType::Class(CppPath::from_good_str(
        "Qt::AlignmentFlag",
    ))]);
    let type1 = CppType::Class(CppPath::from_item(CppPathItem {
        name: "QFlags".into(),
        template_arguments: args,
    }));
    assert!(!type1.is_void());
    assert!(type1.is_class());
    assert!(!type1.is_template_parameter());
    assert_eq!(
        type1.to_cpp_code(None).unwrap(),
        "QFlags< Qt::AlignmentFlag >"
    );
    assert!(type1.to_cpp_code(Some("")).is_err());

    for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
        let ffi_type = ffi_type(&type1, *role).unwrap();
        assert_eq!(ffi_type.original_type(), &type1);
        assert_eq!(
            ffi_type.ffi_type(),
            &CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
        );
        assert_eq!(ffi_type.ffi_type().to_cpp_code(None).unwrap(), "int");
        assert_eq!(ffi_type.conversion(), &CppToFfiTypeConversion::QFlagsToInt);
    }
}

#[test]
fn template_parameter() {
    let type1 = CppType::new_pointer(
        false,
        CppType::TemplateParameter(CppTemplateParameter {
            nested_level: 0,
            index: 0,
            name: "T".into(),
        }),
    );
    assert!(!type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert!(type1.to_cpp_code(None).is_err());
    assert!(type1.to_cpp_code(Some("")).is_err());
    assert!(ffi_type(&type1, CppTypeRole::NotReturnType).is_err());
    assert!(ffi_type(&type1, CppTypeRole::ReturnType).is_err());
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
    assert!(!type1.is_void());
    assert!(!type1.is_class());
    assert!(!type1.is_template_parameter());
    assert!(type1.to_cpp_code(None).is_err());
    assert_type_to_ffi_unchanged(&type1);
}

#[test]
fn instantiate1() {
    let type1 = CppType::new_reference(
        true,
        CppType::TemplateParameter(CppTemplateParameter {
            nested_level: 0,
            index: 0,
            name: "T".into(),
        }),
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
