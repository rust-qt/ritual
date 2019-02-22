use itertools::Itertools;
use log::trace;
use ritual::database::CppItemData;
use ritual::database::DatabaseItemSource;
use ritual::processor::ProcessorData;
use ritual_common::errors::Result;
use std::collections::HashSet;

pub fn detect_signal_argument_types(data: &mut ProcessorData) -> Result<()> {
    let mut all_types = HashSet::new();
    for method in data
        .current_database
        .cpp_items
        .iter()
        .filter_map(|i| i.cpp_data.as_function_ref())
    {
        if let Some(method_info) = &method.member {
            if method_info.is_signal {
                let types = method
                    .arguments
                    .iter()
                    .map(|x| x.argument_type.clone())
                    .collect_vec();
                if !all_types.contains(&types)
                    && !data
                        .all_items()
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
                .filter_map(|i| i.cpp_data.as_signal_arguments_ref())
                .any(|d| d == &types[..])
            {
                types_with_omitted_args.insert(types.clone());
            }
        }
    }
    all_types.extend(types_with_omitted_args.into_iter());

    trace!("[DebugSignals] Signal argument types:");
    for t in &all_types {
        trace!(
            "[DebugSignals] * ({})",
            t.iter().map(|x| x.to_cpp_pseudo_code()).join(", ")
        );
    }
    for item in all_types {
        data.current_database.add_cpp_data(
            DatabaseItemSource::QtSignalArguments,
            CppItemData::QtSignalArguments(item),
        );
    }
    Ok(())
}
