use crate::cpp_data::CppPath;
use crate::cpp_data::CppPathItem;
use crate::cpp_function::CppFunction;
use crate::cpp_function::CppFunctionArgument;
use crate::cpp_function::CppOperator;
use crate::cpp_type::CppType;
use crate::database::CppItemData;
use crate::database::DatabaseItemSource;
use crate::processor::ProcessingStep;
use crate::processor::ProcessorData;
use log::{trace, warn};
use ritual_common::errors::err_msg;
use ritual_common::errors::{bail, Result};

/// Returns true if `type1` is a known template instantiation.
fn check_template_type(data: &ProcessorData<'_>, type1: &CppType) -> Result<()> {
    if let CppType::Class(path) = &type1 {
        if let Some(template_arguments) = &path.last().template_arguments {
            let is_available = data
                .all_items()
                .filter_map(|i| i.cpp_data.as_type_ref())
                .any(|inst| {
                    // TODO: fix after CppPath refactoring
                    &inst.path == path
                });
            if !is_available {
                bail!("type not available: {:?}", type1);
            }
            for arg in template_arguments {
                check_template_type(data, arg)?;
            }
        }
    }
    Ok(())
}

/// Tries to apply each of `template_instantiations` to `method`.
/// Only types at the specified `nested_level` are replaced.
/// Returns `Err` if any of `template_instantiations` is incompatible
/// with the method.
fn apply_instantiation_to_method(
    method: &CppFunction,
    nested_level1: usize,
    template_instantiation: &CppPath,
) -> Result<CppFunction> {
    trace!(
        "[DebugTemplateInstantiation] instantiation: {:?}",
        template_instantiation
    );
    let mut new_method = method.clone();

    let inst_args = template_instantiation
        .last()
        .template_arguments
        .as_ref()
        .ok_or_else(|| err_msg("template instantiation must have template arguments"))?;

    new_method.arguments.clear();
    for arg in &method.arguments {
        new_method.arguments.push(CppFunctionArgument {
            name: arg.name.clone(),
            has_default_value: arg.has_default_value,
            argument_type: arg.argument_type.instantiate(nested_level1, inst_args)?,
        });
    }
    new_method.return_type = method.return_type.instantiate(nested_level1, inst_args)?;

    new_method.path = new_method.path.instantiate(nested_level1, inst_args)?;
    let mut conversion_type = None;
    if let Some(operator) = &mut new_method.operator {
        if let CppOperator::Conversion(cpp_type) = operator {
            let r = cpp_type.instantiate(nested_level1, inst_args)?;
            *cpp_type = r.clone();
            conversion_type = Some(r);
        }
    }
    if new_method
        .all_involved_types()
        .iter()
        .any(|t| t.is_or_contains_template_parameter())
    {
        bail!(
            "extra template parameters left: {}",
            new_method.short_text()
        );
    } else {
        if let Some(conversion_type) = conversion_type {
            *new_method.path.last_mut() = CppPathItem::from_good_str(&format!(
                "operator {}",
                conversion_type.to_cpp_code(None)?
            ));
        }
        trace!(
            "[DebugTemplateInstantiation] success: {}",
            new_method.short_text()
        );
        Ok(new_method)
    }
}

// TODO: instantiations of QObject::findChild and QObject::findChildren should be available

pub fn instantiate_templates_step() -> ProcessingStep {
    ProcessingStep::new("instantiate_templates", instantiate_templates)
}

/// Generates methods as template instantiations of
/// methods of existing template classes and existing template methods.
fn instantiate_templates(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut new_methods = Vec::new();
    for method in data
        .all_items()
        .filter_map(|item| item.cpp_data.as_function_ref())
    {
        for type1 in method.all_involved_types() {
            let path = match &type1 {
                CppType::Class(class_type) => class_type,
                CppType::PointerLike { target, .. } => match &**target {
                    CppType::Class(class_type) => class_type,
                    _ => continue,
                },
                _ => continue,
            };
            if let Some(template_arguments) = &path.last().template_arguments {
                assert!(!template_arguments.is_empty());
                if template_arguments.iter().all(|x| x.is_template_parameter()) {
                    for type1 in data
                        .current_database
                        .cpp_items
                        .iter()
                        .filter_map(|item| item.cpp_data.as_type_ref())
                    {
                        let is_suitable = type1.path.parent().ok() == path.parent().ok()
                            && type1.path.last().name == path.last().name
                            && type1.path.last().template_arguments.as_ref().map_or(
                                false,
                                |args| {
                                    !args
                                        .iter()
                                        .all(|arg| arg.is_or_contains_template_parameter())
                                },
                            );

                        if is_suitable {
                            let nested_level =
                                if let CppType::TemplateParameter { nested_level, .. } =
                                    template_arguments[0]
                                {
                                    nested_level
                                } else {
                                    bail!("only template parameters can be here");
                                };
                            trace!(
                                "[DebugTemplateInstantiation] method: {}",
                                method.short_text()
                            );
                            trace!(
                                "[DebugTemplateInstantiation] found template instantiation: {:?}",
                                type1
                            );
                            match apply_instantiation_to_method(method, nested_level, &type1.path) {
                                Ok(method) => {
                                    let mut ok = true;
                                    for type1 in method.all_involved_types() {
                                        match check_template_type(&data, &type1) {
                                            Ok(_) => {}
                                            Err(msg) => {
                                                ok = false;
                                                trace!("[DebugTemplateInstantiation] method is not accepted: {}",
                                                        method.short_text()
                                                    );
                                                trace!("[DebugTemplateInstantiation]   {}", msg);
                                            }
                                        }
                                    }
                                    if ok {
                                        new_methods.push(method);
                                    }
                                }
                                Err(msg) => trace!("[DebugTemplateInstantiation] failed: {}", msg),
                            }
                        }
                    }
                }
            }
        }
    }
    for item in new_methods {
        data.current_database.add_cpp_data(
            DatabaseItemSource::TemplateInstantiation,
            CppItemData::Function(item),
        );
    }
    Ok(())
}

pub fn find_template_instantiations_step() -> ProcessingStep {
    ProcessingStep::new("find_template_instantiations", find_template_instantiations)
}

/// Searches for template instantiations in this library's API,
/// excluding results that were already processed in dependencies.
fn find_template_instantiations(data: &mut ProcessorData<'_>) -> Result<()> {
    fn check_type(type1: &CppType, data: &ProcessorData<'_>, result: &mut Vec<CppPath>) {
        match &type1 {
            CppType::Class(path) => {
                if let Some(template_arguments) = &path.last().template_arguments {
                    if !template_arguments
                        .iter()
                        .any(|x| x.is_or_contains_template_parameter())
                    {
                        let is_in_database = data
                            .all_items()
                            .filter_map(|item| item.cpp_data.as_type_ref())
                            .any(|i| &i.path == path);
                        if !is_in_database {
                            let is_in_result = result.iter().any(|x| {
                                // TODO: ignore last template args?
                                x == path
                            });
                            if !is_in_result {
                                trace!(
                                    "Found template instantiation: {}",
                                    path.to_cpp_pseudo_code()
                                );
                                result.push(path.clone());
                            }
                        }
                    }
                    for arg in template_arguments {
                        check_type(arg, &data, result);
                    }
                }
            }
            CppType::PointerLike { target, .. } => check_type(target, data, result),
            _ => {}
        }
    }
    let mut result = Vec::new();
    for item in &data.current_database.cpp_items {
        for type1 in item.cpp_data.all_involved_types() {
            check_type(&type1, &data, &mut result);
        }
    }
    for item in result {
        let original_type = data
            .all_items()
            .filter_map(|x| x.cpp_data.as_type_ref())
            .find(|t| {
                t.path.parent().ok() == item.parent().ok()
                    && t.path.last().name == item.last().name
                    && t.path
                        .last()
                        .template_arguments
                        .as_ref()
                        .map_or(false, |args| {
                            args.iter().all(|arg| arg.is_template_parameter())
                        })
            });
        if let Some(original_type) = original_type {
            let mut new_type = original_type.clone();
            new_type.path = item;
            data.current_database.add_cpp_data(
                DatabaseItemSource::TemplateInstantiation,
                CppItemData::Type(new_type),
            );
        } else {
            warn!("original type not found for instantiation: {:?}", item);
        }
    }
    Ok(())
}
