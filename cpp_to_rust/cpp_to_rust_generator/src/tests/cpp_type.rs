use cpp_type::{CppType, CppTypeRole, CppTypeIndirection, CppTypeBase, CppBuiltInNumericType,
               CppSpecificNumericTypeKind, CppTypeClassBase, CppSpecificNumericType,
               CppFunctionPointerType};
use caption_strategy::TypeCaptionStrategy;
use cpp_ffi_data::CppIndirectionChange;

fn assert_type_to_ffi_unchanged(t: &CppType) {
  for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
    let ffi1 = t.to_cpp_ffi_type(role.clone()).unwrap();
    assert_eq!(&ffi1.original_type, t);
    assert_eq!(&ffi1.ffi_type, t);
    assert_eq!(ffi1.conversion, CppIndirectionChange::NoChange);
  }
}

#[test]
fn void() {
  let type1 = CppType::void();
  assert_eq!(type1.is_void(), true);
  assert_eq!(type1.base.is_void(), true);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "void");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "void");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "void"
  );
  assert_eq!(type1.caption(TypeCaptionStrategy::Short).unwrap(), "void");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full).unwrap(), "void");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
}

#[test]
fn void_ptr() {
  let type1 = CppType {
    indirection: CppTypeIndirection::Ptr,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::Void,
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), true);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "void*");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "void");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "void"
  );
  assert_eq!(type1.caption(TypeCaptionStrategy::Short).unwrap(), "void");
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Full).unwrap(),
    "void_ptr"
  );
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
}

#[test]
fn int() {
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "int");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "int");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "int"
  );
  assert_eq!(type1.caption(TypeCaptionStrategy::Short).unwrap(), "int");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full).unwrap(), "int");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
}

#[test]
fn bool_ptr() {
  let type1 = CppType {
    indirection: CppTypeIndirection::Ptr,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "bool*");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "bool");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "bool"
  );
  assert_eq!(type1.caption(TypeCaptionStrategy::Short).unwrap(), "bool");
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Full).unwrap(),
    "bool_ptr"
  );
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
}

#[test]
fn char_ptr_ptr() {
  let type1 = CppType {
    indirection: CppTypeIndirection::PtrPtr,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Char),
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "char**");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "char");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "char"
  );
  assert_eq!(type1.caption(TypeCaptionStrategy::Short).unwrap(), "char");
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Full).unwrap(),
    "char_ptr_ptr"
  );
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
}

#[test]
fn qint64() {
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::SpecificNumeric(CppSpecificNumericType {
      name: "qint64".to_string(),
      bits: 64,
      kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
    }),
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "qint64");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "qint64");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "qint64"
  );
  assert_eq!(type1.caption(TypeCaptionStrategy::Short).unwrap(), "qint64");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full).unwrap(), "qint64");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
}

#[test]
fn quintptr() {
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::PointerSizedInteger {
      name: "quintptr".to_string(),
      is_signed: false,
    },
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "quintptr");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "quintptr");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "quintptr"
  );
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Short).unwrap(),
    "quintptr"
  );
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Full).unwrap(),
    "quintptr"
  );
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
}

#[test]
fn enum1() {
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::Enum { name: "Qt::CaseSensitivity".to_string() },
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "Qt::CaseSensitivity");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "Qt::CaseSensitivity");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "Qt_CaseSensitivity"
  );
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Short).unwrap(),
    "Qt_CaseSensitivity"
  );
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Full).unwrap(),
    "Qt_CaseSensitivity"
  );
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
}


#[test]
fn class_value() {
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::Class(CppTypeClassBase {
      name: "QPoint".to_string(),
      template_arguments: None,
    }),
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), true);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "QPoint");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "QPoint");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "QPoint"
  );
  assert_eq!(type1.caption(TypeCaptionStrategy::Short).unwrap(), "QPoint");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full).unwrap(), "QPoint");

  let ffi_return_type = type1.to_cpp_ffi_type(CppTypeRole::ReturnType).unwrap();
  assert_eq!(&ffi_return_type.original_type, &type1);
  assert_eq!(
    &ffi_return_type.ffi_type,
    &CppType {
      indirection: CppTypeIndirection::Ptr,
      is_const: false,
      is_const2: false,
      base: CppTypeBase::Class(CppTypeClassBase {
        name: "QPoint".to_string(),
        template_arguments: None,
      }),
    }
  );
  assert_eq!(
    &ffi_return_type.ffi_type.to_cpp_code(None).unwrap(),
    "QPoint*"
  );
  assert_eq!(
    ffi_return_type.conversion,
    CppIndirectionChange::ValueToPointer
  );

  let ffi_arg = type1.to_cpp_ffi_type(CppTypeRole::NotReturnType).unwrap();
  assert_eq!(&ffi_arg.original_type, &type1);
  assert_eq!(
    &ffi_arg.ffi_type,
    &CppType {
      indirection: CppTypeIndirection::Ptr,
      is_const: true,
      is_const2: false,
      base: CppTypeBase::Class(CppTypeClassBase {
        name: "QPoint".to_string(),
        template_arguments: None,
      }),
    }
  );
  assert_eq!(
    &ffi_arg.ffi_type.to_cpp_code(None).unwrap(),
    "const QPoint*"
  );
  assert_eq!(ffi_arg.conversion, CppIndirectionChange::ValueToPointer);
  assert!(type1.needs_allocation_place_variants());
}

#[test]
fn class_const_ref() {
  let type1 = CppType {
    indirection: CppTypeIndirection::Ref,
    is_const: true,
    is_const2: false,
    base: CppTypeBase::Class(CppTypeClassBase {
      name: "QRectF".to_string(),
      template_arguments: None,
    }),
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), true);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "const QRectF&");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "QRectF");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "QRectF"
  );
  assert_eq!(type1.caption(TypeCaptionStrategy::Short).unwrap(), "QRectF");
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Full).unwrap(),
    "const_QRectF_ref"
  );

  for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
    let ffi1 = type1.to_cpp_ffi_type(role.clone()).unwrap();
    assert_eq!(&ffi1.original_type, &type1);
    assert_eq!(
      &ffi1.ffi_type,
      &CppType {
        indirection: CppTypeIndirection::Ptr,
        is_const: true,
        is_const2: false,
        base: CppTypeBase::Class(CppTypeClassBase {
          name: "QRectF".to_string(),
          template_arguments: None,
        }),
      }
    );
    assert_eq!(&ffi1.ffi_type.to_cpp_code(None).unwrap(), "const QRectF*");
    assert_eq!(ffi1.conversion, CppIndirectionChange::ReferenceToPointer);
  }
  assert!(!type1.needs_allocation_place_variants());
}

#[test]
fn class_mut_ref() {
  let type1 = CppType {
    indirection: CppTypeIndirection::Ref,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::Class(CppTypeClassBase {
      name: "QRectF".to_string(),
      template_arguments: None,
    }),
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), true);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "QRectF&");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "QRectF");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "QRectF"
  );
  assert_eq!(type1.caption(TypeCaptionStrategy::Short).unwrap(), "QRectF");
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Full).unwrap(),
    "QRectF_ref"
  );

  for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
    let ffi1 = type1.to_cpp_ffi_type(role.clone()).unwrap();
    assert_eq!(&ffi1.original_type, &type1);
    assert_eq!(
      &ffi1.ffi_type,
      &CppType {
        indirection: CppTypeIndirection::Ptr,
        is_const: false,
        is_const2: false,
        base: CppTypeBase::Class(CppTypeClassBase {
          name: "QRectF".to_string(),
          template_arguments: None,
        }),
      }
    );
    assert_eq!(&ffi1.ffi_type.to_cpp_code(None).unwrap(), "QRectF*");
    assert_eq!(ffi1.conversion, CppIndirectionChange::ReferenceToPointer);
  }
  assert!(!type1.needs_allocation_place_variants());
}

#[test]
fn class_mut_ptr() {
  let type1 = CppType {
    indirection: CppTypeIndirection::Ptr,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::Class(CppTypeClassBase {
      name: "QObject".to_string(),
      template_arguments: None,
    }),
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), true);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "QObject*");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "QObject");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "QObject"
  );
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Short).unwrap(),
    "QObject"
  );
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Full).unwrap(),
    "QObject_ptr"
  );
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
}

#[test]
fn class_with_template_args() {
  let args = Some(vec![
    CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
      base: CppTypeBase::Class(CppTypeClassBase {
        name: "QString".to_string(),
        template_arguments: None,
      }),
    },
  ]);
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::Class(CppTypeClassBase {
      name: "QVector".to_string(),
      template_arguments: args.clone(),
    }),
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), true);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "QVector< QString >");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "QVector< QString >");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "QVector_QString"
  );
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Short).unwrap(),
    "QVector_QString"
  );
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Full).unwrap(),
    "QVector_QString"
  );

  let ffi_return_type = type1.to_cpp_ffi_type(CppTypeRole::ReturnType).unwrap();
  assert_eq!(&ffi_return_type.original_type, &type1);
  assert_eq!(
    &ffi_return_type.ffi_type,
    &CppType {
      indirection: CppTypeIndirection::Ptr,
      is_const: false,
      is_const2: false,
      base: CppTypeBase::Class(CppTypeClassBase {
        name: "QVector".to_string(),
        template_arguments: args.clone(),
      }),
    }
  );
  assert_eq!(
    &ffi_return_type.ffi_type.to_cpp_code(None).unwrap(),
    "QVector< QString >*"
  );
  assert_eq!(
    ffi_return_type.conversion,
    CppIndirectionChange::ValueToPointer
  );

  let ffi_arg = type1.to_cpp_ffi_type(CppTypeRole::NotReturnType).unwrap();
  assert_eq!(&ffi_arg.original_type, &type1);
  assert_eq!(
    &ffi_arg.ffi_type,
    &CppType {
      indirection: CppTypeIndirection::Ptr,
      is_const: true,
      is_const2: false,
      base: CppTypeBase::Class(CppTypeClassBase {
        name: "QVector".to_string(),
        template_arguments: args.clone(),
      }),
    }
  );
  assert_eq!(
    &ffi_arg.ffi_type.to_cpp_code(None).unwrap(),
    "const QVector< QString >*"
  );
  assert_eq!(ffi_arg.conversion, CppIndirectionChange::ValueToPointer);
  assert!(type1.needs_allocation_place_variants());
}

#[test]
fn nested_template_cpp_code() {
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::Class(CppTypeClassBase {
      name: "QHash".to_string(),
      template_arguments: Some(vec![
        CppType {
          indirection: CppTypeIndirection::None,
          is_const: false,
          is_const2: false,
          base: CppTypeBase::Class(CppTypeClassBase {
            name: "QString".to_string(),
            template_arguments: None,
          }),
        },
        CppType {
          indirection: CppTypeIndirection::None,
          is_const: false,
          is_const2: false,
          base: CppTypeBase::Class(CppTypeClassBase {
            name: "QList".to_string(),
            template_arguments: Some(vec![
              CppType {
                indirection: CppTypeIndirection::None,
                is_const: false,
                is_const2: false,
                base: CppTypeBase::Class(CppTypeClassBase {
                  name: "QString".to_string(),
                  template_arguments: None,
                }),
              },
            ]),
          }),
        },
      ]),
    }),
  };
  let code = type1.to_cpp_code(None).unwrap();
  assert_eq!(&code, "QHash< QString, QList< QString > >");
  assert!(!code.contains(">>"));
  assert!(!code.contains("<<"));
}

#[test]
fn qflags() {
  let args = Some(vec![
    CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
      base: CppTypeBase::Class(CppTypeClassBase {
        name: "Qt::AlignmentFlag".to_string(),
        template_arguments: None,
      }),
    },
  ]);
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::Class(CppTypeClassBase {
      name: "QFlags".to_string(),
      template_arguments: args.clone(),
    }),
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), true);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(
    type1.to_cpp_code(None).unwrap(),
    "QFlags< Qt::AlignmentFlag >"
  );
  assert_eq!(
    type1.base.to_cpp_code(None).unwrap(),
    "QFlags< Qt::AlignmentFlag >"
  );
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "QFlags_Qt_AlignmentFlag"
  );
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Short).unwrap(),
    "QFlags_Qt_AlignmentFlag"
  );
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Full).unwrap(),
    "QFlags_Qt_AlignmentFlag"
  );

  for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
    let ffi_type = type1.to_cpp_ffi_type(role.clone()).unwrap();
    assert_eq!(&ffi_type.original_type, &type1);
    assert_eq!(
      &ffi_type.ffi_type,
      &CppType {
        indirection: CppTypeIndirection::None,
        is_const: false,
        is_const2: false,
        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::UInt),
      }
    );
    assert_eq!(
      &ffi_type.ffi_type.to_cpp_code(None).unwrap(),
      "unsigned int"
    );
    assert_eq!(ffi_type.conversion, CppIndirectionChange::QFlagsToUInt);
  }
  assert!(!type1.needs_allocation_place_variants());
}

fn create_template_parameter_type() -> CppType {
  CppType {
    indirection: CppTypeIndirection::Ptr,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::TemplateParameter {
      nested_level: 0,
      index: 0,
    },
  }
}

#[test]
fn template_parameter() {
  let type1 = create_template_parameter_type();
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), true);
  assert!(type1.to_cpp_code(None).is_err());
  assert!(type1.base.to_cpp_code(None).is_err());
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.to_cpp_ffi_type(CppTypeRole::NotReturnType).is_err());
  assert!(type1.to_cpp_ffi_type(CppTypeRole::ReturnType).is_err());
  assert!(!type1.needs_allocation_place_variants());

  assert!(type1.base.caption(TypeCaptionStrategy::Short).is_err());
  assert!(type1.caption(TypeCaptionStrategy::Short).is_err());
  assert!(type1.caption(TypeCaptionStrategy::Full).is_err());
}

#[test]
fn function1() {
  let type1 = CppType {
    is_const: false,
    is_const2: false,
    indirection: CppTypeIndirection::None,
    base: CppTypeBase::FunctionPointer(CppFunctionPointerType {
      allows_variadic_arguments: false,
      return_type: Box::new(CppType {
        indirection: CppTypeIndirection::None,
        is_const: false,
        is_const2: false,
        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
      }),
      arguments: vec![
        CppType {
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
        },
      ],
    }),
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), false);
  let name = "my_name".to_string();
  assert_eq!(
    type1.base.to_cpp_code(Some(&name)).unwrap(),
    "int (*my_name)(int, bool*)"
  );
  assert_eq!(
    type1.to_cpp_code(Some(&name)).unwrap(),
    type1.base.to_cpp_code(Some(&name)).unwrap()
  );
  assert!(type1.to_cpp_code(None).is_err());
  assert!(type1.base.to_cpp_code(None).is_err());
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Short).unwrap(),
    "func"
  );
  assert_eq!(
    type1.base.caption(TypeCaptionStrategy::Full).unwrap(),
    "int_func_int_bool_ptr"
  );
  assert_eq!(type1.caption(TypeCaptionStrategy::Short).unwrap(), "func");
  assert_eq!(
    type1.caption(TypeCaptionStrategy::Full).unwrap(),
    "int_func_int_bool_ptr"
  );
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
}

#[test]
fn instantiate1() {
  let type1 = CppType {
    indirection: CppTypeIndirection::Ref,
    is_const: true,
    is_const2: false,
    base: CppTypeBase::TemplateParameter {
      nested_level: 0,
      index: 0,
    },
  };
  let type2 = CppType {
    indirection: CppTypeIndirection::Ptr,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
  };
  let r = type1.instantiate(0, &[type2]).unwrap();
  assert_eq!(
    r.base,
    CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool)
  );
  assert_eq!(r.indirection, CppTypeIndirection::PtrRef);
  assert_eq!(r.is_const, false);
  assert_eq!(r.is_const2, true);
}
