use crate::cpp_data::CppClassField;
use crate::cpp_data::CppItem;
use crate::cpp_data::CppPath;
use crate::cpp_data::CppPathItem;
use crate::cpp_data::CppTypeDeclarationKind;
use crate::cpp_data::CppVisibility;
use crate::cpp_ffi_data::CppFfiFunctionArgument;
use crate::cpp_ffi_data::CppFfiType;
use crate::cpp_ffi_data::{CppFfiArgumentMeaning, CppToFfiTypeConversion};
use crate::cpp_ffi_data::{CppFfiFunction, CppFfiFunctionKind, CppFieldAccessorType};
use crate::cpp_function::ReturnValueAllocationPlace;
use crate::cpp_function::{CppFunction, CppFunctionArgument, CppFunctionKind};
use crate::cpp_type::CppPointerLikeTypeKind;
use crate::cpp_type::CppType;
use crate::cpp_type::CppTypeRole;
use crate::cpp_type::{is_qflags, CppFunctionPointerType};
use crate::database::{CppFfiDatabaseItem, CppItemId};
use crate::processor::ProcessorData;
use itertools::Itertools;
use log::{debug, trace};
use ritual_common::errors::{bail, Result, ResultExt};
use std::collections::HashSet;

/// Converts this C++ type to its adaptation for FFI interface,
/// removing all features not supported by C ABI
/// (e.g. references and passing objects by value).
#[allow(clippy::collapsible_if)]
pub fn ffi_type(original_type: &CppType, role: CppTypeRole) -> Result<CppFfiType> {
    let inner = || -> Result<CppFfiType> {
        if original_type.is_or_contains_template_parameter() {
            bail!("template parameters cannot be expressed in FFI");
        }
        let conversion = match original_type {
            CppType::FunctionPointer(CppFunctionPointerType {
                return_type,
                arguments,
                allows_variadic_arguments,
            }) => {
                if *allows_variadic_arguments {
                    bail!("function pointers with variadic arguments are not supported");
                }
                let mut all_types = arguments.iter().collect_vec();
                all_types.push(return_type.as_ref());
                for arg in all_types {
                    match *arg {
                        CppType::FunctionPointer(..) => {
                            bail!(
                                "function pointers containing nested function pointers are \
                                 not supported"
                            );
                        }
                        CppType::Class(..) => {
                            bail!(
                                "Function pointers containing classes by value are not \
                                 supported"
                            );
                        }
                        _ => {}
                    }
                    if arg.contains_reference() {
                        bail!("Function pointers containing references are not supported");
                    }
                }
                CppToFfiTypeConversion::NoChange
            }
            CppType::Class(path) => {
                if is_qflags(&path) {
                    CppToFfiTypeConversion::QFlagsToInt
                } else {
                    CppToFfiTypeConversion::ValueToPointer {
                        is_ffi_const: role != CppTypeRole::ReturnType,
                    }
                }
            }
            CppType::PointerLike {
                kind,
                is_const,
                target,
            } => {
                match *kind {
                    CppPointerLikeTypeKind::Pointer => CppToFfiTypeConversion::NoChange,
                    CppPointerLikeTypeKind::Reference => {
                        match &**target {
                            CppType::Class(path) if *is_const && is_qflags(path) => {
                                // TODO: use a separate conversion type (QFlagsConstRefToUInt)?
                                CppToFfiTypeConversion::QFlagsToInt
                            }
                            _ => CppToFfiTypeConversion::ReferenceToPointer,
                        }
                    }
                    CppPointerLikeTypeKind::RValueReference => {
                        bail!("rvalue references are not supported");
                    }
                }
            }
            _ => CppToFfiTypeConversion::NoChange,
        };
        CppFfiType::new(original_type.clone(), conversion)
    };
    Ok(inner().with_context(|_| format!("Can't express type to FFI: {:?}", original_type))?)
}

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
        .all_cpp_items()
        .filter_map(|item| {
            if let CppItem::Type(type_data) = &item.item {
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

    let all_cpp_item_ids = data.current_database.cpp_item_ids().collect_vec();

    for cpp_item_id in all_cpp_item_ids {
        let item = &data.current_database.cpp_item(cpp_item_id)?;
        if let Err(err) = check_preconditions(&item.item) {
            trace!("skipping {}: {}", item.item, err);
            continue;
        }
        if let Some(hook) = data.config.ffi_generator_hook() {
            if !hook(&item.item)? {
                trace!("skipping {} (by hook)", item.item);
                continue;
            }
        }
        let result = match &item.item {
            CppItem::Function(method) => generate_ffi_methods_for_method(
                method,
                &movable_types,
                cpp_item_id,
                &mut name_provider,
            )
            .map(|v| v.into_iter().collect_vec()),
            CppItem::ClassField(field) => {
                generate_field_accessors(field, &movable_types, cpp_item_id, &mut name_provider)
                    .map(|v| v.into_iter().collect_vec())
            }
            CppItem::ClassBase(_)
            | CppItem::Type(_)
            | CppItem::EnumValue(_)
            | CppItem::Namespace(_) => {
                // no FFI methods for these items
                continue;
            }
        };

        match result {
            Err(error) => {
                debug!("failed to add FFI item: {}: {}", item.item, error);
            }
            Ok(r) => {
                let mut new_items = Vec::new();
                for item in r {
                    if data.current_database.add_ffi_item(item.clone()) {
                        new_items.push(item);
                    }
                }
                let item = &mut data.current_database.cpp_item_mut(cpp_item_id)?;
                if new_items.is_empty() {
                    debug!("no new FFI items for: {}", item.item);
                } else {
                    debug!(
                        "added FFI items (count: {}) for: {}",
                        new_items.len(),
                        item.item
                    );
                    for item in &new_items {
                        trace!("* {:?}", item);
                    }
                }
            }
        }
    }
    Ok(())
}

fn generate_ffi_methods_for_method(
    method: &CppFunction,
    movable_types: &[CppPath],
    cpp_item_id: CppItemId,
    name_provider: &mut FfiNameProvider,
) -> Result<Vec<CppFfiDatabaseItem>> {
    let mut methods = Vec::new();
    methods.push(CppFfiDatabaseItem::from_function(to_ffi_method(
        cpp_item_id,
        &CppFfiFunctionKind::Function {
            cpp_function: method.clone(),
        },
        movable_types,
        name_provider,
    )?));

    Ok(methods)
}

/// Creates FFI function signature for this function:
/// - converts all types to FFI types;
/// - adds "this" argument explicitly if present;
/// - adds "output" argument for return value if
///   the return value is stack-allocated.
pub fn to_ffi_method(
    cpp_item_id: CppItemId,
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
        cpp_item_id,
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
            argument_type: ffi_type(&this_arg_type, CppTypeRole::NotReturnType)?,
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
        let c_type = ffi_type(&arg.argument_type, CppTypeRole::NotReturnType)?;
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
    let real_return_type_ffi = ffi_type(&real_return_type, CppTypeRole::ReturnType)?;
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
    cpp_item_id: CppItemId,
    name_provider: &mut FfiNameProvider,
) -> Result<Vec<CppFfiDatabaseItem>> {
    let mut new_methods = Vec::new();
    let mut create_method = |accessor_type| -> Result<CppFfiDatabaseItem> {
        let kind = CppFfiFunctionKind::FieldAccessor {
            field: field.clone(),
            accessor_type,
        };
        let ffi_function = to_ffi_method(cpp_item_id, &kind, movable_types, name_provider)?;
        Ok(CppFfiDatabaseItem::from_function(ffi_function))
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

fn check_preconditions(item: &CppItem) -> Result<()> {
    match item {
        CppItem::Function(function) => {
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
            if let Some(args) = &function.path.last().template_arguments {
                if args
                    .iter()
                    .any(|arg| arg.is_or_contains_template_parameter())
                {
                    bail!("template functions are excluded");
                }
            }
        }
        CppItem::ClassField(field) => {
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
        .any(CppType::is_or_contains_template_parameter)
    {
        bail!("item contains template parameter");
    }
    Ok(())
}
