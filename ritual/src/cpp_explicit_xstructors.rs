use crate::cpp_data::{CppPathItem, CppVisibility};
use crate::cpp_function::{CppFunction, CppFunctionKind, CppFunctionMemberData};
use crate::cpp_type::CppType;
use crate::database::{CppItemData, DatabaseItemSource};
use crate::processor::ProcessorData;
use ritual_common::errors::Result;

/// Adds constructors and destructors for every class that does not have explicitly
/// defined constructor or destructor, allowing to create wrappings for
/// constructors and destructors implicitly available in C++.
pub fn run(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut methods = Vec::new();
    for type1 in data.current_database.cpp_items() {
        if let CppItemData::Type(declaration) = &type1.cpp_data {
            if declaration.kind.is_class() {
                let class_path = &declaration.path;
                let found_destructor = data
                    .current_database
                    .cpp_items()
                    .iter()
                    .filter_map(|item| item.cpp_data.as_function_ref())
                    .any(|m| m.is_destructor() && m.class_type().ok().as_ref() == Some(class_path));
                if !found_destructor {
                    let function = CppFunction {
                        path: declaration.path.join(CppPathItem::from_good_str(&format!(
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
                        declaration_code: None,
                        doc: None,
                    };
                    methods.push((type1.source_ffi_item, function));
                }

                let found_constructor = data
                    .current_database
                    .cpp_items()
                    .iter()
                    .filter_map(|item| item.cpp_data.as_function_ref())
                    .any(|m| {
                        m.is_constructor() && m.class_type().ok().as_ref() == Some(class_path)
                    });
                if !found_constructor {
                    let function = CppFunction {
                        path: declaration
                            .path
                            .join(CppPathItem::from_good_str(&declaration.path.last().name)),
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
                        doc: None,
                    };
                    methods.push((type1.source_ffi_item, function));
                }
            }
        }
    }
    for (source_ffi_item, method) in methods {
        data.current_database.add_cpp_item(
            DatabaseItemSource::ImplicitDestructor,
            source_ffi_item,
            CppItemData::Function(method),
        );
    }
    Ok(())
}
