#![allow(dead_code)]

use crate::cpp_data::CppPath;
use crate::cpp_data::CppVisibility;
use crate::cpp_function::CppFunction;
use crate::processor::ProcessorData;
use itertools::Itertools;
use log::trace;
use ritual_common::errors::*;

/// Checks if `class_name` types inherits `base_name` type directly or indirectly.
pub fn inherits(class_name: &CppPath, base_name: &CppPath, data: &ProcessorData<'_>) -> bool {
    for base in data.all_items().filter_map(|x| x.cpp_data.as_base_ref()) {
        if &base.derived_class_type == class_name {
            if &base.base_class_type == base_name {
                return true;
            }
            if inherits(&base.base_class_type, base_name, data) {
                return true;
            }
        }
    }
    false
}

fn detect_inherited_methods2(data: &ProcessorData<'_>) -> Result<Vec<CppFunction>> {
    let mut remaining_classes = data
        .all_items()
        .filter_map(|x| x.cpp_data.as_base_ref())
        .filter(|b| b.visibility != CppVisibility::Private)
        .collect_vec();

    let mut ordered_classes = Vec::new();
    while !remaining_classes.is_empty() {
        let mut any_added = false;
        let mut remaining_classes2 = Vec::new();
        for class in &remaining_classes {
            if remaining_classes
                .iter()
                .any(|c| c.derived_class_type == class.base_class_type)
            {
                remaining_classes2.push(*class);
            } else {
                ordered_classes.push(*class);
                any_added = true;
            }
        }
        remaining_classes = remaining_classes2;
        if !any_added {
            bail!("Cyclic dependency detected while detecting inherited methods");
        }
    }

    let mut result = Vec::new();
    for class in ordered_classes {
        trace!("Detecting inherited methods for {:?}\n", class);
        let methods = data
            .all_items()
            .filter_map(|x| x.cpp_data.as_function_ref())
            .filter(|m| m.class_type().ok().as_ref() == Some(&class.base_class_type));

        for method in methods {
            let mut new_method = (*method).clone();
            new_method.path = class.base_class_type.join(method.path.last().clone());
            new_method.declaration_code = None;
            //new_method.is_fake_inherited_method = true;
            trace!("Method added: {}", new_method.short_text());
            trace!("Base method: {}\n", method.short_text(),);
            result.push(new_method);
        }
    }
    Ok(result)
}
