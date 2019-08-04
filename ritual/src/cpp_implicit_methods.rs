use crate::cpp_data::{CppItem, CppPathItem, CppVisibility};
use crate::cpp_function::{
    CppFunction, CppFunctionArgument, CppFunctionKind, CppFunctionMemberData,
};
use crate::cpp_operator::CppOperator;
use crate::cpp_type::CppType;
use crate::database::ItemWithSource;
use crate::processor::ProcessorData;
use ritual_common::errors::Result;

/// Adds constructors and destructors for every class that does not have explicitly
/// defined constructor or destructor, allowing to create wrappings for
/// constructors and destructors implicitly available in C++.
pub fn run(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut methods = Vec::new();

    let classes = data
        .current_database
        .cpp_items()
        .filter_map(|item| item.filter_map(|item| item.as_type_ref()))
        .filter(|item| item.item.kind.is_class());

    for type1 in classes {
        if type1.item.kind.is_class() {
            let class_path = &type1.item.path;

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
            };
            methods.push(ItemWithSource {
                source_id: type1.id,
                value: destructor,
            });

            let default_constructor = CppFunction {
                path: type1
                    .item
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
            };
            methods.push(ItemWithSource {
                source_id: type1.id,
                value: default_constructor,
            });

            let copy_arg = CppFunctionArgument {
                argument_type: CppType::new_reference(true, CppType::Class(class_path.clone())),
                name: "other".to_string(),
                has_default_value: false,
            };

            let copy_constructor = CppFunction {
                path: type1
                    .item
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
            };
            methods.push(ItemWithSource {
                source_id: type1.id,
                value: copy_constructor,
            });

            let assignment_operator = CppFunction {
                path: type1
                    .item
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
            };
            methods.push(ItemWithSource {
                source_id: type1.id,
                value: assignment_operator,
            });
        }
    }
    for item in methods {
        data.current_database
            .add_cpp_item(Some(item.source_id), CppItem::Function(item.value))?;
    }
    Ok(())
}
