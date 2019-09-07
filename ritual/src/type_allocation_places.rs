#![allow(dead_code)]

use crate::config::MovableTypesHookOutput;
use crate::cpp_data::{CppItem, CppPath};
use crate::cpp_type::{CppPointerLikeTypeKind, CppType};
use crate::processor::ProcessorData;
use log::{info, trace};
use ritual_common::errors::Result;
use std::collections::HashMap;

#[derive(Default, Debug)]
struct TypeStats {
    virtual_functions: Vec<String>,
    pointer_encounters: Vec<String>,
    non_pointer_encounters: Vec<String>,
}

fn log_results(data_map: &HashMap<CppPath, TypeStats>) {
    for (name, stats) in data_map {
        trace!("type = {}; stats = {:?}", name.to_cpp_pseudo_code(), stats);
    }

    for (path, stats) in data_map {
        let suggestion = if stats.virtual_functions.is_empty() {
            if stats.pointer_encounters.is_empty() {
                if stats.non_pointer_encounters.len() == MAX_ITEMS {
                    "movable (no pointers, no virtual functions)"
                } else {
                    "probably movable (no pointers, no virtual functions, but too few items)"
                }
            } else if stats.pointer_encounters.len() < 5
                && stats.non_pointer_encounters.len() == MAX_ITEMS
            {
                "probably movable (few pointers)"
            } else if stats.pointer_encounters.len() == MAX_ITEMS {
                "immovable (many pointers)"
            } else {
                "unknown (too few items)"
            }
        } else {
            "immovable (has virtual functions)"
        };
        info!("{:?} is {}", path.to_templateless_string(), suggestion);
        info!("path = {}", path.to_cpp_pseudo_code());
        info!("* virtual_functions ({}):", stats.virtual_functions.len());
        for item in &stats.virtual_functions {
            info!("* * {}", item);
        }
        info!("* pointer_encounters ({}):", stats.pointer_encounters.len());
        for item in &stats.pointer_encounters {
            info!("* * {}", item);
        }
        info!(
            "* non_pointer_encounters ({}):",
            stats.non_pointer_encounters.len()
        );
        for item in &stats.non_pointer_encounters {
            info!("* * {}", item);
        }
    }
}

fn check_type(
    cpp_type: &CppType,
    is_behind_pointer: bool,
    data_map: &mut HashMap<CppPath, TypeStats>,
    item_text: &str,
) {
    match cpp_type {
        CppType::Class(path) => {
            let good_path = path.deinstantiate();
            if let Some(stats) = data_map.get_mut(&good_path) {
                if is_behind_pointer {
                    if stats.pointer_encounters.len() < MAX_ITEMS {
                        stats.pointer_encounters.push(item_text.to_string());
                    }
                } else if stats.non_pointer_encounters.len() < MAX_ITEMS {
                    stats.non_pointer_encounters.push(item_text.to_string());
                }
            }

            if let Some(args) = &path.last().template_arguments {
                for arg in args {
                    check_type(arg, false, data_map, item_text);
                }
            }
        }
        CppType::PointerLike { kind, target, .. } => {
            check_type(
                target,
                *kind == CppPointerLikeTypeKind::Pointer,
                data_map,
                item_text,
            );
        }
        _ => {}
    }
}

const MAX_ITEMS: usize = 10;

/// Detects the preferred type allocation place for each type based on
/// API of all known methods. Doesn't actually change the data,
/// only suggests stack allocated types for manual configuration.
pub fn suggest_allocation_places(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut data_map = HashMap::new();

    for item in data.db.cpp_items() {
        if item.source_id.is_some() {
            continue;
        }
        if let CppItem::Type(type1) = &item.item {
            if !type1.kind.is_class() {
                continue;
            }
            if let Some(hook) = data.config.movable_types_hook() {
                if hook(&type1.path)? != MovableTypesHookOutput::Unknown {
                    continue;
                }
            }
            let good_path = type1.path.deinstantiate();
            data_map.insert(good_path, Default::default());
        }
    }

    for item in data.db.cpp_items() {
        if item.source_id.is_some() {
            continue;
        }
        if let CppItem::Function(function) = &item.item {
            if function.is_private() {
                continue;
            }
            let item_text = function.short_text();
            for t in &function.arguments {
                check_type(&t.argument_type, false, &mut data_map, &item_text);
            }
            check_type(&function.return_type, false, &mut data_map, &item_text);
            if function.is_virtual() {
                let type1 = function.class_path()?;
                let good_path = type1.deinstantiate();
                if let Some(stats) = data_map.get_mut(&good_path) {
                    if stats.virtual_functions.len() < MAX_ITEMS {
                        stats.virtual_functions.push(item_text);
                    }
                }
            }
        }
    }

    log_results(&data_map);

    Ok(())
}
