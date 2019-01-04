use crate::common::errors::{bail, Result};
use crate::common::log;
use crate::cpp_data::CppPathItem;
use crate::cpp_data::CppTemplateInstantiation;
use crate::cpp_function::CppFunction;
use crate::cpp_function::CppFunctionArgument;
use crate::cpp_function::CppOperator;
use crate::cpp_type::CppType;
use crate::database::CppItemData;
use crate::database::DatabaseItemSource;
use crate::processor::ProcessingStep;
use crate::processor::ProcessorData;

/// Returns true if `type1` is a known template instantiation.
fn check_template_type(data: &ProcessorData, type1: &CppType) -> Result<()> {
    if let CppType::Class(ref path) = type1 {
        if let Some(ref template_arguments) = path.last().template_arguments {
            let is_available = data
                .all_items()
                .iter()
                .filter_map(|i| i.cpp_data.as_template_instantiation_ref())
                .any(|inst| {
                    // TODO: fix after CppPath refactoring
                    &inst.class_name == path && &inst.template_arguments == template_arguments
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
    template_instantiation: &CppTemplateInstantiation,
) -> Result<CppFunction> {
    log::llog(log::DebugTemplateInstantiation, || {
        format!("instantiation: {:?}", template_instantiation)
    });
    let mut new_method = method.clone();

    new_method.arguments.clear();
    for arg in &method.arguments {
        new_method.arguments.push(CppFunctionArgument {
            name: arg.name.clone(),
            has_default_value: arg.has_default_value,
            argument_type: arg
                .argument_type
                .instantiate(nested_level1, &template_instantiation.template_arguments)?,
        });
    }
    new_method.return_type = method
        .return_type
        .instantiate(nested_level1, &template_instantiation.template_arguments)?;

    new_method.path = new_method
        .path
        .instantiate(nested_level1, &template_instantiation.template_arguments)?;
    let mut conversion_type = None;
    if let Some(ref mut operator) = new_method.operator {
        if let CppOperator::Conversion(ref mut cpp_type) = *operator {
            let r =
                cpp_type.instantiate(nested_level1, &template_instantiation.template_arguments)?;
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
            new_method.path.items.pop().expect("CppPath can't be empty");
            new_method
                .path
                .items
                .push(CppPathItem::from_str_unchecked(&format!(
                    "operator {}",
                    conversion_type.to_cpp_code(None)?
                )));
        }
        log::llog(log::DebugTemplateInstantiation, || {
            format!("success: {}", new_method.short_text())
        });
        Ok(new_method)
    }
}

pub fn instantiate_templates_step() -> ProcessingStep {
    ProcessingStep::new(
        "instantiate_templates",
        vec!["find_template_instantiations".to_string()],
        instantiate_templates,
    )
}

/// Generates methods as template instantiations of
/// methods of existing template classes and existing template methods.
fn instantiate_templates(data: &mut ProcessorData) -> Result<()> {
    log::status("Instantiating templates");
    let mut new_methods = Vec::new();
    for method in data
        .all_items()
        .iter()
        .filter_map(|item| item.cpp_data.as_function_ref())
    {
        for type1 in method.all_involved_types() {
            let path = match type1 {
                CppType::Class(ref class_type) => class_type,
                CppType::PointerLike { ref target, .. } => match **target {
                    CppType::Class(ref class_type) => class_type,
                    _ => continue,
                },
                _ => continue,
            };
            if let Some(ref template_arguments) = path.last().template_arguments {
                assert!(!template_arguments.is_empty());
                if template_arguments.iter().all(|x| x.is_template_parameter()) {
                    for template_instantiation in data
                        .current_database
                        .items
                        .iter()
                        .filter_map(|item| item.cpp_data.as_template_instantiation_ref())
                    {
                        if &template_instantiation.class_name == path {
                            // TODO: ignore last template args?
                            let nested_level =
                                if let CppType::TemplateParameter { nested_level, .. } =
                                    template_arguments[0]
                                {
                                    nested_level
                                } else {
                                    bail!("only template parameters can be here");
                                };
                            log::llog(log::DebugTemplateInstantiation, || "");
                            log::llog(log::DebugTemplateInstantiation, || {
                                format!("method: {}", method.short_text())
                            });
                            log::llog(log::DebugTemplateInstantiation, || {
                                format!(
                                    "found template instantiation: {:?}",
                                    template_instantiation
                                )
                            });
                            match apply_instantiation_to_method(
                                method,
                                nested_level,
                                template_instantiation,
                            ) {
                                Ok(method) => {
                                    let mut ok = true;
                                    for type1 in method.all_involved_types() {
                                        match check_template_type(&data, &type1) {
                                            Ok(_) => {}
                                            Err(msg) => {
                                                ok = false;
                                                log::llog(log::DebugTemplateInstantiation, || {
                                                    format!(
                                                        "method is not accepted: {}",
                                                        method.short_text()
                                                    )
                                                });
                                                log::llog(log::DebugTemplateInstantiation, || {
                                                    format!("  {}", msg)
                                                });
                                            }
                                        }
                                    }
                                    if ok {
                                        new_methods.push(method);
                                    }
                                    break;
                                }
                                Err(msg) => log::llog(log::DebugTemplateInstantiation, || {
                                    format!("failed: {}", msg)
                                }),
                            }
                            break;
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
    ProcessingStep::new(
        "find_template_instantiations",
        vec!["cpp_parser".to_string()],
        find_template_instantiations,
    )
}

/// Searches for template instantiations in this library's API,
/// excluding results that were already processed in dependencies.
#[allow(clippy::block_in_if_condition_stmt)]
fn find_template_instantiations(data: &mut ProcessorData) -> Result<()> {
    fn check_type(
        type1: &CppType,
        data: &ProcessorData,
        result: &mut Vec<CppTemplateInstantiation>,
    ) {
        match type1 {
            CppType::Class(ref path) => {
                if let Some(ref template_arguments) = path.last().template_arguments {
                    if !template_arguments
                        .iter()
                        .any(|x| x.is_or_contains_template_parameter())
                    {
                        let is_in_database = data
                            .all_items()
                            .iter()
                            .filter_map(|item| item.cpp_data.as_template_instantiation_ref())
                            .any(|i| {
                                // TODO: ignore last template args?
                                &i.class_name == path && &i.template_arguments == template_arguments
                            });
                        if !is_in_database {
                            let is_in_result = result.iter().any(|x| {
                                // TODO: ignore last template args?
                                &x.class_name == path && &x.template_arguments == template_arguments
                            });
                            if !is_in_result {
                                log::llog(log::DebugParser, || {
                                    format!("Found template instantiation: {}", path)
                                });
                                result.push(CppTemplateInstantiation {
                                    class_name: path.clone(),
                                    template_arguments: template_arguments.clone(),
                                });
                            }
                        }
                    }
                    for arg in template_arguments {
                        check_type(arg, &data, result);
                    }
                }
            }
            CppType::PointerLike { ref target, .. } => check_type(target, data, result),
            _ => {}
        }
    }
    let mut result = Vec::new();
    for item in &data.current_database.items {
        for type1 in item.cpp_data.all_involved_types() {
            check_type(&type1, &data, &mut result);
        }
    }
    for item in result {
        data.current_database.add_cpp_data(
            DatabaseItemSource::TemplateInstantiation,
            CppItemData::TemplateInstantiation(item),
        );
    }
    Ok(())
}
