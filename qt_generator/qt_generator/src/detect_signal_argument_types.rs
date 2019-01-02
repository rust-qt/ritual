use cpp_to_rust_generator::common::errors::Result;
use cpp_to_rust_generator::common::log;
use cpp_to_rust_generator::common::string_utils::JoinWithSeparator;
use cpp_to_rust_generator::database::CppItemData;
use cpp_to_rust_generator::database::DatabaseItemSource;
use cpp_to_rust_generator::processor::ProcessorData;
use std::collections::HashSet;

pub fn detect_signal_argument_types(data: ProcessorData) -> Result<()> {
    let mut all_types = HashSet::new();
    for method in data
        .current_database
        .items
        .iter()
        .filter_map(|i| i.cpp_data.as_function_ref())
    {
        if let Some(ref method_info) = method.member {
            if method_info.is_signal {
                let types: Vec<_> = method
                    .arguments
                    .iter()
                    .map(|x| x.argument_type.clone())
                    .collect();
                if !all_types.contains(&types)
                    && !data
                        .all_items()
                        .iter()
                        .filter_map(|i| i.cpp_data.as_signal_arguments_ref())
                        .any(|d| d == &types[..])
                {
                    all_types.insert(types);
                }
            }
        }
    }

    let mut types_with_omitted_args = HashSet::new();
    for t in &all_types {
        let mut types = t.clone();
        while let Some(_) = types.pop() {
            if !data
                .all_items()
                .iter()
                .filter_map(|i| i.cpp_data.as_signal_arguments_ref())
                .any(|d| d == &types[..])
            {
                types_with_omitted_args.insert(types.clone());
            }
        }
    }
    all_types.extend(types_with_omitted_args.into_iter());

    log::llog(log::DebugSignals, || "Signal argument types:");
    for t in &all_types {
        log::llog(log::DebugSignals, || {
            format!(
                "  ({})",
                t.iter().map(|x| x.to_cpp_pseudo_code()).join(", ")
            )
        });
    }
    for item in all_types {
        data.current_database.add_cpp_data(
            DatabaseItemSource::QtSignalArguments,
            CppItemData::QtSignalArguments(item),
        );
    }
    Ok(())
}
