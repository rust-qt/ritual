use crate::cpp_data::{CppBaseSpecifier, CppItem, CppPath, CppPathItem};
use crate::cpp_ffi_data::CppCast;
use crate::cpp_function::{CppFunction, CppFunctionArgument, CppFunctionDoc};
use crate::cpp_type::{CppPointerLikeTypeKind, CppType};
use crate::processor::ProcessorData;
use ritual_common::errors::Result;

/// Convenience function to create `CppMethod` object for
/// `static_cast` or `dynamic_cast` from type `from` to type `to`.
/// See `CppMethod`'s documentation for more information
/// about `is_unsafe_static_cast` and `is_direct_static_cast`.
fn create_cast_method(cast: CppCast, from: &CppType, to: &CppType) -> Result<CppItem> {
    let function = CppFunction {
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
        doc: CppFunctionDoc::default(),
        cast: Some(cast),
    };
    Ok(CppItem::Function(function))
}

/// Performs a portion of `generate_casts` operation.
/// Adds casts between `target_type` and `base_type` and calls
/// `generate_casts_one` recursively to add casts between `target_type`
/// and base types of `base_type`.
fn generate_casts_one(
    target_type: &CppPath,
    base_type: &CppPath,
    direct_base_index: Option<usize>,
    data: &ProcessorData<'_>,
) -> Result<Vec<CppItem>> {
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
    )?);
    new_methods.push(create_cast_method(
        CppCast::Static {
            is_unsafe: false,
            base_index: direct_base_index,
        },
        &target_ptr_type,
        &base_ptr_type,
    )?);
    new_methods.push(create_cast_method(
        CppCast::Dynamic,
        &base_ptr_type,
        &target_ptr_type,
    )?);

    for item in data.all_cpp_items().filter_map(|i| i.item.as_base_ref()) {
        if &item.derived_class_type == base_type {
            new_methods.extend(generate_casts_one(
                target_type,
                &item.base_class_type,
                None,
                data,
            )?);
        }
    }

    Ok(new_methods)
}

/// Adds `static_cast` and `dynamic_cast` functions for all appropriate pairs of types
/// in this `CppData`.
fn generate_casts(base: &CppBaseSpecifier, data: &ProcessorData<'_>) -> Result<Vec<CppItem>> {
    generate_casts_one(
        &base.derived_class_type,
        &base.base_class_type,
        Some(base.base_index),
        data,
    )
}

pub fn run(data: &mut ProcessorData<'_>) -> Result<()> {
    let mut results = Vec::new();
    for item in data.current_database.cpp_items() {
        if let CppItem::ClassBase(base) = &item.item {
            for data in generate_casts(base, &data)? {
                results.push((item.source_id, data));
            }
        }
    }
    for (source_ffi_item, item) in results {
        data.current_database.add_cpp_item(source_ffi_item, item);
    }
    Ok(())
}
