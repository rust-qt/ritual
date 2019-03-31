use crate::cpp_checks::Condition;
use crate::cpp_ffi_data::{
    CppFfiArgumentMeaning, CppFfiFunctionKind, CppFfiType, CppFieldAccessorType,
    CppToFfiTypeConversion, QtSlotWrapper,
};
use crate::cpp_ffi_data::{CppFfiFunction, CppFfiItem};
use crate::cpp_function::ReturnValueAllocationPlace;
use crate::cpp_type::CppPointerLikeTypeKind;
use crate::cpp_type::CppType;
use crate::database::CppFfiDatabaseItem;
use crate::rust_info::{RustDatabase, RustItem, RustStructKind};
use itertools::Itertools;
use ritual_common::cpp_lib_builder::version_to_number;
use ritual_common::errors::{bail, err_msg, Result};
use ritual_common::file_utils::{create_file, create_file_for_append, os_str_to_str, path_to_str};
use ritual_common::target::LibraryTarget;
use ritual_common::utils::{get_command_output, MapIfOk};
use std::io::Write;
use std::iter::once;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Generates function name, return type and arguments list
/// as it appears in both function declaration and implementation.
fn function_signature(method: &CppFfiFunction) -> Result<String> {
    let mut arg_texts = Vec::new();
    for arg in &method.arguments {
        arg_texts.push(arg.to_cpp_code()?);
    }
    let name_with_args = format!("{}({})", method.path.to_cpp_code()?, arg_texts.join(", "));
    let return_type = method.return_type.ffi_type();
    let r = if let CppType::FunctionPointer(..) = return_type {
        return_type.to_cpp_code(Some(&name_with_args))?
    } else {
        format!("{} {}", return_type.to_cpp_code(None)?, name_with_args)
    };
    Ok(r)
}

/// Generates code for a Qt slot wrapper
pub fn qt_slot_wrapper(wrapper: &QtSlotWrapper) -> Result<String> {
    let func_type = CppType::FunctionPointer(wrapper.function_type.clone());
    let method_args = wrapper
        .arguments
        .iter()
        .enumerate()
        .map_if_ok(|(num, t)| -> Result<_> {
            Ok(format!(
                "{} arg{}",
                t.original_type().to_cpp_code(None)?,
                num
            ))
        })?
        .join(", ");
    let func_args = once("m_data".to_string())
        .chain(
            wrapper
                .arguments
                .iter()
                .enumerate()
                .map_if_ok(|(num, t)| convert_type_to_ffi(t, format!("arg{}", num)))?,
        )
        .join(", ");
    Ok(format!(
        include_str!("../templates/c_lib/qt_slot_wrapper.h"),
        class_name = wrapper.class_path.to_cpp_code()?,
        func_arg = func_type.to_cpp_code(Some("func"))?,
        func_field = func_type.to_cpp_code(Some("m_func"))?,
        method_args = method_args,
        func_args = func_args
    ))
}

/// Generates code that wraps `expression` of type `type1.original_type` and
/// converts it to type `type1.ffi_type`
fn convert_type_to_ffi(type1: &CppFfiType, expression: String) -> Result<String> {
    Ok(match type1.conversion() {
        CppToFfiTypeConversion::NoChange => expression,
        CppToFfiTypeConversion::ValueToPointer { .. } => format!(
            "new {}({})",
            type1.original_type().to_cpp_code(None)?,
            expression
        ),
        CppToFfiTypeConversion::ReferenceToPointer => format!("&{}", expression),
        CppToFfiTypeConversion::QFlagsToInt => format!("int({})", expression),
    })
}

/// Wraps `expression` returned by the original C++ method to
/// convert it to return type of the FFI method.
fn convert_return_type(method: &CppFfiFunction, expression: String) -> Result<String> {
    let mut result = expression;
    match method.return_type.conversion() {
        CppToFfiTypeConversion::NoChange => {}
        CppToFfiTypeConversion::ValueToPointer { .. } => {
            match method.allocation_place {
                ReturnValueAllocationPlace::Stack => {
                    bail!("stack allocated wrappers are expected to return void");
                }
                ReturnValueAllocationPlace::NotApplicable => {
                    bail!("ValueToPointer conflicts with NotApplicable");
                }
                ReturnValueAllocationPlace::Heap => {
                    // constructors are said to return values in parse result,
                    // but in reality we use `new` which returns a pointer,
                    // so no conversion is necessary for constructors.
                    if !method
                        .kind
                        .cpp_function()
                        .map_or(false, |m| m.is_constructor())
                    {
                        result = format!(
                            "new {}({})",
                            method.return_type.original_type().to_cpp_code(None)?,
                            result
                        );
                    }
                }
            }
        }
        CppToFfiTypeConversion::ReferenceToPointer => {
            result = format!("&{}", result);
        }
        CppToFfiTypeConversion::QFlagsToInt => {
            result = format!("int({})", result);
        }
    }

    if method.allocation_place == ReturnValueAllocationPlace::Stack
        && !method
            .kind
            .cpp_function()
            .map_or(false, |m| m.is_constructor())
    {
        if let Some(arg) = method
            .arguments
            .iter()
            .find(|x| x.meaning == CppFfiArgumentMeaning::ReturnValue)
        {
            let type1 = arg.argument_type.ffi_type().pointer_like_to_target()?;
            result = format!("new({}) {}({})", arg.name, type1.to_cpp_code(None)?, result);
        }
    }
    Ok(result)
}

/// Generates code for values passed to the original C++ method.
fn arguments_values(method: &CppFfiFunction) -> Result<String> {
    let r = method
        .arguments
        .iter()
        .filter(|arg| arg.meaning.is_argument())
        .map_if_ok(|argument| -> Result<_> {
            let mut result = argument.name.clone();
            match argument.argument_type.conversion() {
                CppToFfiTypeConversion::ValueToPointer { .. }
                | CppToFfiTypeConversion::ReferenceToPointer => result = format!("*{}", result),
                CppToFfiTypeConversion::NoChange => {}
                CppToFfiTypeConversion::QFlagsToInt => {
                    let type_text = if let CppType::PointerLike {
                        kind,
                        is_const,
                        target,
                    } = argument.argument_type.original_type()
                    {
                        if *kind == CppPointerLikeTypeKind::Reference && *is_const {
                            target.to_cpp_code(None)?
                        } else {
                            bail!("Unsupported original type for QFlagsToUInt conversion");
                        }
                    } else {
                        argument.argument_type.original_type().to_cpp_code(None)?
                    };
                    result = format!("{}({})", type_text, result);
                }
            }
            Ok(result)
        })?;
    Ok(r.join(", "))
}

/// Generates code for the value returned by the FFI method.
#[allow(clippy::collapsible_if)]
fn returned_expression(method: &CppFfiFunction) -> Result<String> {
    let result = if method
        .kind
        .cpp_function()
        .map_or(false, |m| m.is_destructor())
    {
        if let Some(arg) = method
            .arguments
            .iter()
            .find(|x| x.meaning == CppFfiArgumentMeaning::This)
        {
            format!("ritual_call_destructor({})", arg.name)
        } else {
            bail!("no this arg in destructor");
        }
    } else {
        let result_without_args =
            if let Some(cpp_function) = method.kind.cpp_function().filter(|m| m.is_constructor()) {
                match method.allocation_place {
                    ReturnValueAllocationPlace::Stack => {
                        if let Some(arg) = method
                            .arguments
                            .iter()
                            .find(|x| x.meaning == CppFfiArgumentMeaning::ReturnValue)
                        {
                            format!(
                                "new({}) {}",
                                arg.name,
                                cpp_function.class_type()?.to_cpp_code()?
                            )
                        } else {
                            bail!("return value argument not found\n{:?}", method);
                        }
                    }
                    ReturnValueAllocationPlace::Heap => {
                        format!("new {}", cpp_function.class_type()?.to_cpp_code()?)
                    }
                    ReturnValueAllocationPlace::NotApplicable => {
                        bail!("NotApplicable in constructor");
                    }
                }
            } else {
                let path = match &method.kind {
                    CppFfiFunctionKind::Function { cpp_function, .. } => &cpp_function.path,
                    CppFfiFunctionKind::FieldAccessor { field, .. } => &field.path,
                };

                if let Some(arg) = method
                    .arguments
                    .iter()
                    .find(|x| x.meaning == CppFfiArgumentMeaning::This)
                {
                    format!("{}->{}", arg.name, path.last().to_cpp_code()?)
                } else {
                    path.to_cpp_code()?
                }
            };
        if let CppFfiFunctionKind::FieldAccessor { accessor_type, .. } = &method.kind {
            if accessor_type == &CppFieldAccessorType::Setter {
                format!("{} = {}", result_without_args, arguments_values(method)?)
            } else {
                result_without_args
            }
        } else {
            format!("{}({})", result_without_args, arguments_values(method)?)
        }
    };
    convert_return_type(method, result)
}

/// Generates body of the FFI method implementation.
fn source_body(method: &CppFfiFunction) -> Result<String> {
    if method
        .kind
        .cpp_function()
        .map_or(false, |m| m.is_destructor())
        && method.allocation_place == ReturnValueAllocationPlace::Heap
    {
        if let Some(arg) = method
            .arguments
            .iter()
            .find(|x| x.meaning == CppFfiArgumentMeaning::This)
        {
            Ok(format!("delete {};\n", arg.name))
        } else {
            panic!("Error: no this argument found\n{:?}", method);
        }
    } else {
        Ok(format!(
            "{}{};\n",
            if method.return_type.ffi_type().is_void() {
                ""
            } else {
                "return "
            },
            returned_expression(&method)?
        ))
    }
}

/// Generates implementation of the FFI method for the source file.
pub fn function_implementation(method: &CppFfiFunction) -> Result<String> {
    Ok(format!(
        "RITUAL_EXPORT {} {{\n  {}}}\n\n",
        function_signature(method)?,
        source_body(&method)?
    ))
}

fn condition_expression(condition: &Condition) -> String {
    match condition {
        Condition::CppLibraryVersion(version) => {
            let value = version_to_number(version).expect("version_to_number failed");
            format!("RITUAL_CPP_LIB_VERSION == {}", value)
        }
        Condition::Arch(_) => unimplemented!(),
        Condition::OS(_) => unimplemented!(),
        Condition::Family(_) => unimplemented!(),
        Condition::Env(_) => unimplemented!(),
        Condition::PointerWidth(_) => unimplemented!(),
        Condition::Endian(_) => unimplemented!(),
        Condition::And(conditions) => conditions
            .iter()
            .map(|c| format!("({})", condition_expression(c)))
            .join("&&"),
        Condition::Or(conditions) => conditions
            .iter()
            .map(|c| format!("({})", condition_expression(c)))
            .join("||"),
        Condition::Not(condition) => format!("!({})", condition_expression(condition)),
        Condition::True => "true".to_string(),
        Condition::False => "false".to_string(),
    }
}

fn wrap_with_condition(code: &str, condition: &Condition) -> String {
    if condition == &Condition::True {
        return code.to_string();
    }
    format!(
        "#if {}\n{}\n#endif\n",
        condition_expression(condition),
        code
    )
}

/// Generates a source file with the specified FFI methods.
pub fn generate_cpp_file(
    ffi_items: &[CppFfiDatabaseItem],
    environments: &[LibraryTarget],
    file_path: &Path,
    global_header_name: &str,
    crate_name: &str,
) -> Result<()> {
    let mut cpp_file = create_file(file_path)?;
    writeln!(cpp_file, "#include \"{}\"", global_header_name)?;

    let mut any_slot_wrappers = false;
    for ffi_item in ffi_items {
        if !ffi_item.checks.any_success() {
            continue;
        }
        if let CppFfiItem::QtSlotWrapper(qt_slot_wrapper) = &ffi_item.item {
            any_slot_wrappers = true;
            let condition = ffi_item.checks.condition(environments);
            let code = self::qt_slot_wrapper(qt_slot_wrapper)?;
            write!(cpp_file, "{}", wrap_with_condition(&code, &condition))?;
        }
    }

    writeln!(cpp_file, "extern \"C\" {{")?;
    for ffi_item in ffi_items {
        if !ffi_item.checks.any_success() {
            continue;
        }
        if let CppFfiItem::Function(cpp_ffi_function) = &ffi_item.item {
            let condition = ffi_item.checks.condition(environments);
            let code = function_implementation(cpp_ffi_function)?;
            writeln!(cpp_file, "{}", wrap_with_condition(&code, &condition))?;
        }
    }
    writeln!(cpp_file, "}} // extern \"C\"")?;

    if any_slot_wrappers && !crate_name.starts_with("moqt_") {
        let stem = file_path
            .file_stem()
            .ok_or_else(|| err_msg("failed to get file stem"))?;
        writeln!(cpp_file, "#include \"{}.moc\"", os_str_to_str(stem)?)?;
    }
    Ok(())
}

pub fn apply_moc(file_path: &Path) -> Result<()> {
    let moc_output = get_command_output(Command::new("moc").arg("-i").arg(file_path))?;
    let mut cpp_file = create_file_for_append(file_path)?;
    writeln!(
        cpp_file,
        "// start of MOC generated code\n{}\n// end of MOC generated code",
        moc_output
    )?;
    Ok(())
}

/// Generates a C++ program that determines sizes of target C++ types
/// on the current platform and outputs the Rust code for `sized_types.rs` module
/// to the standard output.
pub fn generate_cpp_type_size_requester(
    rust_database: &RustDatabase,
    include_directives: &[PathBuf],
    mut output: impl Write,
) -> Result<()> {
    for dir in include_directives {
        writeln!(output, "#include <{}>", path_to_str(dir)?)?;
    }
    writeln!(output, "#include <stdio.h>\n\nint main() {{")?;

    for item in rust_database.items() {
        if let RustItem::Struct(data) = &item.item {
            if let RustStructKind::SizedType(cpp_path) = &data.kind {
                let cpp_path_code = cpp_path.to_cpp_code()?;

                writeln!(
                    output,
                    "printf(\"#[repr(C, align(%zu))]\\n\", alignof({}));",
                    cpp_path_code
                )?;

                writeln!(
                    output,
                    "printf(\"pub struct {}([u8; %zu]);\\n\\n\", sizeof({}));",
                    data.path.last(),
                    cpp_path_code
                )?;
            }
        }
    }

    writeln!(output, "}}")?;
    Ok(())
}
