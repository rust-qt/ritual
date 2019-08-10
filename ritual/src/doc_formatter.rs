//! Generates markdown code for documentation comments
//! of the output Rust crate.

#![allow(dead_code)]

use crate::cpp_ffi_data::{CppFfiFunctionKind, CppFieldAccessorType};
use crate::cpp_type::CppType;
use crate::database::{DatabaseClient, DbItem, DocItem};
use crate::rust_code_generator::rust_type_to_code;
use crate::rust_info::{
    RustEnumValue, RustFunction, RustFunctionKind, RustModule, RustModuleKind, RustQtReceiverType,
    RustSpecialModuleKind, RustStruct, RustStructKind, RustWrapperType,
};
use itertools::Itertools;
use ritual_common::errors::{err_msg, Result};
use std::fmt::Write;

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

pub fn module_doc(module: DbItem<&RustModule>, _database: &DatabaseClient) -> Result<String> {
    let mut output = String::new();
    match module.item.kind {
        RustModuleKind::Special(kind) => match kind {
            RustSpecialModuleKind::CrateRoot => {
                // TODO: generate some useful docs for crate root
                write!(output, "Crate root")?;
            }
            RustSpecialModuleKind::Ffi => {
                write!(output, "Functions provided by the C++ wrapper library")?;
            }
            RustSpecialModuleKind::Ops => {
                write!(output, "Functions that provide access to C++ operators")?;
            }
            RustSpecialModuleKind::SizedTypes => {
                write!(
                    output,
                    "Types with the same size and alignment as corresponding C++ types"
                )?;
            }
        },
        RustModuleKind::CppNamespace { .. } => {
            if let Some(path) = &module.item.doc.cpp_path {
                let cpp_path_text = wrap_inline_cpp_code(&path.to_cpp_pseudo_code());
                write!(output, "C++ namespace: {}", cpp_path_text)?;
            }
        }
        RustModuleKind::CppNestedTypes { .. } => {
            if let Some(path) = &module.item.doc.cpp_path {
                let cpp_path_text = wrap_inline_cpp_code(&path.to_cpp_pseudo_code());
                write!(output, "C++ type: {}", cpp_path_text)?;
            }
        }
    };
    Ok(output)
}

pub fn struct_doc(type1: DbItem<&RustStruct>, database: &DatabaseClient) -> Result<String> {
    let current_crate = database.crate_name();

    let mut output = String::new();

    match &type1.item.kind {
        RustStructKind::WrapperType(RustWrapperType { doc_data, .. }) => {
            let cpp_type_code = doc_data.cpp_path.to_cpp_pseudo_code();
            write!(
                output,
                "Type corresponding to C++ type: {}.\n\n\
                 This type can only be used behind a pointer or reference.",
                wrap_inline_cpp_code(&cpp_type_code)
            )?;
            // TODO: add description based on the wrapper kind (enum, immovable/movable class)

            if let Some(slot_wrapper) = &doc_data.raw_qt_slot_wrapper {
                let cpp_args = slot_wrapper
                    .cpp_arguments
                    .iter()
                    .map(CppType::to_cpp_pseudo_code)
                    .join(", ");

                let rust_args = slot_wrapper
                    .rust_arguments
                    .iter()
                    .map(|t| rust_type_to_code(t.api_type(), Some(current_crate)))
                    .join(", ");

                write!(
                    output,
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
                )?;
            }
        }
        RustStructKind::QtSlotWrapper(slot_wrapper) => {
            let cpp_args = slot_wrapper
                .signal_arguments
                .iter()
                .map(CppType::to_cpp_pseudo_code)
                .join(", ");

            write!(output, "\
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
            )?;
        }
        // private struct, no doc needed
        RustStructKind::SizedType(_) => {}
    };

    if let Some(doc_item) = database.find_doc_for(&type1.id)? {
        write!(output, "{}", format_doc_item(doc_item.item))?;
    }
    Ok(output)
}

pub fn enum_value_doc(value: DbItem<&RustEnumValue>, database: &DatabaseClient) -> Result<String> {
    let cpp_item = database
        .source_cpp_item(&value.id)?
        .ok_or_else(|| err_msg("source cpp item not found"))?
        .item
        .as_enum_value_ref()
        .ok_or_else(|| err_msg("invalid source cpp item type"))?;

    let mut doc = format!(
        "C++ enum variant: {}",
        wrap_inline_cpp_code(&format!(
            "{} = {}",
            cpp_item.path.last().name,
            value.item.value
        ))
    );
    if let Some(doc_item) = database.find_doc_for(&value.id)? {
        doc = format!("{} ({})", doc_item.item.html, doc);
    }
    Ok(doc)
}

fn format_maybe_link(url: &Option<String>, text: &str) -> String {
    if let Some(url) = url {
        format!("<a href=\"{}\">{}</a>", url, text)
    } else {
        text.into()
    }
}

fn format_doc_item(cpp_doc: &DocItem) -> String {
    let mut output = if let Some(declaration) = &cpp_doc.mismatched_declaration {
        format!(
            "Warning: no exact match found in C++ documentation. \
             Below is the {} for {}:",
            format_maybe_link(&cpp_doc.url, "C++ documentation"),
            wrap_inline_cpp_code(declaration)
        )
    } else {
        format_maybe_link(&cpp_doc.url, "C++ documentation")
    };
    write!(output, " {}", wrap_cpp_doc_block(&cpp_doc.html)).unwrap();
    output
}

pub fn function_doc(function: DbItem<&RustFunction>, database: &DatabaseClient) -> Result<String> {
    let mut output = String::new();

    match &function.item.kind {
        RustFunctionKind::FfiWrapper(data) => {
            match &data.cpp_ffi_function.kind {
                CppFfiFunctionKind::Function { cpp_function, .. } => {
                    write!(
                        output,
                        "Calls C++ function: {}\n\n",
                        wrap_inline_cpp_code(&cpp_function.short_text())
                    )?;

                    // TODO: detect omitted arguments using source_id
                    /*if let Some(arguments_before_omitting) =
                        &cpp_function.doc.arguments_before_omitting
                    {
                        // TODO: handle singular/plural form
                        doc.push(format!(
                            "This version of the function omits some arguments ({}).\n\n",
                            arguments_before_omitting.len() - cpp_function.arguments.len()
                        ));
                    }*/
                }
                CppFfiFunctionKind::FieldAccessor {
                    field,
                    accessor_type,
                } => {
                    let field_text = wrap_inline_cpp_code(&field.short_text());
                    match *accessor_type {
                        CppFieldAccessorType::CopyGetter | CppFieldAccessorType::ConstRefGetter => {
                            write!(output, "Returns the value of C++ field: {}", field_text)?;
                        }
                        CppFieldAccessorType::MutRefGetter => {
                            write!(
                                output,
                                "Returns a mutable reference to the C++ field: {}",
                                field_text
                            )?;
                        }
                        CppFieldAccessorType::Setter => {
                            write!(output, "Sets the value of the C++ field: {}", field_text)?;
                        }
                    };
                    // TODO: add C++ docs of fields
                }
            }
        }
        RustFunctionKind::SignalOrSlotGetter(getter) => {
            write!(
                output,
                "Returns an object representing a built-in Qt {signal} `{cpp_path}`.\n\n\
                 Return value of this function can be used for creating Qt connections using \
                 `qt_core::connection` API.",
                signal = match getter.receiver_type {
                    RustQtReceiverType::Signal => "signal",
                    RustQtReceiverType::Slot => "slot",
                },
                cpp_path = getter.cpp_path.to_cpp_pseudo_code()
            )?;
        }
        // FFI functions are private
        RustFunctionKind::FfiFunction => {}
    }
    // TODO: somehow handle docs for inherited methods (currently only for virtual functions).
    if let Some(doc_item) = database.find_doc_for(&function.id)? {
        write!(output, "{}", format_doc_item(doc_item.item))?;
    }
    Ok(output)
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
