//! Types and functions used for Rust code generation.

use crate::cpp_checks::Condition;
use crate::database::Database;
use crate::doc_formatter;
use crate::rust_generator::qt_core_path;
use crate::rust_info::{
    RustDatabase, RustDatabaseItem, RustEnumValue, RustExtraImpl, RustExtraImplKind,
    RustFFIFunction, RustFfiWrapperData, RustFunction, RustFunctionArgument, RustFunctionKind,
    RustItem, RustModule, RustModuleKind, RustStruct, RustStructKind, RustTraitImpl,
    RustWrapperTypeKind,
};
use crate::rust_type::{
    RustCommonType, RustFinalType, RustPath, RustPointerLikeTypeKind, RustToFfiTypeConversion,
    RustType,
};
use itertools::Itertools;
use ritual_common::errors::{bail, err_msg, Result};
use ritual_common::file_utils::{create_dir_all, create_file, file_to_string, File};
use ritual_common::string_utils::trim_slice;
use ritual_common::utils::MapIfOk;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

fn wrap_unsafe(in_unsafe_context: bool, content: &str) -> String {
    let (unsafe_start, unsafe_end) = if in_unsafe_context {
        ("", "")
    } else {
        ("unsafe { ", " }")
    };
    format!("{}{}{}", unsafe_start, content, unsafe_end)
}

/// Generates Rust code representing type `rust_type` inside crate `crate_name`.
/// Same as `RustCodeGenerator::rust_type_to_code`, but accessible by other modules.
pub fn rust_type_to_code(rust_type: &RustType, current_crate: Option<&str>) -> String {
    match rust_type {
        RustType::Tuple(types) => {
            let types_text = types
                .iter()
                .map(|t| rust_type_to_code(t, current_crate) + ",")
                .join("");
            format!("({})", types_text)
        }
        RustType::PointerLike {
            kind,
            target,
            is_const,
        } => {
            let target_code = rust_type_to_code(&*target, current_crate);
            match kind {
                RustPointerLikeTypeKind::Pointer => {
                    if *is_const {
                        format!("*const {}", target_code)
                    } else {
                        format!("*mut {}", target_code)
                    }
                }
                RustPointerLikeTypeKind::Reference { lifetime } => {
                    let lifetime_text = match lifetime {
                        Some(lifetime) => format!("'{} ", lifetime),
                        None => String::new(),
                    };
                    if *is_const {
                        format!("&{}{}", lifetime_text, target_code)
                    } else {
                        format!("&{}mut {}", lifetime_text, target_code)
                    }
                }
            }
        }
        RustType::Common(RustCommonType {
            path,
            generic_arguments,
        }) => {
            let mut code = path.full_name(current_crate);
            if let Some(args) = generic_arguments {
                code = format!(
                    "{}<{}>",
                    code,
                    args.iter()
                        .map(|x| rust_type_to_code(x, current_crate))
                        .join(", ",)
                );
            }
            code
        }
        RustType::FunctionPointer {
            return_type,
            arguments,
        } => format!(
            "extern \"C\" fn({}){}",
            arguments
                .iter()
                .map(|arg| rust_type_to_code(arg, current_crate))
                .join(", "),
            if return_type.is_unit() {
                String::new()
            } else {
                format!(" -> {}", rust_type_to_code(return_type, current_crate))
            }
        ),
        RustType::ImplTrait(trait_type) => format!(
            "impl {}",
            rust_type_to_code(&RustType::Common(trait_type.clone()), current_crate)
        ),
    }
}

struct Generator<'a> {
    crate_name: String,
    output_src_path: PathBuf,
    crate_template_src_path: Option<PathBuf>,
    destination: Vec<File<BufWriter<fs::File>>>,
    current_database: &'a Database,
}

impl Write for Generator<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::Write::write(
            self.destination
                .last_mut()
                .expect("generator: no open files"),
            buf,
        )
    }

    fn flush(&mut self) -> io::Result<()> {
        self.destination
            .last_mut()
            .expect("generator: no open files")
            .flush()
    }
}

/// Generates documentation comments containing
/// markdown code `doc`.
fn format_doc(doc: &str) -> String {
    format_doc_extended(doc, false)
}

#[derive(Debug, Default)]
struct ConditionTexts {
    attribute: String,
    doc_text: String,
}

/// Generates documentation comments containing
/// markdown code `doc`.
fn format_doc_extended(doc: &str, is_outer: bool) -> String {
    if doc.is_empty() {
        return String::new();
    }
    let prefix = if is_outer { "//! " } else { "/// " };
    let lines = doc.split('\n').collect_vec();
    let lines = trim_slice(&lines, |x| x.is_empty());
    if lines.is_empty() {
        return String::new();
    }
    let extra_line_breaks = if is_outer { "\n\n" } else { "\n" };
    lines
        .iter()
        .map(|x| {
            if x.starts_with("    ") {
                format!("{}{}", prefix, x.replace("    ", "&#32;   "))
            } else {
                format!("{}{}", prefix, x)
            }
        })
        .join("\n")
        + extra_line_breaks
}

fn condition_expression(condition: &Condition) -> String {
    match condition {
        Condition::CppLibraryVersion(version) => format!("cpp_lib_version={:?}", version),
        Condition::Arch(_) => unimplemented!(),
        Condition::OS(_) => unimplemented!(),
        Condition::Family(_) => unimplemented!(),
        Condition::Env(_) => unimplemented!(),
        Condition::PointerWidth(_) => unimplemented!(),
        Condition::Endian(_) => unimplemented!(),
        Condition::And(conditions) => {
            let list = conditions.iter().map(condition_expression).join(", ");
            format!("all({})", list)
        }
        Condition::Or(conditions) => {
            let list = conditions.iter().map(condition_expression).join(", ");
            format!("any({})", list)
        }
        Condition::Not(condition) => format!("not({})", condition_expression(condition)),
        Condition::True => "ritual_true".to_string(),
        Condition::False => "ritual_false".to_string(),
    }
}

impl Generator<'_> {
    fn module_path(&self, rust_path: &RustPath, root_path: &Path) -> Result<PathBuf> {
        let parts = &rust_path.parts;

        assert_eq!(
            &parts[0], &self.crate_name,
            "Generator::push_file expects path from this crate"
        );

        let path = if parts.len() == 1 {
            root_path.join("lib.rs")
        } else {
            let mut path = root_path.to_path_buf();
            for middle_part in &parts[1..parts.len() - 1] {
                path.push(middle_part);
            }
            path.push(format!("{}.rs", parts.last().expect("path is empty")));
            path
        };
        Ok(path)
    }

    fn push_file(&mut self, path: &Path) -> Result<()> {
        create_dir_all(path.parent().expect("module file path must have parent"))?;
        self.destination.push(create_file(path)?);
        Ok(())
    }

    fn pop_file(&mut self) {
        self.destination
            .pop()
            .expect("generator: too much pop_file");
    }

    fn generate_item(
        &mut self,
        item: &RustDatabaseItem,
        self_type: Option<&RustType>,
        database: &RustDatabase,
    ) -> Result<()> {
        let ffi_item = if let Some(index) = item.ffi_item_index {
            Some(
                self.current_database
                    .ffi_items()
                    .get(index)
                    .ok_or_else(|| err_msg("rust item refers to invalid ffi item index"))?,
            )
        } else if let Some(index) = item.cpp_item_index {
            let cpp_item = self
                .current_database
                .cpp_items()
                .get(index)
                .ok_or_else(|| err_msg("rust item refers to invalid cpp item index"))?;
            if let Some(index) = cpp_item.source_ffi_item {
                Some(
                    self.current_database
                        .ffi_items()
                        .get(index)
                        .ok_or_else(|| err_msg("cpp item refers to invalid ffi item index"))?,
                )
            } else {
                None
            }
        } else {
            None
        };

        let mut condition_texts = ConditionTexts::default();
        if let Some(ffi_item) = ffi_item {
            let condition = ffi_item
                .checks
                .condition(self.current_database.environments());
            if condition != Condition::True {
                let expression = condition_expression(&condition);
                condition_texts.attribute =
                    format!("#[cfg(any({}, ritual_rustdoc))]\n", expression);
                condition_texts.doc_text =
                    format!("\n\nThis item is available if `{}`.", expression);
            }
        }

        match &item.item {
            RustItem::Module(module) => self.generate_module(module, database),
            RustItem::Struct(data) => self.generate_struct(data, database, &condition_texts),
            RustItem::EnumValue(value) => self.generate_enum_value(value),
            RustItem::TraitImpl(value) => self.generate_trait_impl(value, &condition_texts),
            RustItem::Function(value) => {
                self.generate_rust_final_function(value, false, self_type, &condition_texts)
            }
            RustItem::FfiFunction(value) => self.generate_ffi_function(value, &condition_texts),
            RustItem::ExtraImpl(value) => self.generate_extra_impl(value, &condition_texts),
            RustItem::Reexport { path, target } => {
                writeln!(
                    self,
                    "pub use {} as {};",
                    self.rust_path_to_string(target),
                    path.last()
                )?;
                Ok(())
            }
        }
    }

    fn rust_type_to_code(&self, rust_type: &RustType) -> String {
        rust_type_to_code(rust_type, Some(&self.crate_name))
    }

    #[allow(clippy::collapsible_if)]
    fn generate_module(&mut self, module: &RustModule, database: &RustDatabase) -> Result<()> {
        if database.children(&module.path).next().is_none() {
            // skip empty module
            return Ok(());
        }

        let vis = if module.is_public { "pub " } else { "" };
        let mut content_from_template = None;
        if module.kind.is_in_separate_file() {
            if module.kind != RustModuleKind::CrateRoot {
                writeln!(self, "{}mod {};", vis, module.path.last())?;
            }
            let path = self.module_path(&module.path, &self.output_src_path)?;
            self.push_file(&path)?;

            if let Some(crate_template_src_path) = &self.crate_template_src_path {
                let template_path = self.module_path(&module.path, crate_template_src_path)?;
                if template_path.exists() {
                    let content = file_to_string(template_path)?;
                    content_from_template = Some(content);
                }
            }
        } else {
            assert_ne!(module.kind, RustModuleKind::CrateRoot);
            writeln!(self, "{}mod {} {{", vis, module.path.last())?;
        }

        write!(
            self,
            "{}",
            format_doc_extended(&doc_formatter::module_doc(module), true)
        )?;

        if let Some(content) = content_from_template {
            writeln!(self, "{}", content)?;
        }

        match module.kind {
            RustModuleKind::Ffi => {
                writeln!(self, "include!(concat!(env!(\"OUT_DIR\"), \"/ffi.rs\"));")?;
            }
            RustModuleKind::SizedTypes => {
                writeln!(
                    self,
                    "include!(concat!(env!(\"OUT_DIR\"), \"/sized_types.rs\"));"
                )?;
            }
            RustModuleKind::CrateRoot
            | RustModuleKind::Ops
            | RustModuleKind::CppNamespace
            | RustModuleKind::CppNestedType => {
                self.generate_children(&module.path, None, database)?;
            }
        }

        if module.kind.is_in_separate_file() {
            self.pop_file();
        } else {
            // close `mod {}`
            writeln!(self, "}}")?;
        }

        if module.kind == RustModuleKind::Ffi {
            let path = self.output_src_path.join("ffi.in.rs");
            self.destination.push(create_file(&path)?);
            self.generate_children(&module.path, None, database)?;
            self.pop_file();
        }

        Ok(())
    }

    fn qt_core_path(&self) -> RustPath {
        qt_core_path(&self.crate_name)
    }

    fn qt_core_prefix(&self) -> String {
        let qt_core_path = self.qt_core_path();
        if qt_core_path.parts[0] == self.crate_name {
            "crate".to_string()
        } else {
            format!("::{}", qt_core_path.parts[0])
        }
    }

    fn generate_struct(
        &mut self,
        rust_struct: &RustStruct,
        database: &RustDatabase,
        condition_texts: &ConditionTexts,
    ) -> Result<()> {
        let doc = doc_formatter::struct_doc(rust_struct) + &condition_texts.doc_text;
        write!(self, "{}", format_doc(&doc))?;

        let visibility = if rust_struct.is_public { "pub " } else { "" };
        match &rust_struct.kind {
            RustStructKind::WrapperType(wrapper) => match &wrapper.kind {
                RustWrapperTypeKind::EnumWrapper => {
                    writeln!(
                        self,
                        include_str!("../templates/crate/enum_wrapper.rs.in"),
                        vis = visibility,
                        name = rust_struct.path.last()
                    )?;
                }
                RustWrapperTypeKind::ImmovableClassWrapper => {
                    writeln!(self, "#[repr(C)]")?;
                    writeln!(
                        self,
                        "{}struct {} {{ _unused: u8, }}",
                        visibility,
                        rust_struct.path.last()
                    )?;
                }
                RustWrapperTypeKind::MovableClassWrapper { sized_type_path } => {
                    writeln!(self, "#[repr(transparent)]")?;
                    writeln!(
                        self,
                        "{}struct {}({});",
                        visibility,
                        rust_struct.path.last(),
                        self.rust_path_to_string(sized_type_path),
                    )?;
                    writeln!(self)?;
                }
            },
            RustStructKind::QtSlotWrapper(slot_wrapper) => {
                let arg_texts = slot_wrapper
                    .arguments
                    .iter()
                    .map(|t| self.rust_type_to_code(t.api_type()))
                    .collect_vec();
                let args = arg_texts.join(", ");

                let callback_args = slot_wrapper
                    .arguments
                    .iter()
                    .enumerate()
                    .map(|(num, t)| format!("arg{}: {}", num, self.rust_type_to_code(t.ffi_type())))
                    .join(", ");
                let func_args = slot_wrapper
                    .arguments
                    .iter()
                    .enumerate()
                    .map_if_ok(|(num, t)| {
                        self.convert_type_from_ffi(t, format!("arg{}", num), false, false)
                    })?
                    .join(", ");
                writeln!(
                    self,
                    include_str!("../templates/crate/closure_slot_wrapper.rs.in"),
                    qt_core = self.qt_core_prefix(),
                    type_name = self.rust_path_to_string(&slot_wrapper.raw_slot_wrapper),
                    pub_type_name = rust_struct.path.last(),
                    args = args,
                    func_args = func_args,
                    callback_args = callback_args,
                    condition_attribute = condition_texts.attribute,
                )?;
            }
            RustStructKind::SizedType(_) => {
                bail!("sized struct can't be generated with rust code generator")
            }
        }

        if database.children(&rust_struct.path).next().is_some() {
            let struct_type = RustType::Common(RustCommonType {
                path: rust_struct.path.clone(),
                generic_arguments: None,
            });

            writeln!(self, "impl {} {{", rust_struct.path.last())?;
            self.generate_children(&rust_struct.path, Some(&struct_type), database)?;
            writeln!(self, "}}")?;
            writeln!(self)?;
        }

        Ok(())
    }

    fn generate_enum_value(&mut self, value: &RustEnumValue) -> Result<()> {
        write!(
            self,
            "{}",
            format_doc(&doc_formatter::enum_value_doc(value))
        )?;
        let struct_path =
            self.rust_path_to_string(&value.path.parent().expect("enum value must have parent"));
        writeln!(self, "#[allow(non_upper_case_globals)]")?;
        writeln!(
            self,
            "pub const {value_name}: {struct_path} = {struct_path}({value});",
            value_name = value.path.last(),
            struct_path = struct_path,
            value = value.value
        )?;
        Ok(())
    }

    // TODO: generate relative paths for better readability
    fn rust_path_to_string(&self, path: &RustPath) -> String {
        path.full_name(Some(&self.crate_name))
    }

    /// Generates Rust code containing declaration of a FFI function `func`.
    fn rust_ffi_function_to_code(&self, func: &RustFFIFunction) -> String {
        let mut args = func.arguments.iter().map(|arg| {
            format!(
                "{}: {}",
                arg.name,
                self.rust_type_to_code(&arg.argument_type)
            )
        });
        format!(
            "  pub fn {}({}){};\n",
            func.path.last(),
            args.join(", "),
            if func.return_type.is_unit() {
                String::new()
            } else {
                format!(" -> {}", self.rust_type_to_code(&func.return_type))
            }
        )
    }

    /// Wraps `expression` of type `type1.rust_ffi_type` to convert
    /// it to type `type1.rust_api_type`.
    /// If `in_unsafe_context` is `true`, the output code will be placed inside
    /// an `unsafe` block.
    /// If `use_ffi_result_var` is `true`, the output code will assign
    /// the value to a temporary variable `ffi_result` and return it.
    fn convert_type_from_ffi(
        &self,
        type1: &RustFinalType,
        expression: String,
        in_unsafe_context: bool,
        use_ffi_result_var: bool,
    ) -> Result<String> {
        if type1.conversion() == &RustToFfiTypeConversion::None {
            return Ok(expression);
        }

        let (code1, source_expr) = if use_ffi_result_var {
            (
                format!("let ffi_result = {};\n", expression),
                "ffi_result".to_string(),
            )
        } else {
            (String::new(), expression)
        };
        let code2 = match type1.conversion() {
            RustToFfiTypeConversion::None => unreachable!(),
            RustToFfiTypeConversion::RefToPtr { .. } => {
                let api_is_const = type1.api_type().is_const_pointer_like()?;
                let code = format!(
                    "{}.{}()",
                    source_expr,
                    if api_is_const { "as_ref" } else { "as_mut" },
                );
                let code = wrap_unsafe(in_unsafe_context, &code);
                format!(
                    "{}.expect(\"Attempted to convert null pointer to reference\")",
                    code
                )
            }
            RustToFfiTypeConversion::ValueToPtr => {
                let code = format!("*{}", source_expr);
                wrap_unsafe(in_unsafe_context, &code)
            }
            RustToFfiTypeConversion::CppBoxToPtr => {
                let code = format!(
                    "::cpp_utils::CppBox::from_raw({}).expect(\"attempted to \
                     construct a null CppBox\")",
                    source_expr
                );
                wrap_unsafe(in_unsafe_context, &code)
            }
            RustToFfiTypeConversion::UtilsPtrToPtr { .. }
            | RustToFfiTypeConversion::UtilsRefToPtr { .. }
            | RustToFfiTypeConversion::OptionUtilsRefToPtr { .. } => {
                let is_option = type1.conversion().is_option_utils_ref_to_ptr();

                let ptr_wrapper_type = if is_option {
                    type1
                        .api_type()
                        .as_common()?
                        .generic_arguments
                        .as_ref()
                        .ok_or_else(|| err_msg("expected generic argument for Option"))?
                        .get(0)
                        .ok_or_else(|| err_msg("expected generic argument for Option"))?
                } else {
                    type1.api_type()
                };
                let ptr_wrapper_path = &ptr_wrapper_type.as_common()?.path;

                let need_unwrap = type1.conversion().is_utils_ref_to_ptr();
                let code = format!(
                    "{}::from_raw({}){}",
                    self.rust_path_to_string(ptr_wrapper_path),
                    source_expr,
                    if need_unwrap {
                        ".expect(\"attempted to construct a null Ref\")"
                    } else {
                        ""
                    },
                );
                wrap_unsafe(in_unsafe_context, &code)
            }
            RustToFfiTypeConversion::QFlagsToUInt { .. } => {
                let mut qflags_type = type1.api_type().clone();
                if let RustType::Common(RustCommonType {
                    generic_arguments, ..
                }) = &mut qflags_type
                {
                    *generic_arguments = None;
                } else {
                    unreachable!();
                }
                format!(
                    "{}::from({})",
                    self.rust_type_to_code(&qflags_type),
                    source_expr
                )
            }
            RustToFfiTypeConversion::UnitToAnything => format!("let _ = {};", source_expr),
            RustToFfiTypeConversion::RefTo(conversion) => {
                let intermediate =
                    RustFinalType::new(type1.ffi_type().clone(), (**conversion).clone())?;
                let expr = self.convert_type_from_ffi(
                    &intermediate,
                    source_expr,
                    in_unsafe_context,
                    false,
                )?;
                format!("&{}", expr)
            }
            RustToFfiTypeConversion::ImplCastInto(_) => {
                bail!("ImplCastInto is not convertable from FFI type");
            }
        };
        Ok(code1 + &code2)
    }

    fn convert_type_to_ffi(&self, expr: &str, type1: &RustFinalType) -> Result<String> {
        let code = match type1.conversion() {
            RustToFfiTypeConversion::None => expr.to_string(),
            RustToFfiTypeConversion::RefToPtr { .. } => {
                if type1.api_type().is_const_pointer_like()?
                    && !type1.ffi_type().is_const_pointer_like()?
                {
                    let mut intermediate_type = type1.ffi_type().clone();
                    intermediate_type.set_const(true)?;
                    format!(
                        "{} as {} as {}",
                        expr,
                        self.rust_type_to_code(&intermediate_type),
                        self.rust_type_to_code(type1.ffi_type())
                    )
                } else {
                    format!("{} as {}", expr, self.rust_type_to_code(type1.ffi_type()))
                }
            }
            RustToFfiTypeConversion::ValueToPtr => {
                let is_const = type1.ffi_type().is_const_pointer_like()?;
                format!(
                    "{}{} as {}",
                    if is_const { "&" } else { "&mut " },
                    expr,
                    self.rust_type_to_code(type1.ffi_type())
                )
            }
            RustToFfiTypeConversion::CppBoxToPtr => format!("{}.into_raw_ptr()", expr),
            RustToFfiTypeConversion::UtilsPtrToPtr { .. }
            | RustToFfiTypeConversion::UtilsRefToPtr { .. } => {
                let api_type_path = &type1.api_type().as_common()?.path;
                let api_is_const = api_type_path == &RustPath::from_good_str("cpp_utils::Ptr")
                    || api_type_path == &RustPath::from_good_str("cpp_utils::Ref");
                let ffi_is_const = type1.ffi_type().is_const_pointer_like()?;
                let call = if !api_is_const && !ffi_is_const {
                    format!("{}.as_mut_raw_ptr()", expr)
                } else {
                    format!("{}.as_raw_ptr()", expr)
                };

                if api_is_const != ffi_is_const {
                    format!("{} as {}", call, self.rust_type_to_code(type1.ffi_type()))
                } else {
                    call
                }
            }
            RustToFfiTypeConversion::OptionUtilsRefToPtr { .. } => {
                bail!("OptionUtilsRefToPtr is not supported in argument position");
            }
            RustToFfiTypeConversion::QFlagsToUInt { .. } => format!("{}.to_int()", expr),
            RustToFfiTypeConversion::UnitToAnything => {
                bail!("UnitToAnything is not possible to use in argument position");
            }
            RustToFfiTypeConversion::RefTo(conversion) => {
                let intermediate =
                    RustFinalType::new(type1.ffi_type().clone(), (**conversion).clone())?;
                let code = self.convert_type_to_ffi(expr, &intermediate)?;
                if **conversion == RustToFfiTypeConversion::None {
                    format!("*{}", code)
                } else {
                    code
                }
            }
            RustToFfiTypeConversion::ImplCastInto(conversion) => {
                let intermediate =
                    RustFinalType::new(type1.ffi_type().clone(), (**conversion).clone())?;

                let intermediate_expr = format!(
                    "::cpp_utils::CastInto::<{}>::cast_into({})",
                    self.rust_type_to_code(&intermediate.api_type()),
                    expr
                );
                self.convert_type_to_ffi(&intermediate_expr, &intermediate)?
            }
        };
        Ok(code)
    }

    /// Generates Rust code for calling an FFI function from a wrapper function.
    /// If `in_unsafe_context` is `true`, the output code will be placed inside
    /// an `unsafe` block.
    fn generate_ffi_call(
        &self,
        arguments: &[RustFunctionArgument],
        return_type: &RustFinalType,
        wrapper_data: &RustFfiWrapperData,
        in_unsafe_context: bool,
    ) -> Result<String> {
        let mut final_args = Vec::new();
        final_args.resize(wrapper_data.cpp_ffi_function.arguments.len(), None);
        for arg in arguments {
            assert!(arg.ffi_index < final_args.len());
            let code = self.convert_type_to_ffi(&arg.name, &arg.argument_type)?;
            final_args[arg.ffi_index] = Some(code);
        }

        let mut result = Vec::new();
        let mut maybe_result_var_name = None;
        if let Some(i) = &wrapper_data.return_type_ffi_index {
            let mut return_var_name = "object".to_string();
            let mut ii = 1;
            while arguments.iter().any(|x| x.name == return_var_name) {
                ii += 1;
                return_var_name = format!("object{}", ii);
            }
            let struct_name = if return_type.conversion() == &RustToFfiTypeConversion::CppBoxToPtr {
                if let RustType::Common(RustCommonType {
                    generic_arguments, ..
                }) = return_type.api_type()
                {
                    let generic_arguments = generic_arguments
                        .as_ref()
                        .ok_or_else(|| err_msg("CppBox must have generic_arguments"))?;
                    let arg = generic_arguments
                        .get(0)
                        .ok_or_else(|| err_msg("CppBox must have non-empty generic_arguments"))?;
                    self.rust_type_to_code(arg)
                } else {
                    bail!("CppBox type expected");
                }
            } else {
                self.rust_type_to_code(return_type.api_type())
            };
            // TODO: use MybeUninit when it's stable
            let expr = wrap_unsafe(in_unsafe_context, "::std::mem::uninitialized()");
            result.push(format!(
                "{{\nlet mut {var}: {t} = {e};\n",
                var = return_var_name,
                t = struct_name,
                e = expr
            ));
            final_args[*i as usize] = Some(format!("&mut {}", return_var_name));
            maybe_result_var_name = Some(return_var_name);
        }
        let final_args = final_args
            .into_iter()
            .map_if_ok(|x| x.ok_or_else(|| err_msg("ffi argument is missing")))?;

        result.push(wrap_unsafe(
            in_unsafe_context,
            &format!(
                "{}({}){maybe_semicolon}",
                self.rust_path_to_string(&wrapper_data.ffi_function_path),
                final_args.join(", "),
                maybe_semicolon = if maybe_result_var_name.is_some() {
                    ";"
                } else {
                    ""
                },
            ),
        ));
        if let Some(name) = &maybe_result_var_name {
            result.push(format!("{}\n}}", name));
        }
        let code = result.join("");
        if maybe_result_var_name.is_none() {
            self.convert_type_from_ffi(&return_type, code, in_unsafe_context, true)
        } else {
            Ok(code)
        }
    }

    /// Generates Rust code for declaring a function's arguments.
    fn arg_texts(
        &self,
        args: &[RustFunctionArgument],
        lifetime: Option<&String>,
        self_type: Option<&RustType>,
    ) -> Result<Vec<String>> {
        args.iter().map_if_ok(|arg| {
            if &arg.name == "self" {
                let self_arg_type = match lifetime {
                    Some(lifetime) => arg.argument_type.api_type().with_lifetime(lifetime.clone()),
                    None => arg.argument_type.api_type().clone(),
                };
                let self_type = self_type.ok_or_else(|| {
                    err_msg("self argument is present but no self_type is specified")
                })?;
                if self_type == &self_arg_type {
                    return Ok("self".to_string());
                } else if let Ok(self_arg_target) = self_arg_type.pointer_like_to_target() {
                    if &self_arg_target == self_type {
                        if let RustType::PointerLike { kind, is_const, .. } = &self_arg_type {
                            if let RustPointerLikeTypeKind::Reference { lifetime } = kind {
                                let maybe_mut = if *is_const { "" } else { "mut " };
                                let text = match lifetime {
                                    Some(lifetime) => format!("&'{} {}self", lifetime, maybe_mut),
                                    None => format!("&{}self", maybe_mut),
                                };
                                return Ok(text);
                            }
                        }
                    }
                }
            }
            let mut maybe_mut_declaration = "";
            if let RustType::Common { .. } = arg.argument_type.api_type() {
                if arg.argument_type.conversion() == &RustToFfiTypeConversion::ValueToPtr {
                    if let RustType::PointerLike { is_const, .. } = &arg.argument_type.ffi_type() {
                        if !*is_const {
                            maybe_mut_declaration = "mut ";
                        }
                    }
                }
            }

            Ok(format!(
                "{}{}: {}",
                maybe_mut_declaration,
                arg.name,
                match lifetime {
                    Some(lifetime) => self.rust_type_to_code(
                        &arg.argument_type.api_type().with_lifetime(lifetime.clone())
                    ),
                    None => self.rust_type_to_code(arg.argument_type.api_type()),
                }
            ))
        })
    }

    /// Generates complete code of a Rust wrapper function.
    fn generate_rust_final_function(
        &mut self,
        func: &RustFunction,
        is_in_trait_context: bool,
        self_type: Option<&RustType>,
        condition_texts: &ConditionTexts,
    ) -> Result<()> {
        let maybe_pub = if func.is_public && !is_in_trait_context {
            "pub "
        } else {
            ""
        };
        let maybe_unsafe = if func.is_unsafe { "unsafe " } else { "" };

        let body = match &func.kind {
            RustFunctionKind::FfiWrapper(data) => {
                self.generate_ffi_call(&func.arguments, &func.return_type, data, func.is_unsafe)?
            }
            RustFunctionKind::CppDeletableImpl { deleter } => self.rust_path_to_string(deleter),
            RustFunctionKind::SignalOrSlotGetter { receiver_id, .. } => {
                let path = &func.return_type.api_type().as_common()?.path;
                let call = format!(
                    "{}::new(::cpp_utils::Ref::from_raw(self as &{})\
                     .expect(\"attempted to construct a null Ref\"), \
                     ::std::ffi::CStr::from_bytes_with_nul_unchecked(b\"{}\\0\"))",
                    self.rust_path_to_string(&path),
                    self.rust_path_to_string(&self.qt_core_path().join("QObject")),
                    receiver_id
                );
                wrap_unsafe(func.is_unsafe, &call)
            }
        };

        let return_type_for_signature = if func.return_type.api_type().is_unit() {
            String::new()
        } else {
            format!(
                " -> {}",
                self.rust_type_to_code(func.return_type.api_type())
            )
        };
        let all_lifetimes = func
            .arguments
            .iter()
            .filter_map(|x| x.argument_type.api_type().lifetime())
            .collect_vec();
        let lifetimes_text = if all_lifetimes.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                all_lifetimes.iter().map(|x| format!("'{}", x)).join(", ")
            )
        };

        let doc = doc_formatter::function_doc(&func) + &condition_texts.doc_text;
        writeln!(
            self,
            "{doc}{condition}{maybe_pub}{maybe_unsafe}fn {name}{lifetimes_text}({args}){return_type} \
             {{\n{body}}}\n\n",
            doc = format_doc(&doc),
            condition = condition_texts.attribute,
            maybe_pub = maybe_pub,
            maybe_unsafe = maybe_unsafe,
            lifetimes_text = lifetimes_text,
            name = func.path.last(),
            args = self.arg_texts(&func.arguments, None, self_type)?.join(", "),
            return_type = return_type_for_signature,
            body = body
        )?;
        Ok(())
    }

    fn generate_children(
        &mut self,
        parent: &RustPath,
        self_type: Option<&RustType>,
        database: &RustDatabase,
    ) -> Result<()> {
        if database
            .children(&parent)
            .any(|item| item.item.is_ffi_function())
        {
            writeln!(self, "extern \"C\" {{\n")?;
            for item in database
                .children(&parent)
                .filter(|item| item.item.is_ffi_function())
            {
                self.generate_item(item, self_type, database)?;
            }
            writeln!(self, "}}\n")?;
        }

        for item in database
            .children(&parent)
            .filter(|item| !item.item.is_ffi_function())
        {
            self.generate_item(item, self_type, database)?;
        }
        Ok(())
    }

    fn generate_trait_impl(
        &mut self,
        trait1: &RustTraitImpl,
        condition_texts: &ConditionTexts,
    ) -> Result<()> {
        let associated_types_text = trait1
            .associated_types
            .iter()
            .map(|t| format!("type {} = {};", t.name, self.rust_type_to_code(&t.value)))
            .join("\n");

        // TODO: use condition_texts.doc_text
        writeln!(
            self,
            "{}impl {} for {} {{\n{}",
            condition_texts.attribute,
            self.rust_type_to_code(&trait1.trait_type),
            self.rust_type_to_code(&trait1.target_type),
            associated_types_text,
        )?;

        for func in &trait1.functions {
            self.generate_rust_final_function(
                func,
                true,
                Some(&trait1.target_type),
                &ConditionTexts::default(),
            )?;
        }

        writeln!(self, "}}\n")?;
        Ok(())
    }

    fn generate_ffi_function(
        &mut self,
        function: &RustFFIFunction,
        condition_texts: &ConditionTexts,
    ) -> Result<()> {
        writeln!(self, "{}", condition_texts.attribute)?;
        writeln!(self, "{}", self.rust_ffi_function_to_code(function))?;
        Ok(())
    }

    fn generate_extra_impl(
        &mut self,
        data: &RustExtraImpl,
        condition_texts: &ConditionTexts,
    ) -> Result<()> {
        match &data.kind {
            RustExtraImplKind::FlagEnum { enum_path } => {
                let enum_path = self.rust_path_to_string(enum_path);
                let qflags = self.rust_path_to_string(&self.qt_core_path().join("QFlags"));

                writeln!(
                    self,
                    include_str!("../templates/crate/flag_enum_impl.rs.in"),
                    e = enum_path,
                    qflags = qflags
                )?;
            }
            RustExtraImplKind::RawSlotReceiver(data) => {
                writeln!(
                    self,
                    include_str!("../templates/crate/impl_receiver_for_raw_slot.rs.in"),
                    qt_core = self.qt_core_prefix(),
                    type_path = self.rust_path_to_string(&data.target_path),
                    args = self.rust_type_to_code(&data.arguments),
                    receiver_id = data.receiver_id,
                    condition_attribute = condition_texts.attribute,
                )?;
                // TODO: use condition_texts.doc_text
            }
        }
        Ok(())
    }
}

pub fn generate(
    crate_name: &str,
    current_database: &Database,
    database: &RustDatabase,
    output_src_path: impl Into<PathBuf>,
    crate_template_src_path: Option<impl Into<PathBuf>>,
) -> Result<()> {
    let mut generator = Generator {
        crate_name: crate_name.to_string(),
        destination: Vec::new(),
        output_src_path: output_src_path.into(),
        crate_template_src_path: crate_template_src_path.map(Into::into),
        current_database,
    };

    let crate_root = database
        .items()
        .iter()
        .filter_map(RustDatabaseItem::as_module_ref)
        .find(|module| module.kind == RustModuleKind::CrateRoot)
        .ok_or_else(|| err_msg("crate root not found"))?;

    generator.generate_module(crate_root, database)?;
    Ok(())
}
