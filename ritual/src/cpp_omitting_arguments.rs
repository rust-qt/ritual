use crate::cpp_data::CppItem;
use crate::processor::ProcessorData;
use ritual_common::errors::Result;

pub fn run(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut results = Vec::new();
    for (function, source_ffi_item) in data.current_database.cpp_items().iter().filter_map(|item| {
        item.item
            .as_function_ref()
            .map(|f| (f, item.source_ffi_item))
    }) {
        if function.arguments.iter().any(|arg| arg.has_default_value) {
            let mut function_copy = function.clone();
            while let Some(arg) = function_copy.arguments.pop() {
                if !arg.has_default_value {
                    break;
                }
                function_copy.doc.arguments_before_omitting = Some(function.arguments.clone());
                results.push((function_copy.clone(), source_ffi_item));
            }
        }
    }

    for (function, source_ffi_item) in results {
        data.current_database
            .add_cpp_item(source_ffi_item, CppItem::Function(function));
    }

    Ok(())
}
