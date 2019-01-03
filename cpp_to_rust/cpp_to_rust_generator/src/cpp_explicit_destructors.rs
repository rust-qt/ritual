use crate::common::errors::Result;
use crate::cpp_data::CppName;
use crate::cpp_data::CppTypeDataKind;
use crate::cpp_data::CppVisibility;
use crate::cpp_function::CppFunction;
use crate::cpp_function::CppFunctionKind;
use crate::cpp_function::CppFunctionMemberData;
use crate::cpp_type::CppType;
use crate::database::CppItemData;
use crate::database::DatabaseItemSource;
use crate::processor::ProcessingStep;
use crate::processor::ProcessorData;

pub fn add_explicit_destructors_step() -> ProcessingStep {
    ProcessingStep::new(
        "add_explicit_destructors",
        vec!["cpp_parser".to_string()],
        add_explicit_destructors,
    )
}

/// Adds destructors for every class that does not have explicitly
/// defined destructor, allowing to create wrappings for
/// destructors implicitly available in C++.
fn add_explicit_destructors(data: &mut ProcessorData) -> Result<()> {
    let mut methods = Vec::new();
    for type1 in &data.current_database.items {
        if let CppItemData::Type(ref type1) = type1.cpp_data {
            if let CppTypeDataKind::Class { ref type_base } = type1.kind {
                let class_name = &type1.name;
                let found_destructor = data
                    .current_database
                    .items
                    .iter()
                    .filter_map(|item| item.cpp_data.as_function_ref())
                    .any(|m| m.is_destructor() && m.class_name() == Some(class_name));
                if !found_destructor {
                    methods.push(CppFunction {
                        name: CppName::from_one_part(format!("~{}", class_name)),
                        member: Some(CppFunctionMemberData {
                            class_type: type_base.clone(),
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
                        template_arguments: None,
                        declaration_code: None,
                        doc: None,
                    });
                }
            }
        }
    }
    for method in methods {
        data.current_database.add_cpp_data(
            DatabaseItemSource::ImplicitDestructor,
            CppItemData::Function(method),
        );
    }
    Ok(())
}
