use crate::cpp_data::CppBaseSpecifier;
use crate::cpp_data::CppClassField;
use crate::cpp_data::CppPath;
use crate::cpp_data::CppPathItem;
use crate::cpp_data::CppTypeDeclarationKind;
use crate::cpp_data::CppVisibility;
use crate::cpp_ffi_data::CppFfiArgumentMeaning;
use crate::cpp_ffi_data::CppFfiFunctionArgument;
use crate::cpp_ffi_data::CppFfiType;
use crate::cpp_ffi_data::{CppCast, CppFfiFunction, CppFfiFunctionKind, CppFieldAccessorType};
use crate::cpp_function::ReturnValueAllocationPlace;
use crate::cpp_function::{CppFunction, CppFunctionArgument, CppFunctionKind};
use crate::cpp_type::is_qflags;
use crate::cpp_type::CppPointerLikeTypeKind;
use crate::cpp_type::CppType;
use crate::cpp_type::CppTypeRole;
use crate::database::CppFfiItem;
use crate::database::CppItemData;
use crate::processor::ProcessorData;
use itertools::Itertools;
use log::{debug, trace};
use ritual_common::errors::{bail, Result};
use std::collections::HashSet;

pub struct FfiNameProvider {
    names: HashSet<String>,
    prefix: String,
}

impl FfiNameProvider {
    pub fn new(data: &ProcessorData<'_>) -> Self {
        let prefix = format!("ctr_{}_ffi", &data.config.crate_properties().name());
        let names = data
            .current_database
            .ffi_items()
            .iter()
            .map(|f| f.path().to_cpp_code().unwrap())
            .collect();

        FfiNameProvider { prefix, names }
    }

    pub fn testing() -> Self {
        FfiNameProvider {
            names: HashSet::new(),
            prefix: String::new(),
        }
    }

    pub fn create_path(&mut self, name: &str) -> CppPath {
        let mut num: Option<u32> = None;
        let full_name = loop {
            let full_name = format!(
                "{}_{}{}",
                self.prefix,
                name,
                num.map_or(String::new(), |num| num.to_string())
            );
            if !self.names.contains(&full_name) {
                break full_name;
            }
            num = Some(num.map_or(1, |num| num + 1));
        };
        let item = CppPathItem::from_good_str(&full_name);
        self.names.insert(full_name);
        CppPath::from_item(item)
    }
}

/// Runs the FFI generator
pub fn run(data: &mut ProcessorData<'_>) -> Result<()> {
    let movable_types = data
        .all_items()
        .filter_map(|item| {
            if let CppItemData::Type(type_data) = &item.cpp_data {
                if let CppTypeDeclarationKind::Class { is_movable } = type_data.kind {
                    if is_movable {
                        return Some(type_data.path.clone());
                    }
                }
            }
            None
        })
        .collect_vec();

    let mut name_provider = FfiNameProvider::new(data);

    for index in 0..data.current_database.cpp_items().len() {
        let item = &mut data.current_database.cpp_items_mut()[index];
        if item.is_cpp_ffi_processed {
            trace!(
                "cpp_data = {}; already processed",
                item.cpp_data.to_string()
            );
            continue;
        }
        if let Err(err) = check_preconditions(&item.cpp_data) {
            trace!("skipping {}: {}", item.cpp_data, err);
            continue;
        }
        let result = match &item.cpp_data {
            CppItemData::Function(method) => {
                generate_ffi_methods_for_method(method, &movable_types, &mut name_provider)
                    .map(|v| v.into_iter().collect_vec())
            }
            CppItemData::ClassField(field) => {
                generate_field_accessors(field, &movable_types, &mut name_provider)
                    .map(|v| v.into_iter().collect_vec())
            }
            CppItemData::ClassBase(base) => {
                generate_casts(base, &mut name_provider).map(|v| v.into_iter().collect_vec())
            }
            CppItemData::Type(_) | CppItemData::EnumValue(_) | CppItemData::Namespace(_) => {
                // no FFI methods for these items
                continue;
            }
        };

        match result {
            Err(error) => {
                debug!("failed to add FFI item: {}: {}", item.cpp_data, error);
            }
            Ok(r) => {
                debug!(
                    "added FFI items (count: {}) for: {}",
                    r.len(),
                    item.cpp_data
                );
                for item in &r {
                    trace!("* {:?}", item);
                }
                item.is_cpp_ffi_processed = true;
                data.current_database.add_ffi_items(r);
            }
        }
    }
    Ok(())
}

/// Convenience function to create `CppMethod` object for
/// `static_cast` or `dynamic_cast` from type `from` to type `to`.
/// See `CppMethod`'s documentation for more information
/// about `is_unsafe_static_cast` and `is_direct_static_cast`.
fn create_cast_method(
    cast: CppCast,
    from: &CppType,
    to: &CppType,
    name_provider: &mut FfiNameProvider,
) -> Result<CppFfiItem> {
    let method = CppFunction {
        path: CppPath::from_item(CppPathItem {
            name: cast.cpp_method_name().into(),
            template_arguments: Some(vec![to.clone()]),
        }),
        member: None,
        operator: None,
        return_type: to.clone(),
        arguments: vec![CppFunctionArgument {
            name: "ptr".to_string(),
            argument_type: from.clone(),
            has_default_value: false,
        }],
        allows_variadic_arguments: false,
        declaration_code: None,
        doc: None,
    };
    // no need for movable_types since all cast methods operate on pointers
    let r = to_ffi_method(
        &CppFfiFunctionKind::Function {
            cpp_function: method.clone(),
            omitted_arguments: None,
            cast: Some(cast),
        },
        &[],
        name_provider,
    )?;

    Ok(CppFfiItem::from_function(r))
}

/// Performs a portion of `generate_casts` operation.
/// Adds casts between `target_type` and `base_type` and calls
/// `generate_casts_one` recursively to add casts between `target_type`
/// and base types of `base_type`.
fn generate_casts_one(
    target_type: &CppPath,
    base_type: &CppPath,
    direct_base_index: usize,
    name_provider: &mut FfiNameProvider,
) -> Result<Vec<CppFfiItem>> {
    let target_ptr_type = CppType::PointerLike {
        is_const: false,
        kind: CppPointerLikeTypeKind::Pointer,
        target: Box::new(CppType::Class(target_type.clone())),
    };
    let base_ptr_type = CppType::PointerLike {
        is_const: false,
        kind: CppPointerLikeTypeKind::Pointer,
        target: Box::new(CppType::Class(base_type.clone())),
    };
    let mut new_methods = Vec::new();
    new_methods.push(create_cast_method(
        CppCast::Static {
            is_unsafe: true,
            base_index: direct_base_index,
        },
        &base_ptr_type,
        &target_ptr_type,
        name_provider,
    )?);
    new_methods.push(create_cast_method(
        CppCast::Static {
            is_unsafe: false,
            base_index: direct_base_index,
        },
        &target_ptr_type,
        &base_ptr_type,
        name_provider,
    )?);
    new_methods.push(create_cast_method(
        CppCast::Dynamic,
        &base_ptr_type,
        &target_ptr_type,
        name_provider,
    )?);

    Ok(new_methods)
}

/// Adds `static_cast` and `dynamic_cast` functions for all appropriate pairs of types
/// in this `CppData`.
fn generate_casts(
    base: &CppBaseSpecifier,
    name_provider: &mut FfiNameProvider,
) -> Result<Vec<CppFfiItem>> {
    //log::status("Adding cast functions");
    generate_casts_one(
        &base.derived_class_type,
        &base.base_class_type,
        base.base_index,
        name_provider,
    )
}

fn generate_ffi_methods_for_method(
    method: &CppFunction,
    movable_types: &[CppPath],
    name_provider: &mut FfiNameProvider,
) -> Result<Vec<CppFfiItem>> {
    let mut methods = Vec::new();
    methods.push(CppFfiItem::from_function(to_ffi_method(
        &CppFfiFunctionKind::Function {
            cpp_function: method.clone(),
            omitted_arguments: None,
            cast: None,
        },
        movable_types,
        name_provider,
    )?));

    if let Some(last_arg) = method.arguments.last() {
        if last_arg.has_default_value {
            let mut method_copy = method.clone();
            while let Some(arg) = method_copy.arguments.pop() {
                if !arg.has_default_value {
                    break;
                }
                let processed_method = to_ffi_method(
                    &CppFfiFunctionKind::Function {
                        cpp_function: method_copy.clone(),
                        omitted_arguments: Some(
                            method.arguments.len() - method_copy.arguments.len(),
                        ),
                        cast: None,
                    },
                    movable_types,
                    name_provider,
                )?;
                methods.push(CppFfiItem::from_function(processed_method));
            }
        }
    }

    Ok(methods)
}

/// Creates FFI function signature for this function:
/// - converts all types to FFI types;
/// - adds "this" argument explicitly if present;
/// - adds "output" argument for return value if
///   the return value is stack-allocated.
pub fn to_ffi_method(
    kind: &CppFfiFunctionKind,
    movable_types: &[CppPath],
    name_provider: &mut FfiNameProvider,
) -> Result<CppFfiFunction> {
    let ascii_caption = match &kind {
        CppFfiFunctionKind::Function { cpp_function, .. } => cpp_function.path.ascii_caption(),
        CppFfiFunctionKind::FieldAccessor {
            field,
            accessor_type,
        } => {
            let field_caption = field.path.ascii_caption();
            match *accessor_type {
                CppFieldAccessorType::CopyGetter | CppFieldAccessorType::ConstRefGetter => {
                    field_caption
                }
                CppFieldAccessorType::MutRefGetter => format!("{}_mut", field_caption),
                CppFieldAccessorType::Setter => format!("set_{}", field_caption),
            }
        }
    };

    let mut r = CppFfiFunction {
        arguments: Vec::new(),
        return_type: CppFfiType::void(),
        path: name_provider.create_path(&ascii_caption),
        allocation_place: ReturnValueAllocationPlace::NotApplicable,
        kind: kind.clone(),
    };

    let this_arg_type = match &kind {
        CppFfiFunctionKind::Function { cpp_function, .. } => match &cpp_function.member {
            Some(info) if !info.is_static && info.kind != CppFunctionKind::Constructor => {
                let class_type = CppType::Class(cpp_function.class_type().unwrap());
                Some(CppType::new_pointer(info.is_const, class_type))
            }
            _ => None,
        },
        CppFfiFunctionKind::FieldAccessor {
            field,
            accessor_type,
        } => {
            if field.is_static {
                None
            } else {
                let class_type = CppType::Class(field.path.parent()?);
                let is_const = match *accessor_type {
                    CppFieldAccessorType::CopyGetter | CppFieldAccessorType::ConstRefGetter => true,
                    CppFieldAccessorType::MutRefGetter | CppFieldAccessorType::Setter => false,
                };
                Some(CppType::new_pointer(is_const, class_type))
            }
        }
    };

    if let Some(this_arg_type) = this_arg_type {
        r.arguments.push(CppFfiFunctionArgument {
            name: "this_ptr".to_string(),
            argument_type: this_arg_type.to_cpp_ffi_type(CppTypeRole::NotReturnType)?,
            meaning: CppFfiArgumentMeaning::This,
        });
    }

    let normal_args = match &kind {
        CppFfiFunctionKind::Function { cpp_function, .. } => {
            if cpp_function.allows_variadic_arguments {
                bail!("Variable arguments are not supported");
            }

            if cpp_function.is_destructor() {
                // destructor doesn't have a return type that needs special handling,
                // but its `allocation_place` must match `allocation_place` of the type's constructor
                let class_type = &cpp_function.class_type().unwrap();
                r.allocation_place = if movable_types.iter().any(|t| t == class_type) {
                    ReturnValueAllocationPlace::Stack
                } else {
                    ReturnValueAllocationPlace::Heap
                };
            }
            cpp_function.arguments.clone()
        }
        CppFfiFunctionKind::FieldAccessor {
            field,
            accessor_type,
        } => {
            if accessor_type == &CppFieldAccessorType::Setter {
                let arg = CppFunctionArgument {
                    name: "value".to_string(),
                    argument_type: field.field_type.clone(),
                    has_default_value: false,
                };
                vec![arg]
            } else {
                Vec::new()
            }
        }
    };

    for (index, arg) in normal_args.iter().enumerate() {
        let c_type = arg
            .argument_type
            .to_cpp_ffi_type(CppTypeRole::NotReturnType)?;
        r.arguments.push(CppFfiFunctionArgument {
            name: arg.name.clone(),
            argument_type: c_type,
            meaning: CppFfiArgumentMeaning::Argument(index),
        });
    }

    let real_return_type = match &kind {
        CppFfiFunctionKind::Function { cpp_function, .. } => match &cpp_function.member {
            Some(info) if info.kind.is_constructor() => {
                CppType::Class(cpp_function.class_type().unwrap())
            }
            _ => cpp_function.return_type.clone(),
        },
        CppFfiFunctionKind::FieldAccessor {
            field,
            accessor_type,
        } => match *accessor_type {
            CppFieldAccessorType::CopyGetter => field.field_type.clone(),
            CppFieldAccessorType::ConstRefGetter => {
                CppType::new_reference(true, field.field_type.clone())
            }
            CppFieldAccessorType::MutRefGetter => {
                CppType::new_reference(false, field.field_type.clone())
            }
            CppFieldAccessorType::Setter => CppType::Void,
        },
    };
    let real_return_type_ffi = real_return_type.to_cpp_ffi_type(CppTypeRole::ReturnType)?;
    match &real_return_type {
        // QFlags is converted to uint in FFI
        CppType::Class(path) if !is_qflags(path) => {
            if movable_types.iter().any(|t| t == path) {
                r.arguments.push(CppFfiFunctionArgument {
                    name: "output".to_string(),
                    argument_type: real_return_type_ffi,
                    meaning: CppFfiArgumentMeaning::ReturnValue,
                });
                r.allocation_place = ReturnValueAllocationPlace::Stack;
            } else {
                r.return_type = real_return_type_ffi;
                r.allocation_place = ReturnValueAllocationPlace::Heap;
            }
        }
        _ => {
            r.return_type = real_return_type_ffi;
        }
    }

    Ok(r)
}

/// Adds fictional getter and setter methods for each known public field of each class.
fn generate_field_accessors(
    field: &CppClassField,
    movable_types: &[CppPath],
    name_provider: &mut FfiNameProvider,
) -> Result<Vec<CppFfiItem>> {
    let mut new_methods = Vec::new();
    let mut create_method = |accessor_type| -> Result<CppFfiItem> {
        let kind = CppFfiFunctionKind::FieldAccessor {
            field: field.clone(),
            accessor_type,
        };
        let ffi_function = to_ffi_method(&kind, movable_types, name_provider)?;
        Ok(CppFfiItem::from_function(ffi_function))
    };

    if field.visibility == CppVisibility::Public {
        // Classes may be non-copyable, so copy getters may not be possible for them,
        // so we generate reference getters instead.
        if field.field_type.is_class() {
            new_methods.push(create_method(CppFieldAccessorType::ConstRefGetter)?);
            new_methods.push(create_method(CppFieldAccessorType::MutRefGetter)?);
        } else {
            new_methods.push(create_method(CppFieldAccessorType::CopyGetter)?);
        }
        new_methods.push(create_method(CppFieldAccessorType::Setter)?);
    }

    Ok(new_methods)
}

fn check_preconditions(item: &CppItemData) -> Result<()> {
    match item {
        CppItemData::Function(function) => {
            if let Some(membership) = &function.member {
                if membership.visibility == CppVisibility::Private {
                    bail!("function is private");
                }
                if membership.visibility == CppVisibility::Protected {
                    bail!("function is protected");
                }
                if membership.is_signal {
                    bail!("signals are excluded");
                }
            }
            if function.path.last().template_arguments.is_some() {
                bail!("template functions are excluded");
            }
        }
        CppItemData::ClassField(field) => {
            if field.visibility == CppVisibility::Private {
                bail!("field is private");
            }
            if field.visibility == CppVisibility::Protected {
                bail!("field is protected");
            }
        }
        _ => {}
    }
    if item
        .all_involved_types()
        .iter()
        .any(|x| x.is_or_contains_template_parameter())
    {
        bail!("item contains template parameter");
    }
    Ok(())
}
