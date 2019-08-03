use itertools::Itertools;
use log::trace;
use ritual::cpp_type::CppType;
use ritual::processor::ProcessorData;
use ritual_common::errors::Result;
use std::collections::HashSet;

pub fn detect_signal_argument_types(data: &mut ProcessorData<'_>) -> Result<HashSet<Vec<CppType>>> {
    let mut all_types = HashSet::new();
    for method in data
        .current_database
        .cpp_items()
        .filter_map(|i| i.item.as_function_ref())
    {
        if let Some(method_info) = &method.member {
            if method_info.is_signal {
                let types = method
                    .arguments
                    .iter()
                    .map(|x| x.argument_type.clone())
                    .collect_vec();
                all_types.insert(types);
            }
        }
    }

    let mut types_with_omitted_args = HashSet::new();
    for t in &all_types {
        let mut types = t.clone();
        while let Some(_) = types.pop() {
            types_with_omitted_args.insert(types.clone());
        }
    }
    all_types.extend(types_with_omitted_args.into_iter());

    trace!("Signal argument types:");
    for t in &all_types {
        trace!(
            "* ({})",
            t.iter().map(CppType::to_cpp_pseudo_code).join(", ")
        );
    }

    Ok(all_types)
}
