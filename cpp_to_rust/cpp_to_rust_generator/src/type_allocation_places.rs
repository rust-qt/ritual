use common::errors::Result;
use common::log;
use cpp_type::CppClassType;
use cpp_type::CppPointerLikeTypeKind;
use cpp_type::CppType;
use new_impl::processor::ProcessingStep;
use new_impl::processor::ProcessorData;
use std::collections::HashMap;

pub fn choose_allocation_places_step() -> ProcessingStep {
    ProcessingStep::new(
        "choose_allocation_places",
        Vec::new(),
        choose_allocation_places,
    )
}

/// Detects the preferred type allocation place for each type based on
/// API of all known methods. Doesn't actually change the data,
/// only suggests stack allocated types for manual configuration.
fn choose_allocation_places(mut data: ProcessorData) -> Result<()> {
    log::status("Detecting type allocation places");

    #[derive(Default)]
    struct TypeStats {
        // has_derived_classes: bool,
        has_virtual_methods: bool,
        pointers_count: usize,
        not_pointers_count: usize,
    };
    fn check_type(
        cpp_type: &CppType,
        is_behind_pointer: bool,
        data: &mut HashMap<String, TypeStats>,
    ) {
        match cpp_type {
            CppType::Class(CppClassType {
                ref name,
                ref template_arguments,
            }) => {
                if !data.contains_key(name) {
                    data.insert(name.clone(), TypeStats::default());
                }
                if is_behind_pointer {
                    data.get_mut(name).unwrap().pointers_count += 1;
                } else {
                    data.get_mut(name).unwrap().not_pointers_count += 1;
                }
                if let Some(ref args) = *template_arguments {
                    for arg in args {
                        check_type(arg, false, data);
                    }
                }
            }
            CppType::PointerLike {
                ref kind,
                ref target,
                ..
            } => {
                check_type(target, *kind == CppPointerLikeTypeKind::Pointer, data);
            }
            _ => {}
        }
    }

    let mut data_map = HashMap::new();
    for type1 in data
        .current_database
        .items
        .iter()
        .filter_map(|i| i.cpp_data.as_type_ref())
    {
        if data
            .current_database
            .items
            .iter()
            .filter_map(|i| i.cpp_data.as_function_ref())
            .any(|m| m.class_name() == Some(&type1.name) && m.is_virtual())
        {
            if !data_map.contains_key(&type1.name) {
                data_map.insert(type1.name.clone(), TypeStats::default());
            }
            data_map.get_mut(&type1.name).unwrap().has_virtual_methods = true;
        }
    }
    for method in data
        .current_database
        .items
        .iter()
        .filter_map(|i| i.cpp_data.as_function_ref())
    {
        for type1 in method.all_involved_types() {
            check_type(&type1, false, &mut data_map);
        }
    }
    data.html_logger.add_header(&[
        "Name",
        "has_virtual_methods",
        "pointers_count",
        "not_pointers_count",
    ])?;

    for (name, stats) in &data_map {
        data.html_logger.add(
            &[
                name.clone(),
                format!("{}", stats.has_virtual_methods),
                format!("{}", stats.pointers_count),
                format!("{}", stats.not_pointers_count),
            ],
            "type_allocation_places_stats",
        )?;
    }

    let mut stack_allocated = Vec::new();
    let mut heap_allocated = Vec::new();

    for type1 in data
        .current_database
        .items
        .iter()
        .filter_map(|i| i.cpp_data.as_type_ref())
    {
        if !type1.kind.is_class() {
            continue;
        }
        let name = &type1.name;
        // TODO: add `heap_allocated_types` to `Config` just for suppressing the output of this function
        if data
            .config
            .rust_stack_allocated_types()
            .iter()
            .any(|n| n == name)
        {
            continue;
        }
        let suggest_stack_allocated = if let Some(ref stats) = data_map.get(name) {
            if stats.has_virtual_methods {
                false
            } else if stats.pointers_count == 0 {
                true
            } else {
                let min_safe_data_count = 5;
                let min_not_pointers_percent = 0.3;
                if stats.pointers_count + stats.not_pointers_count < min_safe_data_count {
                    data.html_logger.add(
                        &[
                            name.as_str(),
                            "Can't determine type allocation place: not enough data",
                        ],
                        "type_allocation_places_error",
                    )?;
                } else if stats.not_pointers_count as f32
                    / (stats.pointers_count + stats.not_pointers_count) as f32
                    > min_not_pointers_percent
                {
                    data.html_logger.add(
                        &[
                            name.as_str(),
                            "Can't determine type allocation place: many non-pointers",
                        ],
                        "type_allocation_places_error",
                    )?;
                }
                false
            }
        } else {
            data.html_logger.add(
                &[
                    name.as_str(),
                    "Can't determine type allocation place: no stats",
                ],
                "type_allocation_places_error",
            )?;
            false
        };

        if suggest_stack_allocated {
            stack_allocated.push(name.clone());
        } else {
            heap_allocated.push(name.clone());
        }
    }

    data.html_logger.add(
        &[
            "Heap allocation is suggested for types:".to_string(),
            format!("{:?}", heap_allocated),
        ],
        "type_allocation_places_result",
    )?;
    data.html_logger.add(
        &[
            "Stack allocation is suggested for types:".to_string(),
            format!("{:?}", stack_allocated),
        ],
        "type_allocation_places_result",
    )?;

    Ok(())
}
