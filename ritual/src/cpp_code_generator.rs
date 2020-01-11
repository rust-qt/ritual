use crate::config::Config;
use crate::cpp_checks::Condition;
use crate::cpp_ffi_data::{
    CppFfiArgumentMeaning, CppFfiFunctionKind, CppFfiType, CppFieldAccessorType,
    CppToFfiTypeConversion, QtSignalWrapper, QtSlotWrapper,
};
use crate::cpp_ffi_data::{CppFfiFunction, CppFfiItem};
use crate::cpp_function::{CppFunction, ReturnValueAllocationPlace};
use crate::cpp_type::CppPointerLikeTypeKind;
use crate::cpp_type::CppType;
use crate::database::{DatabaseClient, DbItem};
use crate::rust_info::{RustItem, RustStructKind};
use itertools::Itertools;
use ritual_common::cpp_lib_builder::version_to_number;
use ritual_common::errors::{bail, err_msg, format_err, Result};
use ritual_common::file_utils::{create_file, os_str_to_str, path_to_str, read_dir};
use ritual_common::utils::MapIfOk;
use std::collections::HashSet;
use std::io::Write;
use std::iter::once;
use std::path::{Path, PathBuf};

struct Generator<'a>(&'a DatabaseClient);

impl Generator<'_> {
    /// Generates function name, return type and arguments list
    /// as it appears in both function declaration and implementation.
    fn function_signature(&self, method: &CppFfiFunction) -> Result<String> {
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
    fn qt_slot_wrapper(&self, wrapper: &QtSlotWrapper) -> Result<String> {
        let func_type = CppType::FunctionPointer(wrapper.function_type.clone());
        let method_args = wrapper
            .arguments
            .iter()
            .enumerate()
            .map_if_ok(|(num, t)| -> Result<_> {
                let arg_type = t.original_type().to_cpp_code(None)?;
                let arg_type = CppFunction::patch_receiver_argument_type(&arg_type);
                Ok(format!("{} arg{}", arg_type, num))
            })?
            .join(", ");
        let func_args = once("m_callback.data()".to_string())
            .chain(
                wrapper
                    .arguments
                    .iter()
                    .enumerate()
                    .map_if_ok(|(num, t)| self.convert_type_to_ffi(t, format!("arg{}", num)))?,
            )
            .join(", ");
        Ok(format!(
            include_str!("../templates/c_lib/qt_slot_wrapper.h"),
            class_name = wrapper.class_path.to_cpp_code()?,
            callback_arg = func_type.to_cpp_code(Some("callback"))?,
            callback_type = func_type.to_cpp_code(Some(""))?,
            method_args = method_args,
            func_args = func_args
        ))
    }

    /// Generates code for a Qt signal wrapper
    fn qt_signal_wrapper(&self, wrapper: &QtSignalWrapper) -> Result<String> {
        let method_args = wrapper
            .signal_arguments
            .iter()
            .enumerate()
            .map_if_ok(|(num, t)| -> Result<_> {
                let arg_type = t.to_cpp_code(None)?;
                let arg_type = CppFunction::patch_receiver_argument_type(&arg_type);
                Ok(format!("{} arg{}", arg_type, num))
            })?
            .join(", ");
        Ok(format!(
            include_str!("../templates/c_lib/qt_signal_wrapper.h"),
            class_name = wrapper.class_path.to_cpp_code()?,
            method_args = method_args,
            signal_impl = if self.0.crate_name().starts_with("moqt_") {
                "{}"
            } else {
                ";"
            }
        ))
    }

    /// Generates code that wraps `expression` of type `type1.original_type` and
    /// converts it to type `type1.ffi_type`
    fn convert_type_to_ffi(&self, type1: &CppFfiType, expression: String) -> Result<String> {
        Ok(match type1.conversion() {
            CppToFfiTypeConversion::NoChange | CppToFfiTypeConversion::ImplicitCast { .. } => {
                expression
            }
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
    fn convert_return_type(
        &self,
        item: DbItem<&CppFfiFunction>,
        expression: String,
    ) -> Result<String> {
        let cpp_item = self
            .0
            .source_cpp_item(&item.id)?
            .ok_or_else(|| format_err!("failed to find original cpp item for {:?}", item))?;
        let is_constructor = cpp_item
            .item
            .as_function_ref()
            .map_or(false, |f| f.is_constructor());

        let method = item.item;
        let mut result = expression;
        match method.return_type.conversion() {
            CppToFfiTypeConversion::NoChange | CppToFfiTypeConversion::ImplicitCast { .. } => {}
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
                        if !is_constructor {
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

        if method.allocation_place == ReturnValueAllocationPlace::Stack && !is_constructor {
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
    fn arguments_values(&self, method: &CppFfiFunction) -> Result<String> {
        let r = method
            .arguments
            .iter()
            .filter(|arg| arg.meaning.is_argument())
            .map_if_ok(|argument| -> Result<_> {
                let mut result = argument.name.clone();
                match argument.argument_type.conversion() {
                    CppToFfiTypeConversion::ValueToPointer { .. }
                    | CppToFfiTypeConversion::ReferenceToPointer => result = format!("*{}", result),
                    CppToFfiTypeConversion::NoChange
                    | CppToFfiTypeConversion::ImplicitCast { .. } => {}
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
    fn returned_expression(&self, item: DbItem<&CppFfiFunction>) -> Result<String> {
        let cpp_item = self
            .0
            .source_cpp_item(&item.id)?
            .ok_or_else(|| format_err!("failed to find original cpp item for {:?}", item))?;
        let is_destructor = cpp_item
            .item
            .as_function_ref()
            .map_or(false, |f| f.is_destructor());

        let method = item.item;
        let result = if is_destructor {
            if let Some(arg) = method
                .arguments
                .iter()
                .find(|x| x.meaning == CppFfiArgumentMeaning::This)
            {
                format!("ritual::call_destructor({})", arg.name)
            } else {
                bail!("no this arg in destructor");
            }
        } else {
            let result_without_args = if let Some(cpp_function) = cpp_item
                .item
                .as_function_ref()
                .filter(|m| m.is_constructor())
            {
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
                                cpp_function.class_path()?.to_cpp_code()?
                            )
                        } else {
                            bail!("return value argument not found\n{:?}", method);
                        }
                    }
                    ReturnValueAllocationPlace::Heap => {
                        format!("new {}", cpp_function.class_path()?.to_cpp_code()?)
                    }
                    ReturnValueAllocationPlace::NotApplicable => {
                        bail!("NotApplicable in constructor");
                    }
                }
            } else {
                let path = cpp_item.item.path().ok_or_else(|| {
                    err_msg("cpp item (function or field) expected to have a path")
                })?;

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
                    format!(
                        "{} = {}",
                        result_without_args,
                        self.arguments_values(method)?
                    )
                } else {
                    result_without_args
                }
            } else {
                format!(
                    "{}({})",
                    result_without_args,
                    self.arguments_values(method)?
                )
            }
        };
        self.convert_return_type(item, result)
    }

    /// Generates body of the FFI method implementation.
    fn source_body(&self, item: DbItem<&CppFfiFunction>) -> Result<String> {
        let cpp_item = self
            .0
            .source_cpp_item(&item.id)?
            .ok_or_else(|| format_err!("failed to find original cpp item for {:?}", item))?;

        let is_destructor = cpp_item
            .item
            .as_function_ref()
            .map_or(false, |f| f.is_destructor());

        let method = item.item;
        if is_destructor && method.allocation_place == ReturnValueAllocationPlace::Heap {
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
                self.returned_expression(item)?
            ))
        }
    }

    /// Generates implementation of the FFI method for the source file.
    fn function_implementation(&self, method: DbItem<&CppFfiFunction>) -> Result<String> {
        Ok(format!(
            "RITUAL_EXPORT {} {{\n  {}}}\n\n",
            self.function_signature(method.item)?,
            self.source_body(method)?
        ))
    }

    fn condition_expression(&self, condition: &Condition) -> String {
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
                .map(|c| format!("({})", self.condition_expression(c)))
                .join("&&"),
            Condition::Or(conditions) => conditions
                .iter()
                .map(|c| format!("({})", self.condition_expression(c)))
                .join("||"),
            Condition::Not(condition) => format!("!({})", self.condition_expression(condition)),
            Condition::True => "true".to_string(),
            Condition::False => "false".to_string(),
        }
    }

    fn wrap_with_condition(&self, code: &str, condition: &Condition) -> String {
        if condition == &Condition::True {
            return code.to_string();
        }
        format!(
            "#if {}\n{}\n#endif\n",
            self.condition_expression(condition),
            code
        )
    }

    /// Generates a source file with the specified FFI methods.
    fn generate_cpp_file(&self, file_path: &Path, global_header_name: &str) -> Result<()> {
        let mut cpp_file = create_file(file_path)?;
        writeln!(cpp_file, "#include \"{}\"", global_header_name)?;

        let used_ffi_functions = self
            .0
            .rust_items()
            .filter_map(|item| item.item.as_function_ref())
            .filter(|item| item.kind.is_ffi_function())
            .map(|item| item.path.last())
            .collect::<HashSet<&str>>();

        let ffi_items = self
            .0
            .ffi_items()
            .filter(|item| {
                !item.item.is_function()
                    || used_ffi_functions.contains(item.item.path().last().name.as_str())
            })
            .collect_vec();

        let mut needs_moc = false;
        for ffi_item in &ffi_items {
            match &ffi_item.item {
                CppFfiItem::QtSlotWrapper(qt_slot_wrapper) => {
                    let checks = self.0.cpp_checks(&ffi_item.id)?;
                    if !checks.any_success() {
                        continue;
                    }
                    needs_moc = true;
                    let condition = checks.condition(self.0.environments());
                    let code = self.qt_slot_wrapper(qt_slot_wrapper)?;
                    write!(cpp_file, "{}", self.wrap_with_condition(&code, &condition))?;
                }
                CppFfiItem::QtSignalWrapper(qt_signal_wrapper) => {
                    let checks = self.0.cpp_checks(&ffi_item.id)?;
                    if !checks.any_success() {
                        continue;
                    }
                    needs_moc = true;
                    let condition = checks.condition(self.0.environments());
                    let code = self.qt_signal_wrapper(qt_signal_wrapper)?;
                    write!(cpp_file, "{}", self.wrap_with_condition(&code, &condition))?;
                }
                _ => {}
            }
        }

        writeln!(cpp_file, "extern \"C\" {{")?;
        for ffi_item in &ffi_items {
            if let Some(item) = ffi_item.clone().filter_map(|item| item.as_function_ref()) {
                let checks = self.0.cpp_checks(&ffi_item.id)?;
                if !checks.any_success() {
                    continue;
                }
                let condition = checks.condition(self.0.environments());
                let code = self.function_implementation(item)?;
                writeln!(cpp_file, "{}", self.wrap_with_condition(&code, &condition))?;
            }
        }
        writeln!(cpp_file, "}} // extern \"C\"")?;

        if needs_moc && !self.0.crate_name().starts_with("moqt_") {
            let stem = file_path
                .file_stem()
                .ok_or_else(|| err_msg("failed to get file stem"))?;
            writeln!(cpp_file, "#include \"{}.moc\"", os_str_to_str(stem)?)?;
        }
        Ok(())
    }

    /// Generates a C++ program that determines sizes of target C++ types
    /// on the current platform and outputs the Rust code for `sized_types.rs` module
    /// to the standard output.
    fn generate_cpp_type_size_requester(
        &self,
        include_directives: &[PathBuf],
        mut output: impl Write,
    ) -> Result<()> {
        for dir in include_directives {
            writeln!(output, "#include <{}>", path_to_str(dir)?)?;
        }
        writeln!(output, "#include <stdio.h>\n\nint main() {{")?;

        let rust_items = self.0.rust_items().map(|i| i.item);
        for item in rust_items {
            if let RustItem::Struct(data) = item {
                if let RustStructKind::SizedType(sized_type) = &data.kind {
                    let cpp_path_code = sized_type.cpp_path.to_cpp_code()?;

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
}

pub fn function_implementation(
    db: &DatabaseClient,
    method: DbItem<&CppFfiFunction>,
) -> Result<String> {
    Generator(db).function_implementation(method)
}

pub fn qt_slot_wrapper(db: &DatabaseClient, wrapper: &QtSlotWrapper) -> Result<String> {
    Generator(db).qt_slot_wrapper(wrapper)
}

pub fn qt_signal_wrapper(db: &DatabaseClient, wrapper: &QtSignalWrapper) -> Result<String> {
    Generator(db).qt_signal_wrapper(wrapper)
}

pub fn generate_cpp_file(
    db: &DatabaseClient,
    file_path: &Path,
    global_header_name: &str,
) -> Result<()> {
    Generator(db).generate_cpp_file(file_path, global_header_name)
}

pub fn generate_cpp_type_size_requester(
    db: &DatabaseClient,
    include_directives: &[PathBuf],
    output: impl Write,
) -> Result<()> {
    Generator(db).generate_cpp_type_size_requester(include_directives, output)
}

pub fn all_include_directives(config: &Config) -> Result<Vec<PathBuf>> {
    let mut all_include_directives = config.include_directives().to_vec();

    if let Some(crate_template_path) = config.crate_template_path() {
        let extra_template = crate_template_path.join("c_lib/extra");
        if extra_template.exists() {
            for item in read_dir(&extra_template)? {
                all_include_directives.push(PathBuf::from(format!(
                    "extra/{}",
                    os_str_to_str(&item?.file_name())?
                )));
            }
        }
    }

    Ok(all_include_directives)
}

pub fn write_include_directives(mut destination: impl Write, directives: &[PathBuf]) -> Result<()> {
    for directive in directives {
        writeln!(
            &mut destination,
            "#include \"{}\"",
            path_to_str(&directive)?
        )?;
    }
    Ok(())
}
