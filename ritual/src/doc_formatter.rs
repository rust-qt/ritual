//! Generates markdown code for documentation comments
//! of the output Rust crate.

#![allow(dead_code)]

use crate::cpp_ffi_data::CppFfiFunctionKind;
use crate::cpp_ffi_data::CppFieldAccessorType;
use crate::rust_code_generator::rust_type_to_code;
use crate::rust_info::RustFunctionKind;
use crate::rust_info::RustModuleDoc;
use crate::rust_info::RustStruct;
use crate::rust_info::RustStructKind;
use crate::rust_info::RustWrapperType;
use crate::rust_info::{RustEnumValue, RustFunction, RustQtReceiverType};
use itertools::Itertools;
use log;

pub fn wrap_inline_cpp_code(code: &str) -> String {
    format!("<span style='color: green;'>```{}```</span>", code)
}

pub fn wrap_cpp_doc_block(html: &str) -> String {
    format!(
        "<div style='border: 1px solid #5CFF95; \
         background: #D6FFE4; padding: 16px;'>{}</div>",
        html
    )
}

pub fn module_doc(doc: &RustModuleDoc) -> String {
    let auto_doc = if let Some(ref path) = doc.cpp_path {
        format!(
            "C++ namespace: {}",
            wrap_inline_cpp_code(&path.to_cpp_pseudo_code())
        )
    } else {
        String::new()
    };
    if let Some(ref doc) = doc.extra_doc {
        format!("{}\n\n{}", doc, auto_doc)
    } else {
        auto_doc
    }
}

pub fn struct_doc(type1: &RustStruct) -> String {
    let current_crate = type1
        .path
        .crate_name()
        .expect("generated type's path must have crate name");

    let auto_doc = match type1.kind {
        RustStructKind::WrapperType(RustWrapperType { ref doc_data, .. }) => {
            let cpp_type_code = doc_data.cpp_path.to_cpp_pseudo_code();
            let mut doc = format!("C++ type: {}", wrap_inline_cpp_code(&cpp_type_code));
            // TODO: add description based on the wrapper kind (enum, immovable/movable class)
            if let Some(ref cpp_doc) = doc_data.cpp_doc {
                doc += &format!(
                    "\n\n<a href=\"{}\">C++ documentation:</a> {}",
                    cpp_doc.url,
                    wrap_cpp_doc_block(&cpp_doc.html)
                );
            }

            if let Some(ref slot_wrapper) = doc_data.raw_qt_slot_wrapper {
                let cpp_args = slot_wrapper
                    .cpp_arguments
                    .iter()
                    .map(|t| t.to_cpp_pseudo_code())
                    .join(", ");

                let rust_args = slot_wrapper
                    .rust_arguments
                    .iter()
                    .map(|t| rust_type_to_code(&t.api_type, current_crate))
                    .join(", ");

                doc += &format!(
                    "Allows to bind Qt signals with arguments `({rust_args})` to a \
           Rust extern function.\n\n\
           Corresponding C++ argument types: ({cpp_args}).\n\n
           Use `{public_type_name}` to bind signals to a Rust closure instead.\n\n\
           Create an object using `new()` and bind your function and payload using `set()`. \
           The function will receive the payload as its first arguments, and the rest of arguments \
           will be values passed through the Qt connection system. Use \
           `connect()` method of a `qt_core::connection::Signal` object to connect the signal \
           to this slot. The callback function will be executed each time the slot is invoked \
           until source signals are disconnected or the slot object is destroyed.\n\n\
           If `set()` was not called, slot invokation has no effect.",
                    rust_args = rust_args,
                    cpp_args = cpp_args,
                    public_type_name = slot_wrapper
                        .public_wrapper_path
                        .full_name(Some(current_crate)),
                );
            }
            doc
        }
        RustStructKind::QtSlotWrapper(ref slot_wrapper) => {
            let cpp_args = slot_wrapper
                .cpp_arguments
                .iter()
                .map(|t| t.to_cpp_pseudo_code())
                .join(", ");

            format!("\
                Allows to bind Qt signals with arguments `({cpp_args})` to a Rust closure. \
                \
                Create an object using `new()` and bind your closure using `set()`.\
                The closure will be called with the signal's arguments when the slot is invoked.\
                Use `connect()` method of a `qt_core::connection::Signal` object to connect the signal\
                to this slot. The closure will be executed each time the slot is invoked\
                until source signals are disconnected or the slot object is destroyed.\
                The slot object takes ownership of the passed closure. If `set()` is called again,\
                previously set closure is dropped. Make sure that the slot object does not outlive\
                objects referenced by the closure.\
                If `set()` was not called, slot invokation has no effect.\
            ",
                    cpp_args = cpp_args
            )
        }
        RustStructKind::FfiClassType(ref doc_data) => format!(
            "FFI type corresponding to C++ type: {}.\n\n\
             This type can only be used behind a pointer.\n\n\
             Use `{}` for accessing public API for this type.",
            wrap_inline_cpp_code(&doc_data.cpp_path.to_cpp_pseudo_code()),
            doc_data.public_rust_path.full_name(Some(current_crate))
        ),
        // private struct, no doc needed
        RustStructKind::SizedType(_) => String::new(),
    };
    if let Some(ref doc) = type1.extra_doc {
        format!("{}\n\n{}", doc, auto_doc)
    } else {
        auto_doc
    }
}

pub fn enum_value_doc(value: &RustEnumValue) -> String {
    let mut doc = format!(
        "C++ enum variant: {}",
        wrap_inline_cpp_code(&format!("{} = {}", value.doc.cpp_path.last(), value.value))
    );
    if let Some(ref cpp_doc) = value.doc.cpp_doc {
        doc = format!("{} ({})", cpp_doc, doc);
    }
    if let Some(ref extra_doc) = value.doc.extra_doc {
        format!("{}\n\n{}", extra_doc, doc)
    } else {
        doc
    }
}

pub fn function_doc(function: &RustFunction) -> String {
    let mut doc = Vec::new();

    match function.kind {
        RustFunctionKind::FfiWrapper(ref data) => {
            match data.cpp_ffi_function.kind {
                CppFfiFunctionKind::Function {
                    ref cpp_function,
                    ref omitted_arguments,
                    ..
                } => {
                    doc.push(format!(
                        "Calls C++ function: {}\n\n",
                        wrap_inline_cpp_code(&cpp_function.short_text())
                    ));
                    if let Some(ref omitted_arguments) = *omitted_arguments {
                        // TODO: handle singular/plural form
                        doc.push(format!(
                            "This version of the function omits some arguments ({}).\n\n",
                            omitted_arguments
                        ));
                    }

                    if let Some(ref cpp_doc) = cpp_function.doc {
                        let prefix = if let Some(ref declaration) = cpp_doc.mismatched_declaration {
                            format!(
                                "Warning: no exact match found in C++ documentation. \
                                 Below is the <a href=\"{}\">C++ documentation</a> \
                                 for {}:",
                                cpp_doc.url,
                                wrap_inline_cpp_code(declaration)
                            )
                        } else {
                            format!("<a href=\"{}\">C++ documentation:</a>", cpp_doc.url)
                        };
                        doc.push(format!("{} {}", prefix, wrap_cpp_doc_block(&cpp_doc.html)));
                    }
                }
                CppFfiFunctionKind::FieldAccessor {
                    ref field,
                    ref accessor_type,
                } => {
                    let field_text = wrap_inline_cpp_code(&field.short_text());
                    let text = match *accessor_type {
                        CppFieldAccessorType::CopyGetter | CppFieldAccessorType::ConstRefGetter => {
                            format!("Returns the value of C++ field: {}", field_text)
                        }
                        CppFieldAccessorType::MutRefGetter => format!(
                            "Returns a mutable reference to the C++ field: {}",
                            field_text
                        ),
                        CppFieldAccessorType::Setter => {
                            format!("Sets the value of the C++ field: {}", field_text)
                        }
                    };
                    doc.push(text);
                    // TODO: add C++ docs of fields
                }
            }
        }
        RustFunctionKind::CppDeletableImpl { .. } => {
            // should not need doc because trait doc will be propagated
        }
        RustFunctionKind::SignalOrSlotGetter {
            ref receiver_type,
            ref cpp_path,
            ..
        } => {
            doc.push(format!(
                "Returns an object representing a built-in Qt {signal} `{cpp_path}`.\n\n\
                 Return value of this function can be used for creating Qt connections using \
                 `qt_core::connection` API.",
                signal = match receiver_type {
                    RustQtReceiverType::Signal => "signal",
                    RustQtReceiverType::Slot => "slot",
                },
                cpp_path = cpp_path
            ));
            // TODO: add doc
        }
    }
    // TODO: somehow handle docs for inherited methods (currently only for virtual functions).

    let variant_docs = doc.join("");
    if let Some(ref common_doc) = function.doc.extra_doc {
        format!("{}\n\n{}", common_doc, variant_docs)
    } else {
        variant_docs
    }
}

pub fn slots_module_doc() -> String {
    "Binding Qt signals to Rust closures or extern functions.\n\n\
     Types in this module allow to connect Qt signals with certain argument types \
     to a Rust closure. \n\nThere is one slot type for each distinct set of argument types \
     present in this crate. However, if argument types were present in a dependency crate, \
     the corresponding slot type is located in the dependency's `slots` module."
        .into()
}

pub fn slots_raw_module_doc() -> String {
    "Binding Qt signals to Rust extern functions.\n\n\
     Types in this module to connect Qt signals with certain argument types \
     to a Rust extern function with payload. Raw slots expose low level C++ API and are used \
     to implement the closure slots located in the parent module. Raw slots are less convenient \
     but slightly faster than closure slots.\n\n\
     There is one slot type for each distinct set of argument types \
     present in this crate. However, if argument types were present in a dependency crate, \
     the corresponding slot type is located in the dependency's `slots::raw` module."
        .into()
}

pub fn overloading_module_doc() -> String {
    "Types for emulating overloading for overloaded functions in this module".into()
}

pub fn doc_for_qt_builtin_receivers_struct(rust_type_name: &str, receiver_type: &str) -> String {
    format!(
        "Provides access to built-in Qt {} of `{}`.",
        receiver_type, rust_type_name
    )
}

// TODO: add docs for slot wrapper functions
/*
    for method in methods {
        if method.name.parts.len() != 1 {
            return Err(unexpected("method name should have one part here").into());
        }
        if method.variant_docs.len() != 1 {
            return Err(
                unexpected("method.variant_docs should have one item here").into()
            );
        }
        match method.name.parts[0].as_str() {
            "new" => {
                method.common_doc = Some("Constructs a new object.".into());
            }
            "set" => {
                method.common_doc = Some(
    "Sets `func` as the callback function \
     and `data` as the payload. When the slot is invoked, `func(data)` will be called. \
     Note that it may happen at any time and in any thread."
      .into(),
  );
            }
            "custom_slot" => {
                method.common_doc = Some(
    "Executes the callback function, as if the slot was invoked \
     with these arguments. Does nothing if no callback function was set."
      .into(),
  );
            }
            _ => {
                return Err(unexpected("unknown method for slot wrapper").into());
            }
        }
    }
*/