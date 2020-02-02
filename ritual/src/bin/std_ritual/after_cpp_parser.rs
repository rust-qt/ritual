use ritual::cpp_data::{
    CppItem, CppPath, CppPathItem, CppTypeDeclaration, CppTypeDeclarationKind, CppVisibility,
};
use ritual::cpp_function::{
    CppFunction, CppFunctionArgument, CppFunctionKind, CppFunctionMemberData,
};
use ritual::cpp_type::{
    CppBuiltInNumericType, CppSpecificNumericType, CppSpecificNumericTypeKind,
    CppTemplateParameter, CppType,
};
use ritual::processor::ProcessorData;
use ritual_common::errors::Result;

fn add_vector_from_pointers_functions(data: &mut ProcessorData<'_>) -> Result<()> {
    let t = CppType::TemplateParameter(CppTemplateParameter {
        name: "_Tp".into(),
        nested_level: 0,
        index: 0,
    });
    let allocator_type = CppType::Class(CppPath::from_good_str("std").join(CppPathItem {
        name: "allocator".into(),
        template_arguments: Some(vec![t.clone()]),
    }));
    data.add_cpp_item(
        None,
        CppItem::Function(CppFunction {
            path: CppPath::from_good_str("std")
                .join(CppPathItem {
                    name: "vector".into(),
                    template_arguments: Some(vec![
                        t.clone(),
                        CppType::TemplateParameter(CppTemplateParameter {
                            name: "_Alloc".into(),
                            nested_level: 0,
                            index: 1,
                        }),
                    ]),
                })
                .join(CppPathItem::from_good_str("vector")),
            member: Some(CppFunctionMemberData {
                kind: CppFunctionKind::Constructor,
                is_virtual: false,
                is_pure_virtual: false,
                is_const: false,
                is_static: false,
                visibility: CppVisibility::Public,
                is_signal: false,
                is_slot: false,
            }),
            operator: None,
            return_type: CppType::Void,
            arguments: vec![
                CppFunctionArgument {
                    name: "first".into(),
                    argument_type: CppType::new_pointer(true, t.clone()),
                    has_default_value: false,
                },
                CppFunctionArgument {
                    name: "last".into(),
                    argument_type: CppType::new_pointer(true, t.clone()),
                    has_default_value: false,
                },
                CppFunctionArgument {
                    name: "alloc".into(),
                    argument_type: allocator_type,
                    has_default_value: true,
                },
            ],
            allows_variadic_arguments: false,
            cast: None,
            declaration_code: None,
        }),
    )?;
    data.add_cpp_item(
        None,
        CppItem::Function(CppFunction {
            path: CppPath::from_good_str("std")
                .join(CppPathItem {
                    name: "vector".into(),
                    template_arguments: Some(vec![
                        t.clone(),
                        CppType::TemplateParameter(CppTemplateParameter {
                            name: "_Alloc".into(),
                            nested_level: 0,
                            index: 1,
                        }),
                    ]),
                })
                .join(CppPathItem::from_good_str("assign")),
            member: Some(CppFunctionMemberData {
                kind: CppFunctionKind::Regular,
                is_virtual: false,
                is_pure_virtual: false,
                is_const: false,
                is_static: false,
                visibility: CppVisibility::Public,
                is_signal: false,
                is_slot: false,
            }),
            operator: None,
            return_type: CppType::Void,
            arguments: vec![
                CppFunctionArgument {
                    name: "first".into(),
                    argument_type: CppType::new_pointer(true, t.clone()),
                    has_default_value: false,
                },
                CppFunctionArgument {
                    name: "last".into(),
                    argument_type: CppType::new_pointer(true, t.clone()),
                    has_default_value: false,
                },
            ],
            allows_variadic_arguments: false,
            cast: None,
            declaration_code: None,
        }),
    )?;
    Ok(())
}

fn add_vector_instantiations(data: &mut ProcessorData<'_>) -> Result<()> {
    let vector_instantiations = &[
        CppType::BuiltInNumeric(CppBuiltInNumericType::Bool),
        CppType::BuiltInNumeric(CppBuiltInNumericType::Char),
        CppType::BuiltInNumeric(CppBuiltInNumericType::SChar),
        CppType::BuiltInNumeric(CppBuiltInNumericType::UChar),
        CppType::BuiltInNumeric(CppBuiltInNumericType::WChar),
        CppType::BuiltInNumeric(CppBuiltInNumericType::Char16),
        CppType::BuiltInNumeric(CppBuiltInNumericType::Char32),
        CppType::BuiltInNumeric(CppBuiltInNumericType::Short),
        CppType::BuiltInNumeric(CppBuiltInNumericType::UShort),
        CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
        CppType::BuiltInNumeric(CppBuiltInNumericType::UInt),
        CppType::BuiltInNumeric(CppBuiltInNumericType::Long),
        CppType::BuiltInNumeric(CppBuiltInNumericType::ULong),
        CppType::BuiltInNumeric(CppBuiltInNumericType::LongLong),
        CppType::BuiltInNumeric(CppBuiltInNumericType::ULongLong),
        CppType::BuiltInNumeric(CppBuiltInNumericType::Float),
        CppType::BuiltInNumeric(CppBuiltInNumericType::Double),
        CppType::SpecificNumeric(CppSpecificNumericType {
            path: CppPath::from_good_str("int8_t"),
            bits: 8,
            kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
        }),
        CppType::SpecificNumeric(CppSpecificNumericType {
            path: CppPath::from_good_str("uint8_t"),
            bits: 8,
            kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
        }),
        CppType::SpecificNumeric(CppSpecificNumericType {
            path: CppPath::from_good_str("int16_t"),
            bits: 16,
            kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
        }),
        CppType::SpecificNumeric(CppSpecificNumericType {
            path: CppPath::from_good_str("uint16_t"),
            bits: 16,
            kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
        }),
        CppType::SpecificNumeric(CppSpecificNumericType {
            path: CppPath::from_good_str("int32_t"),
            bits: 32,
            kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
        }),
        CppType::SpecificNumeric(CppSpecificNumericType {
            path: CppPath::from_good_str("uint32_t"),
            bits: 32,
            kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
        }),
        CppType::SpecificNumeric(CppSpecificNumericType {
            path: CppPath::from_good_str("int64_t"),
            bits: 64,
            kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
        }),
        CppType::SpecificNumeric(CppSpecificNumericType {
            path: CppPath::from_good_str("uint64_t"),
            bits: 64,
            kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
        }),
        CppType::PointerSizedInteger {
            path: CppPath::from_good_str("size_t"),
            is_signed: false,
        },
        CppType::new_pointer(false, CppType::Void),
    ];

    for arg in vector_instantiations {
        let allocator_type = CppType::Class(CppPath::from_good_str("std").join(CppPathItem {
            name: "allocator".into(),
            template_arguments: Some(vec![arg.clone()]),
        }));
        data.add_cpp_item(
            None,
            CppItem::Type(CppTypeDeclaration {
                kind: CppTypeDeclarationKind::Class,
                path: CppPath::from_good_str("std").join(CppPathItem {
                    name: "vector".into(),
                    template_arguments: Some(vec![arg.clone(), allocator_type]),
                }),
            }),
        )?;
    }
    Ok(())
}

pub fn hook(data: &mut ProcessorData<'_>) -> Result<()> {
    add_vector_from_pointers_functions(data)?;
    add_vector_instantiations(data)?;
    Ok(())
}
