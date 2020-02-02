use crate::cpp_data::{CppItem, CppPath, CppPathItem, CppTypeDeclaration};
use crate::cpp_function::{CppFunction, CppFunctionArgument, CppOperator};
use crate::cpp_type::CppType;
use crate::database::{DatabaseClient, ItemWithSource};
use crate::processor::ProcessorData;
use log::{debug, trace};
use ritual_common::errors::{bail, err_msg, Result};
use ritual_common::utils::MapIfOk;

/// Returns true if `type1` is a known template instantiation.
fn check_template_type(data: &ProcessorData<'_>, type1: &CppType) -> Result<()> {
    match &type1 {
        CppType::Class(path) => {
            if let Some(template_arguments) = &path.last().template_arguments {
                let is_available = data
                    .db
                    .all_cpp_items()
                    .filter_map(|i| i.item.as_type_ref())
                    .any(|inst| &inst.path == path);
                if !is_available {
                    bail!("type is not available: {:?}", type1);
                }
                for arg in template_arguments {
                    check_template_type(data, arg)?;
                }
            }
        }
        CppType::PointerLike { ref target, .. } => {
            check_template_type(data, target)?;
        }
        _ => {}
    }
    Ok(())
}

/// Tries to apply each of `template_instantiations` to `function`.
/// Only types at the specified `nested_level` are replaced.
/// Returns `Err` if any of `template_instantiations` is incompatible
/// with the function.
pub fn instantiate_function(
    function: &CppFunction,
    nested_level: usize,
    arguments: &[CppType],
) -> Result<CppFunction> {
    let mut new_method = function.clone();
    new_method.arguments.clear();
    for arg in &function.arguments {
        new_method.arguments.push(CppFunctionArgument {
            name: arg.name.clone(),
            has_default_value: arg.has_default_value,
            argument_type: arg.argument_type.instantiate(nested_level, arguments)?,
        });
    }
    new_method.return_type = function.return_type.instantiate(nested_level, arguments)?;

    new_method.path = new_method.path.instantiate(nested_level, arguments)?;
    if let Some(args) = &new_method.path.last().template_arguments {
        if args
            .iter()
            .any(|arg| arg.is_or_contains_template_parameter())
        {
            bail!(
                "extra template parameters left: {}",
                new_method.short_text()
            );
        }
        if function.can_infer_template_arguments() {
            // explicitly specifying template arguments sometimes causes compiler errors,
            // so we prefer to get them inferred
            new_method.path.last_mut().template_arguments = None;
        }
    }

    let mut conversion_type = None;
    if let Some(operator) = &mut new_method.operator {
        if let CppOperator::Conversion(cpp_type) = operator {
            let r = cpp_type.instantiate(nested_level, arguments)?;
            *cpp_type = r.clone();
            conversion_type = Some(r);
        }
    }
    if new_method
        .all_involved_types()
        .iter()
        .any(CppType::is_or_contains_template_parameter)
    {
        bail!(
            "extra template parameters left: {}",
            new_method.short_text()
        );
    } else {
        if let Some(conversion_type) = conversion_type {
            *new_method.path.last_mut() = CppPathItem {
                name: format!("operator {}", conversion_type.to_cpp_code(None)?),
                template_arguments: None,
            };
        }
        trace!("success: {}", new_method.short_text());
        Ok(new_method)
    }
}

// TODO: instantiations of QObject::findChild and QObject::findChildren should be available

#[derive(Debug)]
struct Substitution<'a> {
    nested_level: usize,
    arguments: &'a [CppType],
}

fn find_suitable_template_arguments<'a>(
    path: &CppPath,
    db: &'a DatabaseClient,
) -> Result<Vec<Substitution<'a>>> {
    let mut current_path = path.clone();
    let mut result = Vec::new();
    loop {
        if let Some(template_arguments) = &current_path.last().template_arguments {
            assert!(!template_arguments.is_empty());
            if template_arguments.iter().all(|t| t.is_template_parameter()) {
                let items =
                    db.cpp_items()
                        .filter_map(|item| item.item.as_type_ref())
                        .filter(|type1| {
                            type1.path.parent_parts().ok() == current_path.parent_parts().ok()
                                && type1.path.last().name == current_path.last().name
                                && type1.path.last().template_arguments.as_ref().map_or(
                                    false,
                                    |args| {
                                        !args.iter().all(CppType::is_or_contains_template_parameter)
                                    },
                                )
                        })
                        .map_if_ok(|type1| {
                            let nested_level =
                                if let CppType::TemplateParameter(param) = &template_arguments[0] {
                                    param.nested_level
                                } else {
                                    bail!("only template parameters can be here");
                                };

                            let arguments = type1
                                .path
                                .last()
                                .template_arguments
                                .as_ref()
                                .ok_or_else(|| {
                                    err_msg("template instantiation must have template arguments")
                                })?;

                            Ok(Substitution {
                                nested_level,
                                arguments,
                            })
                        })?;
                result.extend(items);
            }
        }
        if let Ok(p) = current_path.parent() {
            current_path = p;
        } else {
            return Ok(result);
        }
    }
}

fn instantiate_types(data: &mut ProcessorData<'_>) -> Result<()> {
    loop {
        let mut new_types = Vec::<ItemWithSource<_>>::new();
        for type1 in data
            .db
            .all_cpp_items()
            .filter_map(|item| item.filter_map(|item| item.as_type_ref()))
        {
            trace!("class: {}", type1.item.path.to_cpp_pseudo_code());
            for substitution in find_suitable_template_arguments(&type1.item.path, data.db)? {
                trace!("found template instantiation: {:?}", substitution);

                let new_type = CppTypeDeclaration {
                    kind: type1.item.kind.clone(),
                    path: type1
                        .item
                        .path
                        .instantiate(substitution.nested_level, substitution.arguments)?,
                };

                if data
                    .db
                    .all_cpp_items()
                    .filter_map(|item| item.item.as_type_ref())
                    .any(|item| item.is_same(&new_type))
                    || new_types.iter().any(|item| item.item == new_type)
                {
                    trace!("type already exists");
                    continue;
                }
                new_types.push(ItemWithSource::new(&type1.id, new_type));
            }
        }
        let any_found = !new_types.is_empty();
        for new_type in new_types {
            data.add_cpp_item(Some(new_type.source_id), CppItem::Type(new_type.item))?;
        }
        if !any_found {
            break;
        }
    }
    Ok(())
}

fn instantiate_functions(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut new_methods = Vec::new();
    for item in data.db.all_cpp_items() {
        let function = if let Some(f) = item.item.as_function_ref() {
            f
        } else {
            continue;
        };

        for type1 in function.all_involved_types() {
            let path = match &type1 {
                CppType::Class(class_type) => class_type,
                CppType::PointerLike { target, .. } => match &**target {
                    CppType::Class(class_type) => class_type,
                    _ => continue,
                },
                _ => continue,
            };

            for substitution in find_suitable_template_arguments(path, data.db)? {
                trace!("method: {}", function.short_text());
                trace!("found template instantiation: {:?}", substitution);

                match instantiate_function(
                    function,
                    substitution.nested_level,
                    substitution.arguments,
                ) {
                    Ok(method) => {
                        let mut ok = true;
                        for type1 in method.all_involved_types() {
                            match check_template_type(&data, &type1) {
                                Ok(_) => {}
                                Err(msg) => {
                                    ok = false;
                                    trace!("method is not accepted: {}", method.short_text());
                                    trace!("  {}", msg);
                                }
                            }
                        }
                        if ok {
                            if data
                                .db
                                .all_cpp_items()
                                .filter_map(|item| item.item.as_function_ref())
                                .any(|item| item.is_same(&method))
                            {
                                trace!("this method already exists");
                            } else {
                                new_methods.push(ItemWithSource::new(&item.id, method));
                            }
                        }
                    }
                    Err(msg) => trace!("failed: {}", msg),
                }
            }
        }
    }
    for new_method in new_methods {
        data.add_cpp_item(
            Some(new_method.source_id),
            CppItem::Function(new_method.item),
        )?;
    }
    Ok(())
}

/// Generates methods as template instantiations of
/// methods of existing template classes and existing template methods.
pub fn instantiate_templates(data: &mut ProcessorData<'_>) -> Result<()> {
    instantiate_types(data)?;
    instantiate_functions(data)?;
    Ok(())
}

/// Searches for template instantiations in this library's API,
/// excluding results that were already processed in dependencies.
pub fn find_template_instantiations(data: &mut ProcessorData<'_>) -> Result<()> {
    fn check_type(type1: &CppType, data: &ProcessorData<'_>, result: &mut Vec<CppPath>) {
        match &type1 {
            CppType::Class(path) => {
                if let Some(template_arguments) = &path.last().template_arguments {
                    if !template_arguments
                        .iter()
                        .any(CppType::is_or_contains_template_parameter)
                    {
                        let is_in_database = data
                            .db
                            .all_cpp_items()
                            .filter_map(|item| item.item.as_type_ref())
                            .any(|i| &i.path == path);
                        if !is_in_database {
                            let is_in_result = result.iter().any(|x| x == path);
                            if !is_in_result {
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
    for item in data.db.cpp_items() {
        for type1 in item.item.all_involved_types() {
            check_type(&type1, &data, &mut result);
        }
    }
    for item in result {
        let original_type = data
            .db
            .all_cpp_items()
            .filter_map(|x| x.filter_map(|item| item.as_type_ref()))
            .find(|t| {
                let t = &t.item;
                t.path.parent_parts().ok() == item.parent_parts().ok()
                    && t.path.last().name == item.last().name
                    && t.path
                        .last()
                        .template_arguments
                        .as_ref()
                        .map_or(false, |args| {
                            args.iter().all(CppType::is_template_parameter)
                        })
            });
        if let Some(original_type) = original_type {
            let mut new_type = original_type.item.clone();
            new_type.path = item;
            let source_id = original_type.id.clone();
            data.add_cpp_item(Some(source_id), CppItem::Type(new_type))?;
        } else {
            debug!(
                "original type not found for instantiation: {}",
                item.to_cpp_pseudo_code()
            );
        }
    }
    Ok(())
}
