use crate::cpp_data::{CppItem, CppPathItem, CppVisibility};
use crate::cpp_function::{
    CppFunction, CppFunctionArgument, CppFunctionDoc, CppFunctionKind, CppFunctionMemberData,
};
use crate::cpp_operator::CppOperator;
use crate::cpp_type::CppType;
use crate::processor::ProcessorData;
use ritual_common::errors::Result;

/// Adds constructors and destructors for every class that does not have explicitly
/// defined constructor or destructor, allowing to create wrappings for
/// constructors and destructors implicitly available in C++.
pub fn run(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut methods = Vec::new();
    for type1 in data.current_database.cpp_items() {
        if let CppItem::Type(declaration) = &type1.item {
            if declaration.kind.is_class() {
                let class_path = &declaration.path;

                let destructor = CppFunction {
                    path: class_path.join(CppPathItem::from_good_str(&format!(
                        "~{}",
                        class_path.last().name
                    ))),
                    member: Some(CppFunctionMemberData {
                        is_virtual: false, // the destructor can actually be virtual but we don't care about it here
                        is_pure_virtual: false,
                        is_const: false,
                        is_static: false,
                        visibility: CppVisibility::Public,
                        is_signal: false,
                        is_slot: false,
                        kind: CppFunctionKind::Destructor,
                    }),
                    operator: None,
                    return_type: CppType::Void,
                    arguments: vec![],
                    allows_variadic_arguments: false,
                    cast: None,
                    declaration_code: None,
                    doc: CppFunctionDoc::default(),
                };
                methods.push((type1.source_id, destructor));

                let default_constructor = CppFunction {
                    path: declaration
                        .path
                        .join(CppPathItem::from_good_str(&class_path.last().name)),
                    member: Some(CppFunctionMemberData {
                        is_virtual: false,
                        is_pure_virtual: false,
                        is_const: false,
                        is_static: false,
                        visibility: CppVisibility::Public,
                        is_signal: false,
                        is_slot: false,
                        kind: CppFunctionKind::Constructor,
                    }),
                    operator: None,
                    return_type: CppType::Void,
                    arguments: vec![],
                    allows_variadic_arguments: false,
                    declaration_code: None,
                    cast: None,
                    doc: CppFunctionDoc::default(),
                };
                methods.push((type1.source_id, default_constructor));

                let copy_arg = CppFunctionArgument {
                    argument_type: CppType::new_reference(true, CppType::Class(class_path.clone())),
                    name: "other".to_string(),
                    has_default_value: false,
                };

                let copy_constructor = CppFunction {
                    path: declaration
                        .path
                        .join(CppPathItem::from_good_str(&class_path.last().name)),
                    member: Some(CppFunctionMemberData {
                        is_virtual: false,
                        is_pure_virtual: false,
                        is_const: false,
                        is_static: false,
                        visibility: CppVisibility::Public,
                        is_signal: false,
                        is_slot: false,
                        kind: CppFunctionKind::Constructor,
                    }),
                    operator: None,
                    return_type: CppType::Void,
                    arguments: vec![copy_arg.clone()],
                    allows_variadic_arguments: false,
                    cast: None,
                    declaration_code: None,
                    doc: CppFunctionDoc::default(),
                };
                methods.push((type1.source_id, copy_constructor));

                let assignment_operator = CppFunction {
                    path: declaration
                        .path
                        .join(CppPathItem::from_good_str("operator=")),
                    member: Some(CppFunctionMemberData {
                        is_virtual: false,
                        is_pure_virtual: false,
                        is_const: false,
                        is_static: false,
                        visibility: CppVisibility::Public,
                        is_signal: false,
                        is_slot: false,
                        kind: CppFunctionKind::Regular,
                    }),
                    operator: Some(CppOperator::Assignment),
                    return_type: CppType::new_reference(false, CppType::Class(class_path.clone())),
                    arguments: vec![copy_arg],
                    allows_variadic_arguments: false,
                    cast: None,
                    declaration_code: None,
                    doc: CppFunctionDoc::default(),
                };
                methods.push((type1.source_id, assignment_operator));
            }
        }
    }
    for (source_ffi_item, method) in methods {
        data.current_database
            .add_cpp_item(source_ffi_item, CppItem::Function(method))?;
    }
    Ok(())
}
