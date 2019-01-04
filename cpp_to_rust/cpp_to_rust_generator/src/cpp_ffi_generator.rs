use crate::common::errors::{bail, should_panic_on_unexpected, unexpected, Result};
use crate::common::utils::MapIfOk;
use crate::cpp_data::CppBaseSpecifier;
use crate::cpp_data::CppClassField;
use crate::cpp_data::CppPath;
use crate::cpp_data::CppPathItem;
use crate::cpp_data::CppTypeDataKind;
use crate::cpp_data::CppVisibility;
use crate::cpp_ffi_data::CppFfiArgumentMeaning;
use crate::cpp_ffi_data::CppFfiFunctionArgument;
use crate::cpp_ffi_data::CppFfiItem;
use crate::cpp_ffi_data::CppFfiType;
use crate::cpp_ffi_data::QtSlotWrapper;
use crate::cpp_ffi_data::{CppCast, CppFfiFunction, CppFfiFunctionKind, CppFieldAccessorType};
use crate::cpp_function::ReturnValueAllocationPlace;
use crate::cpp_function::{
    CppFunction, CppFunctionArgument, CppFunctionKind, CppFunctionMemberData,
};
use crate::cpp_type::is_qflags;
use crate::cpp_type::CppFunctionPointerType;
use crate::cpp_type::CppPointerLikeTypeKind;
use crate::cpp_type::CppType;
use crate::cpp_type::CppTypeRole;
use crate::database::CppItemData;
use crate::database::FfiItem;
use crate::processor::ProcessingStep;
use crate::processor::ProcessorData;
use std::iter::once;

pub struct FfiNameProvider {
    prefix: String,
    next_id: u64,
}

impl FfiNameProvider {
    pub fn new(prefix: String, next_id: u64) -> Self {
        FfiNameProvider { prefix, next_id }
    }
    pub fn next_path(&mut self) -> CppPath {
        let id = self.next_id;
        self.next_id += 1;
        let item = CppPathItem::from_str_unchecked(&format!("{}{}", self.prefix, id));
        CppPath::from_item(item)
    }
}

/// Runs the FFI generator
fn run(mut data: &mut ProcessorData) -> Result<()> {
    let cpp_ffi_lib_name = format!("ctr_{}_ffi", &data.config.crate_properties().name());
    let movable_types: Vec<_> = data
        .all_items()
        .iter()
        .filter_map(|item| {
            if let CppItemData::Type(ref type_data) = item.cpp_data {
                if type_data.kind == CppTypeDataKind::Class && type_data.is_movable {
                    return Some(type_data.name.clone());
                }
            }
            None
        })
        .collect();

    let all_class_bases: Vec<_> = data
        .all_items()
        .iter()
        .filter_map(|item| {
            if let CppItemData::ClassBase(ref base) = item.cpp_data {
                Some(base.clone())
            } else {
                None
            }
        })
        .collect();

    // TODO: save and use next_id
    let mut name_provider =
        FfiNameProvider::new(cpp_ffi_lib_name.clone(), data.current_database.next_ffi_id);

    for item in &mut data.current_database.items {
        if item.ffi_items.is_some() {
            data.html_logger.add(
                &[item.cpp_data.to_string(), "already processed".to_string()],
                "already_processed",
            )?;
            continue;
        }
        if !should_process_item(&item.cpp_data)? {
            data.html_logger.add(
                &[item.cpp_data.to_string(), "skipped".to_string()],
                "skipped",
            )?;
            continue;
        }
        let result = match item.cpp_data {
            CppItemData::Type(_) | CppItemData::EnumValue(_) | CppItemData::Namespace(_) => {
                Ok(Vec::new())
                // no FFI methods for these items
            }
            CppItemData::Function(ref method) => {
                generate_ffi_methods_for_method(method, &movable_types, &mut name_provider)
                    .map(|v| v.into_iter().map(Into::into).collect())
            }
            CppItemData::ClassField(ref field) => {
                generate_field_accessors(field, &movable_types, &mut name_provider)
                    .map(|v| v.into_iter().map(Into::into).collect())
            }
            CppItemData::ClassBase(ref base) => {
                generate_casts(base, &all_class_bases, &mut name_provider)
                    .map(|v| v.into_iter().map(Into::into).collect())
            }
            CppItemData::QtSignalArguments(ref signal_arguments) => {
                generate_slot_wrapper(signal_arguments, &mut name_provider)
            }
            CppItemData::TemplateInstantiation(..) => continue,
        };

        match result {
            Err(msg) => {
                item.ffi_items = Some(Vec::new());
                data.html_logger
                    .add(&[item.cpp_data.to_string(), msg.to_string()], "error")?;
            }
            Ok(r) => {
                data.html_logger.add(
                    &[
                        item.cpp_data.to_string(),
                        match r.len() {
                            0 => "no methods".to_string(),
                            1 => format!("added method: {:?}", r[0]),
                            _ => format!("added methods ({}): {:?}", r.len(), r),
                        },
                    ],
                    "success",
                )?;
                item.ffi_items = Some(r.into_iter().map(FfiItem::new).collect());
            }
        }
    }
    data.current_database.next_ffi_id = name_provider.next_id;
    Ok(())
}

pub fn cpp_ffi_generator_step() -> ProcessingStep {
    ProcessingStep::new("cpp_ffi_generator", vec!["cpp_parser".to_string()], run)
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
) -> Result<CppFfiFunction> {
    let method = CppFunction {
        name: CppPath::from_item(CppPathItem {
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
    let mut r = to_ffi_method(&method, &[], name_provider)?;
    let cast1 = cast;
    if let CppFfiFunctionKind::Function { ref mut cast, .. } = r.kind {
        *cast = Some(cast1);
    } else {
        unexpected!("to_ffi_method must return a value with CppFfiFunctionKind::Function",);
    }
    Ok(r)
}

/// Performs a portion of `generate_casts` operation.
/// Adds casts between `target_type` and `base_type` and calls
/// `generate_casts_one` recursively to add casts between `target_type`
/// and base types of `base_type`.
fn generate_casts_one(
    data: &[CppBaseSpecifier],
    target_type: &CppPath,
    base_type: &CppPath,
    direct_base_index: Option<usize>,
    name_provider: &mut FfiNameProvider,
) -> Result<Vec<CppFfiFunction>> {
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
            direct_base_index,
        },
        &base_ptr_type,
        &target_ptr_type,
        name_provider,
    )?);
    new_methods.push(create_cast_method(
        CppCast::Static {
            is_unsafe: false,
            direct_base_index,
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

    for base in data {
        if &base.derived_class_type == base_type {
            new_methods.append(&mut generate_casts_one(
                data,
                target_type,
                &base.base_class_type,
                None,
                name_provider,
            )?);
        }
    }

    Ok(new_methods)
}

/// Adds `static_cast` and `dynamic_cast` functions for all appropriate pairs of types
/// in this `CppData`.
fn generate_casts(
    base: &CppBaseSpecifier,
    data: &[CppBaseSpecifier],
    name_provider: &mut FfiNameProvider,
) -> Result<Vec<CppFfiFunction>> {
    //log::status("Adding cast functions");
    generate_casts_one(
        data,
        &base.derived_class_type,
        &base.base_class_type,
        Some(base.base_index),
        name_provider,
    )
}

fn generate_ffi_methods_for_method(
    method: &CppFunction,
    movable_types: &[CppPath],
    name_provider: &mut FfiNameProvider,
) -> Result<Vec<CppFfiFunction>> {
    let mut methods = Vec::new();
    // TODO: don't use name here at all, generate names for all methods elsewhere
    methods.push(to_ffi_method(method, movable_types, name_provider)?);

    if let Some(last_arg) = method.arguments.last() {
        if last_arg.has_default_value {
            let mut method_copy = method.clone();
            while let Some(arg) = method_copy.arguments.pop() {
                if !arg.has_default_value {
                    break;
                }
                let mut processed_method =
                    to_ffi_method(&method_copy, movable_types, name_provider)?;
                if let CppFfiFunctionKind::Function {
                    ref mut omitted_arguments,
                    ..
                } = processed_method.kind
                {
                    *omitted_arguments = Some(method.arguments.len() - method_copy.arguments.len());
                } else {
                    unexpected!("expected method kind here");
                }
                methods.push(processed_method);
            }
        }
    }

    Ok(methods)
}

/// Creates FFI method signature for this method:
/// - converts all types to FFI types;
/// - adds "this" argument explicitly if present;
/// - adds "output" argument for return value if
///   the return value is stack-allocated.
pub fn to_ffi_method(
    method: &CppFunction,
    movable_types: &[CppPath],
    name_provider: &mut FfiNameProvider,
) -> Result<CppFfiFunction> {
    if method.allows_variadic_arguments {
        bail!("Variable arguments are not supported");
    }
    let mut r = CppFfiFunction {
        arguments: Vec::new(),
        return_type: CppFfiType::void(),
        name: name_provider.next_path(),
        allocation_place: ReturnValueAllocationPlace::NotApplicable,
        kind: CppFfiFunctionKind::Function {
            cpp_function: method.clone(),
            omitted_arguments: None,
            cast: None,
        },
    };
    if let Some(ref info) = method.member {
        if !info.is_static && info.kind != CppFunctionKind::Constructor {
            r.arguments.push(CppFfiFunctionArgument {
                name: "this_ptr".to_string(),
                argument_type: CppType::PointerLike {
                    is_const: false,
                    kind: CppPointerLikeTypeKind::Pointer,
                    target: Box::new(CppType::Class(method.class_type().unwrap())),
                }
                .to_cpp_ffi_type(CppTypeRole::NotReturnType)?,
                meaning: CppFfiArgumentMeaning::This,
            });
        }
    }
    for (index, arg) in method.arguments.iter().enumerate() {
        let c_type = arg
            .argument_type
            .to_cpp_ffi_type(CppTypeRole::NotReturnType)?;
        r.arguments.push(CppFfiFunctionArgument {
            name: arg.name.clone(),
            argument_type: c_type,
            meaning: CppFfiArgumentMeaning::Argument(index as i8),
        });
    }

    if method.is_destructor() {
        // destructor doesn't have a return type that needs special handling,
        // but its `allocation_place` must match `allocation_place` of the type's constructor
        let class_type = &method.class_type().unwrap();
        r.allocation_place = if movable_types.iter().any(|t| t == class_type) {
            ReturnValueAllocationPlace::Stack
        } else {
            ReturnValueAllocationPlace::Heap
        };
    } else {
        let real_return_type = match method.member {
            Some(ref info) if info.kind.is_constructor() => {
                CppType::Class(method.class_type().unwrap())
            }
            _ => method.return_type.clone(),
        };
        let real_return_type_ffi = real_return_type.to_cpp_ffi_type(CppTypeRole::ReturnType)?;
        match real_return_type {
            // QFlags is converted to uint in FFI
            CppType::Class(ref path) if !is_qflags(path) => {
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
    }

    Ok(r)
}

/// Adds fictional getter and setter methods for each known public field of each class.
fn generate_field_accessors(
    field: &CppClassField,
    movable_types: &[CppPath],
    name_provider: &mut FfiNameProvider,
) -> Result<Vec<CppFfiFunction>> {
    // TODO: fix doc generator for field accessors
    //log::status("Adding field accessors");
    let mut new_methods = Vec::new();
    let mut create_method =
        |name, accessor_type, return_type, arguments| -> Result<CppFfiFunction> {
            let fake_method = CppFunction {
                name,
                member: Some(CppFunctionMemberData {
                    kind: CppFunctionKind::Regular,
                    is_virtual: false,
                    is_pure_virtual: false,
                    is_const: match accessor_type {
                        CppFieldAccessorType::CopyGetter | CppFieldAccessorType::ConstRefGetter => {
                            true
                        }
                        CppFieldAccessorType::MutRefGetter | CppFieldAccessorType::Setter => false,
                    },
                    is_static: false,
                    visibility: CppVisibility::Public,
                    is_signal: false,
                    is_slot: false,
                }),
                operator: None,
                return_type,
                arguments,
                allows_variadic_arguments: false,
                declaration_code: None,
                doc: None,
            };
            let mut ffi_method = to_ffi_method(&fake_method, movable_types, name_provider)?;
            ffi_method.kind = CppFfiFunctionKind::FieldAccessor {
                accessor_type,
                field: field.clone(),
            };
            Ok(ffi_method)
        };
    let class_path = field.class_type.clone();

    if field.visibility == CppVisibility::Public {
        if field.field_type.is_class() {
            let type2_const = CppType::PointerLike {
                is_const: true,
                kind: CppPointerLikeTypeKind::Reference,
                target: Box::new(field.field_type.clone()),
            };
            let type2_mut = CppType::PointerLike {
                is_const: false,
                kind: CppPointerLikeTypeKind::Reference,
                target: Box::new(field.field_type.clone()),
            };
            new_methods.push(create_method(
                class_path.with_added(CppPathItem::from_str_unchecked(&field.name)),
                CppFieldAccessorType::ConstRefGetter,
                type2_const,
                Vec::new(),
            )?);
            new_methods.push(create_method(
                class_path.with_added(CppPathItem::from_str_unchecked(&format!(
                    "{}_mut",
                    field.name
                ))),
                CppFieldAccessorType::MutRefGetter,
                type2_mut,
                Vec::new(),
            )?);
        } else {
            new_methods.push(create_method(
                class_path.with_added(CppPathItem::from_str_unchecked(&field.name)),
                CppFieldAccessorType::CopyGetter,
                field.field_type.clone(),
                Vec::new(),
            )?);
        }
        let arg = CppFunctionArgument {
            argument_type: field.field_type.clone(),
            name: "value".to_string(),
            has_default_value: false,
        };
        new_methods.push(create_method(
            class_path.with_added(CppPathItem::from_str_unchecked(&format!(
                "set_{}",
                field.name
            ))),
            CppFieldAccessorType::Setter,
            CppType::Void,
            vec![arg],
        )?);
    }

    Ok(new_methods)
}

fn should_process_item(item: &CppItemData) -> Result<bool> {
    if let CppItemData::Function(ref method) = *item {
        if let Some(class_name) = method.class_type() {
            if is_qflags(&class_name) {
                return Ok(false);
            }
        }
        if let Some(ref membership) = method.member {
            if membership.visibility == CppVisibility::Private {
                return Ok(false);
            }
            if membership.visibility == CppVisibility::Protected {
                return Ok(false);
            }
            if membership.is_signal {
                return Ok(false);
            }
        }
        if method.name.last().template_arguments.is_some() {
            return Ok(false);
        }
    }
    if item
        .all_involved_types()
        .iter()
        .any(|x| x.is_or_contains_template_parameter())
    {
        return Ok(false);
    }
    Ok(true)

    //if method.template_arguments_values.is_some() && !method.is_ffi_whitelisted {
    // TODO: re-enable after template test compilation (#24) is implemented
    // TODO: QObject::findChild and QObject::findChildren should be allowed
    //return Ok(false);
    //}
}

/// Generates slot wrappers for all encountered argument types
/// (excluding types already handled in the dependencies).
#[allow(dead_code)]
fn generate_slot_wrapper(
    arguments: &[CppType],
    name_provider: &mut FfiNameProvider,
) -> Result<Vec<CppFfiItem>> {
    let ffi_types = arguments.map_if_ok(|t| t.to_cpp_ffi_type(CppTypeRole::NotReturnType))?;

    let void_ptr = CppType::PointerLike {
        is_const: false,
        kind: CppPointerLikeTypeKind::Pointer,
        target: Box::new(CppType::Void),
    };
    let func_arguments = once(void_ptr.clone())
        .chain(ffi_types.iter().map(|t| t.ffi_type.clone()))
        .collect();
    let class_path = name_provider.next_path();
    let function_type = CppFunctionPointerType {
        return_type: Box::new(CppType::Void),
        arguments: func_arguments,
        allows_variadic_arguments: false,
    };
    let create_function = |kind: CppFunctionKind,
                           name: CppPath,
                           is_slot: bool,
                           arguments: Vec<CppFunctionArgument>|
     -> CppFunction {
        CppFunction {
            name,
            member: Some(CppFunctionMemberData {
                is_virtual: true,
                is_pure_virtual: false,
                is_const: false,
                is_static: false,
                visibility: CppVisibility::Public,
                is_signal: false,
                is_slot,
                kind,
            }),
            operator: None,
            return_type: CppType::Void,
            arguments,
            allows_variadic_arguments: false,
            declaration_code: None,
            doc: None,
        }
    };
    let mut methods = Vec::new();
    methods.push(create_function(
        CppFunctionKind::Constructor,
        class_path.with_added(CppPathItem::from_str_unchecked(&class_path.last().name)),
        false,
        vec![],
    ));
    methods.push(create_function(
        CppFunctionKind::Destructor,
        class_path.with_added(CppPathItem::from_str_unchecked(&format!(
            "~{}",
            class_path.last().name
        ))),
        false,
        vec![],
    ));
    let method_set_args = vec![
        CppFunctionArgument {
            name: "func".to_string(),
            argument_type: CppType::FunctionPointer(function_type.clone()),
            has_default_value: false,
        },
        CppFunctionArgument {
            name: "data".to_string(),
            argument_type: void_ptr.clone(),
            has_default_value: false,
        },
    ];
    methods.push(create_function(
        CppFunctionKind::Regular,
        class_path.with_added(CppPathItem::from_str_unchecked("set")),
        false,
        method_set_args,
    ));

    let method_custom_slot = create_function(
        CppFunctionKind::Regular,
        class_path.with_added(CppPathItem::from_str_unchecked("custom_slot")),
        true,
        arguments
            .iter()
            .enumerate()
            .map(|(num, t)| CppFunctionArgument {
                name: format!("arg{}", num),
                argument_type: t.clone(),
                has_default_value: false,
            })
            .collect(),
    );
    let receiver_id = method_custom_slot.receiver_id()?;
    methods.push(method_custom_slot);
    let class_bases = vec![CppBaseSpecifier {
        derived_class_type: class_path.clone(),
        base_class_type: CppPath::from_str_unchecked("QObject"),
        base_index: 0,
        is_virtual: false,
        visibility: CppVisibility::Public,
    }];

    let qt_slot_wrapper = QtSlotWrapper {
        class_name: class_path.clone(),
        arguments: ffi_types,
        function_type: function_type.clone(),
        receiver_id,
    };

    let mut items = vec![qt_slot_wrapper.into()];
    for method in methods {
        items.extend(
            generate_ffi_methods_for_method(&method, &[], name_provider)?
                .into_iter()
                .map(Into::into),
        );
    }
    items.extend(
        generate_casts(&class_bases[0], &class_bases, name_provider)?
            .into_iter()
            .map(Into::into),
    );
    Ok(items)
}
