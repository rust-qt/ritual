use crate::detect_signal_argument_types::detect_signal_argument_types;
use itertools::Itertools;
use log::trace;
use ritual::cpp_ffi_data::QtSlotWrapper;
use ritual::cpp_ffi_generator::FfiNameProvider;
use ritual::cpp_type::CppFunctionPointerType;
use ritual::cpp_type::CppPointerLikeTypeKind;
use ritual::cpp_type::CppType;
use ritual::cpp_type::CppTypeRole;
use ritual::database::CppFfiItem;
use ritual::database::CppFfiItemKind;
use ritual::processor::ProcessorData;
use ritual_common::errors::Result;
use ritual_common::utils::MapIfOk;
use std::iter::once;

/// Generates slot wrappers for all encountered argument types
/// (excluding types already handled in the dependencies).
fn generate_slot_wrapper(
    arguments: &[CppType],
    name_provider: &mut FfiNameProvider,
) -> Result<QtSlotWrapper> {
    let ffi_types = arguments.map_if_ok(|t| t.to_cpp_ffi_type(CppTypeRole::NotReturnType))?;
    let class_path = name_provider.create_path(&format!(
        "slot_wrapper_{}",
        arguments.iter().map(|arg| arg.ascii_caption()).join("_")
    ));

    let void_ptr = CppType::PointerLike {
        is_const: false,
        kind: CppPointerLikeTypeKind::Pointer,
        target: Box::new(CppType::Void),
    };
    let func_arguments = once(void_ptr.clone())
        .chain(ffi_types.iter().map(|t| t.ffi_type.clone()))
        .collect();

    let function_type = CppFunctionPointerType {
        return_type: Box::new(CppType::Void),
        arguments: func_arguments,
        allows_variadic_arguments: false,
    };

    let qt_slot_wrapper = QtSlotWrapper {
        signal_arguments: arguments.to_vec(),
        class_path: class_path.clone(),
        arguments: ffi_types,
        function_type: function_type.clone(),
    };
    Ok(qt_slot_wrapper)
}

pub fn add_slot_wrappers(data: &mut ProcessorData) -> Result<()> {
    let all_types = detect_signal_argument_types(data)?;

    let mut name_provider = FfiNameProvider::new(data);

    for arg_types in all_types {
        let found = data
            .all_databases()
            .flat_map(|db| &db.ffi_items)
            .any(|ffi_item| {
                if let CppFfiItemKind::QtSlotWrapper(data) = &ffi_item.kind {
                    data.signal_arguments == arg_types
                } else {
                    false
                }
            });
        if found {
            trace!("slot wrapper already exists: {:?}", arg_types);
        } else {
            match generate_slot_wrapper(&arg_types, &mut name_provider) {
                Ok(slot_wrapper) => {
                    data.current_database
                        .ffi_items
                        .push(CppFfiItem::from_qt_slot_wrapper(slot_wrapper));
                    trace!("adding slot wrapper for args: {:?}", arg_types);
                }
                Err(err) => {
                    trace!(
                        "failed to add slot wrapper for args: {:?}: {}",
                        arg_types,
                        err
                    );
                }
            }
        }
    }
    Ok(())
}
