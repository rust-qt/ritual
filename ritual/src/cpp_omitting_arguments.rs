use crate::cpp_data::CppItem;
use crate::database::ItemWithSource;
use crate::processor::ProcessorData;
use ritual_common::errors::Result;

pub fn run(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut results = Vec::new();
    for item in data.current_database.cpp_items() {
        let function = if let Some(f) = item.item.as_function_ref() {
            f
        } else {
            continue;
        };

        if function.arguments.iter().any(|arg| arg.has_default_value) {
            let mut function_copy = function.clone();
            while let Some(arg) = function_copy.arguments.pop() {
                if !arg.has_default_value {
                    break;
                }
                results.push(ItemWithSource {
                    value: function_copy.clone(),
                    source_id: item.id,
                });
            }
        }
    }

    for item in results {
        data.current_database
            .add_cpp_item(Some(item.source_id), CppItem::Function(item.value))?;
    }

    Ok(())
}
