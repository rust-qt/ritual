use cpp_type::{CppType, CppTypeRole, CppTypeIndirection, CppTypeBase, CppBuiltInNumericType,
               CppSpecificNumericTypeKind};
use caption_strategy::TypeCaptionStrategy;
use cpp_ffi_data::IndirectionChange;

fn assert_type_to_ffi_unchanged(t: &CppType) {
  for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
    let ffi1 = t.to_cpp_ffi_type(role.clone()).unwrap();
    assert_eq!(&ffi1.original_type, t);
    assert_eq!(&ffi1.ffi_type, t);
    assert_eq!(ffi1.conversion, IndirectionChange::NoChange);
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
  assert_eq!(type1.base.caption(), "void");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "void");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "void");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), None);
}

#[test]
fn void_ptr() {
  let type1 = CppType {
    indirection: CppTypeIndirection::Ptr,
    is_const: false,
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
  assert_eq!(type1.base.caption(), "void");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "void");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "void_ptr");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), None);
}

#[test]
fn int() {
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
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
  assert_eq!(type1.base.caption(), "int");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "int");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "int");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), None);
}

#[test]
fn bool_ptr() {
  let type1 = CppType {
    indirection: CppTypeIndirection::Ptr,
    is_const: false,
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
  assert_eq!(type1.base.caption(), "bool");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "bool");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "bool_ptr");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), None);
}

#[test]
fn char_ptr_ptr() {
  let type1 = CppType {
    indirection: CppTypeIndirection::PtrPtr,
    is_const: false,
    base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::CharS),
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "char**");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "char");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(type1.base.caption(), "char");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "char");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "char_ptr_ptr");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), None);
}

#[test]
fn qint64() {
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    base: CppTypeBase::SpecificNumeric {
      name: "qint64".to_string(),
      bits: 64,
      kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
    },
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "qint64");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "qint64");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(type1.base.caption(), "qint64");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "qint64");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "qint64");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), Some(&"qint64".to_string()));
}

#[test]
fn quintptr() {
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
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
  assert_eq!(type1.base.caption(), "quintptr");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "quintptr");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "quintptr");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), Some(&"quintptr".to_string()));
}

#[test]
fn enum1() {
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
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
  assert_eq!(type1.base.caption(), "Qt_CaseSensitivity");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short),
             "Qt_CaseSensitivity");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full),
             "Qt_CaseSensitivity");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(),
             Some(&"Qt::CaseSensitivity".to_string()));
}


#[test]
fn class_value() {
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    base: CppTypeBase::Class {
      name: "QPoint".to_string(),
      template_arguments: None,
    },
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), true);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "QPoint");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "QPoint");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(type1.base.caption(), "QPoint");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "QPoint");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "QPoint");

  let ffi_return_type = type1.to_cpp_ffi_type(CppTypeRole::ReturnType).unwrap();
  assert_eq!(&ffi_return_type.original_type, &type1);
  assert_eq!(&ffi_return_type.ffi_type,
             &CppType {
               indirection: CppTypeIndirection::Ptr,
               is_const: false,
               base: CppTypeBase::Class {
                 name: "QPoint".to_string(),
                 template_arguments: None,
               },
             });
  assert_eq!(&ffi_return_type.ffi_type.to_cpp_code(None).unwrap(),
             "QPoint*");
  assert_eq!(ffi_return_type.conversion,
             IndirectionChange::ValueToPointer);

  let ffi_arg = type1.to_cpp_ffi_type(CppTypeRole::NotReturnType).unwrap();
  assert_eq!(&ffi_arg.original_type, &type1);
  assert_eq!(&ffi_arg.ffi_type,
             &CppType {
               indirection: CppTypeIndirection::Ptr,
               is_const: true,
               base: CppTypeBase::Class {
                 name: "QPoint".to_string(),
                 template_arguments: None,
               },
             });
  assert_eq!(&ffi_arg.ffi_type.to_cpp_code(None).unwrap(),
             "const QPoint*");
  assert_eq!(ffi_arg.conversion, IndirectionChange::ValueToPointer);
  assert!(type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), Some(&"QPoint".to_string()));
}

#[test]
fn class_const_ref() {
  let type1 = CppType {
    indirection: CppTypeIndirection::Ref,
    is_const: true,
    base: CppTypeBase::Class {
      name: "QRectF".to_string(),
      template_arguments: None,
    },
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), true);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "const QRectF&");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "QRectF");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(type1.base.caption(), "QRectF");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "QRectF");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "const_QRectF_ref");

  for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
    let ffi1 = type1.to_cpp_ffi_type(role.clone()).unwrap();
    assert_eq!(&ffi1.original_type, &type1);
    assert_eq!(&ffi1.ffi_type,
               &CppType {
                 indirection: CppTypeIndirection::Ptr,
                 is_const: true,
                 base: CppTypeBase::Class {
                   name: "QRectF".to_string(),
                   template_arguments: None,
                 },
               });
    assert_eq!(&ffi1.ffi_type.to_cpp_code(None).unwrap(), "const QRectF*");
    assert_eq!(ffi1.conversion, IndirectionChange::ReferenceToPointer);
  }
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), Some(&"QRectF".to_string()));
}

#[test]
fn class_mut_ref() {
  let type1 = CppType {
    indirection: CppTypeIndirection::Ref,
    is_const: false,
    base: CppTypeBase::Class {
      name: "QRectF".to_string(),
      template_arguments: None,
    },
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), true);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "QRectF&");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "QRectF");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(type1.base.caption(), "QRectF");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "QRectF");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "QRectF_ref");

  for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
    let ffi1 = type1.to_cpp_ffi_type(role.clone()).unwrap();
    assert_eq!(&ffi1.original_type, &type1);
    assert_eq!(&ffi1.ffi_type,
               &CppType {
                 indirection: CppTypeIndirection::Ptr,
                 is_const: false,
                 base: CppTypeBase::Class {
                   name: "QRectF".to_string(),
                   template_arguments: None,
                 },
               });
    assert_eq!(&ffi1.ffi_type.to_cpp_code(None).unwrap(), "QRectF*");
    assert_eq!(ffi1.conversion, IndirectionChange::ReferenceToPointer);
  }
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), Some(&"QRectF".to_string()));
}

#[test]
fn class_mut_ptr() {
  let type1 = CppType {
    indirection: CppTypeIndirection::Ptr,
    is_const: false,
    base: CppTypeBase::Class {
      name: "QObject".to_string(),
      template_arguments: None,
    },
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), true);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "QObject*");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "QObject");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(type1.base.caption(), "QObject");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "QObject");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "QObject_ptr");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), Some(&"QObject".to_string()));
}

#[test]
fn class_with_template_args() {
  let args = Some(vec![CppType {
                         indirection: CppTypeIndirection::None,
                         is_const: false,
                         base: CppTypeBase::Class {
                           name: "QString".to_string(),
                           template_arguments: None,
                         },
                       }]);
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    base: CppTypeBase::Class {
      name: "QVector".to_string(),
      template_arguments: args.clone(),
    },
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), true);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(), "QVector< QString >");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(), "QVector< QString >");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(type1.base.caption(), "QVector_QString");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "QVector_QString");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "QVector_QString");

  let ffi_return_type = type1.to_cpp_ffi_type(CppTypeRole::ReturnType).unwrap();
  assert_eq!(&ffi_return_type.original_type, &type1);
  assert_eq!(&ffi_return_type.ffi_type,
             &CppType {
               indirection: CppTypeIndirection::Ptr,
               is_const: false,
               base: CppTypeBase::Class {
                 name: "QVector".to_string(),
                 template_arguments: args.clone(),
               },
             });
  assert_eq!(&ffi_return_type.ffi_type.to_cpp_code(None).unwrap(),
             "QVector< QString >*");
  assert_eq!(ffi_return_type.conversion,
             IndirectionChange::ValueToPointer);

  let ffi_arg = type1.to_cpp_ffi_type(CppTypeRole::NotReturnType).unwrap();
  assert_eq!(&ffi_arg.original_type, &type1);
  assert_eq!(&ffi_arg.ffi_type,
             &CppType {
               indirection: CppTypeIndirection::Ptr,
               is_const: true,
               base: CppTypeBase::Class {
                 name: "QVector".to_string(),
                 template_arguments: args.clone(),
               },
             });
  assert_eq!(&ffi_arg.ffi_type.to_cpp_code(None).unwrap(),
             "const QVector< QString >*");
  assert_eq!(ffi_arg.conversion, IndirectionChange::ValueToPointer);
  assert!(type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), Some(&"QVector".to_string()));
}

#[test]
fn nested_template_cpp_code() {
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    base: CppTypeBase::Class {
      name: "QHash".to_string(),
      template_arguments: Some(vec![CppType {
                                      indirection: CppTypeIndirection::None,
                                      is_const: false,
                                      base: CppTypeBase::Class {
                                        name: "QString".to_string(),
                                        template_arguments: None,
                                      },
                                    },
                                    CppType {
                                      indirection: CppTypeIndirection::None,
                                      is_const: false,
                                      base: CppTypeBase::Class {
                                        name: "QList".to_string(),
                                        template_arguments: Some(vec![CppType {
                                                                        indirection:
                                                                          CppTypeIndirection::None,
                                                                        is_const: false,
                                                                        base: CppTypeBase::Class {
                                                                          name: "QString"
                                                                            .to_string(),
                                                                          template_arguments: None,
                                                                        },
                                                                      }]),
                                      },
                                    }]),
    },
  };
  let code = type1.to_cpp_code(None).unwrap();
  assert_eq!(&code, "QHash< QString, QList< QString > >");
  assert!(!code.contains(">>"));
  assert!(!code.contains("<<"));
}

#[test]
fn qflags() {
  let args = Some(vec![CppType {
                         indirection: CppTypeIndirection::None,
                         is_const: false,
                         base: CppTypeBase::Class {
                           name: "Qt::AlignmentFlag".to_string(),
                           template_arguments: None,
                         },
                       }]);
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    base: CppTypeBase::Class {
      name: "QFlags".to_string(),
      template_arguments: args.clone(),
    },
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), true);
  assert_eq!(type1.base.is_template_parameter(), false);
  assert_eq!(type1.to_cpp_code(None).unwrap(),
             "QFlags< Qt::AlignmentFlag >");
  assert_eq!(type1.base.to_cpp_code(None).unwrap(),
             "QFlags< Qt::AlignmentFlag >");
  assert!(type1.to_cpp_code(Some(&String::new())).is_err());
  assert!(type1.base.to_cpp_code(Some(&String::new())).is_err());
  assert_eq!(type1.base.caption(), "QFlags_Qt_AlignmentFlag");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short),
             "QFlags_Qt_AlignmentFlag");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full),
             "QFlags_Qt_AlignmentFlag");

  for role in &[CppTypeRole::NotReturnType, CppTypeRole::ReturnType] {
    let ffi_type = type1.to_cpp_ffi_type(role.clone()).unwrap();
    assert_eq!(&ffi_type.original_type, &type1);
    assert_eq!(&ffi_type.ffi_type,
               &CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::UInt),
               });
    assert_eq!(&ffi_type.ffi_type.to_cpp_code(None).unwrap(),
               "unsigned int");
    assert_eq!(ffi_type.conversion, IndirectionChange::QFlagsToUInt);
  }
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), Some(&"QFlags".to_string()));
}

fn create_template_parameter_type() -> CppType {
  CppType {
    indirection: CppTypeIndirection::Ptr,
    is_const: false,
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
  assert_eq!(type1.base.maybe_name(), None);
}

#[test]
#[should_panic]
fn template_parameter2() {
  let type1 = create_template_parameter_type();
  type1.base.caption();
}

#[test]
#[should_panic]
fn template_parameter3() {
  let type1 = create_template_parameter_type();
  type1.caption(TypeCaptionStrategy::Short);
}

#[test]
#[should_panic]
fn template_parameter4() {
  let type1 = create_template_parameter_type();
  type1.caption(TypeCaptionStrategy::Full);
}

#[test]
fn function1() {
  let type1 = CppType {
    is_const: false,
    indirection: CppTypeIndirection::None,
    base: CppTypeBase::FunctionPointer {
      allows_variable_arguments: false,
      return_type: Box::new(CppType {
        indirection: CppTypeIndirection::None,
        is_const: false,
        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
      }),
      arguments: vec![CppType {
                        indirection: CppTypeIndirection::None,
                        is_const: false,
                        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
                      },
                      CppType {
                        indirection: CppTypeIndirection::Ptr,
                        is_const: false,
                        base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
                      }],
    },
  };
  assert_eq!(type1.is_void(), false);
  assert_eq!(type1.base.is_void(), false);
  assert_eq!(type1.base.is_class(), false);
  assert_eq!(type1.base.is_template_parameter(), false);
  let name = "my_name".to_string();
  assert_eq!(type1.base.to_cpp_code(Some(&name)).unwrap(),
             "int (*my_name)(int, bool*)");
  assert_eq!(type1.to_cpp_code(Some(&name)).unwrap(),
             type1.base.to_cpp_code(Some(&name)).unwrap());
  assert!(type1.to_cpp_code(None).is_err());
  assert!(type1.base.to_cpp_code(None).is_err());
  assert_eq!(type1.base.caption(), "func");
  assert_eq!(type1.caption(TypeCaptionStrategy::Short), "func");
  assert_eq!(type1.caption(TypeCaptionStrategy::Full), "func");
  assert_type_to_ffi_unchanged(&type1);
  assert!(!type1.needs_allocation_place_variants());
  assert_eq!(type1.base.maybe_name(), None);
}
