use crate::config::MovableTypesHookOutput;
use crate::cpp_data::CppPath;
use crate::cpp_data::CppTypeDeclarationKind;
use crate::cpp_type::CppPointerLikeTypeKind;
use crate::cpp_type::CppType;
use crate::database::{CppItemData, DatabaseItemSource};
use crate::processor::ProcessorData;
use itertools::Itertools;
use log::{info, trace};
use ritual_common::errors::Result;
use std::collections::HashMap;

pub fn set_allocation_places(data: &mut ProcessorData<'_>) -> Result<()> {
    if let Some(hook) = data.config.movable_types_hook() {
        for type1 in data
            .current_database
            .cpp_items_mut()
            .iter_mut()
            .filter_map(|item| item.cpp_data.as_type_mut())
        {
            if let CppTypeDeclarationKind::Class { is_movable, .. } = &mut type1.kind {
                *is_movable = hook(&type1.path)? == MovableTypesHookOutput::Movable;
            }
        }
    }

    Ok(())
}

/// Detects the preferred type allocation place for each type based on
/// API of all known methods. Doesn't actually change the data,
/// only suggests stack allocated types for manual configuration.
pub fn suggest_allocation_places(data: &mut ProcessorData<'_>) -> Result<()> {
    #[derive(Default, Debug)]
    struct TypeStats {
        // has_derived_classes: bool,
        has_virtual_methods: bool,
        pointers_count: usize,
        not_pointers_count: usize,
    };
    fn check_type(
        cpp_type: &CppType,
        is_behind_pointer: bool,
        data_map: &mut HashMap<CppPath, TypeStats>,
    ) {
        match cpp_type {
            CppType::Class(path) => {
                let good_path = path.deinstantiate();
                if let Some(stats) = data_map.get_mut(&good_path) {
                    if is_behind_pointer {
                        stats.pointers_count += 1;
                    } else {
                        stats.not_pointers_count += 1;
                    }
                }

                if let Some(args) = &path.last().template_arguments {
                    for arg in args {
                        check_type(arg, false, data_map);
                    }
                }
            }
            CppType::PointerLike { kind, target, .. } => {
                check_type(target, *kind == CppPointerLikeTypeKind::Pointer, data_map);
            }
            _ => {}
        }
    }

    let mut data_map = HashMap::new();

    for type1 in data
        .current_database
        .cpp_items()
        .iter()
        .filter_map(|i| i.cpp_data.as_type_ref())
    {
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

    for item in data.current_database.cpp_items() {
        if let DatabaseItemSource::CppParser { .. } = &item.source {
            if let CppItemData::Function(function) = &item.cpp_data {
                for t in &function.arguments {
                    check_type(&t.argument_type, false, &mut data_map);
                }
                check_type(&function.return_type, false, &mut data_map);
                if function.is_virtual() {
                    let type1 = function.class_type()?;
                    let good_path = type1.deinstantiate();
                    if let Some(stats) = data_map.get_mut(&good_path) {
                        stats.has_virtual_methods = true;
                    }
                }
            }
        }
    }

    for (name, stats) in &data_map {
        trace!("type = {}; stats = {:?}", name.to_cpp_pseudo_code(), stats);
    }

    let mut movable_types = Vec::new();
    let mut immovable_types = Vec::new();

    for (path, stats) in data_map {
        let is_good = stats.pointers_count == 0;
        let is_probably_good = stats.pointers_count < 5 && stats.not_pointers_count > 10;
        let suggest_movable = (is_good || is_probably_good) && !stats.has_virtual_methods;
        if suggest_movable {
            movable_types.push(path.clone());
        } else {
            immovable_types.push(path.clone());
        }
    }
    info!(
        "Recommended immovable types: {}",
        immovable_types
            .iter()
            .map(|p| format!("{:?}", p.to_cpp_pseudo_code()))
            .join(", ")
    );
    info!(
        "Recommended movable types: {}",
        movable_types
            .iter()
            .map(|p| format!("{:?}", p.to_cpp_pseudo_code()))
            .join(", ")
    );

    Ok(())
}
