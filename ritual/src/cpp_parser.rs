use crate::config::Config;
use crate::cpp_code_generator::{all_include_directives, write_include_directives};
use crate::cpp_data::{
    CppBaseSpecifier, CppClassField, CppEnumValue, CppItem, CppNamespace, CppOriginLocation,
    CppPath, CppPathItem, CppTypeDeclaration, CppTypeDeclarationKind, CppVisibility,
};
use crate::cpp_function::{
    CppFunction, CppFunctionArgument, CppFunctionKind, CppFunctionMemberData,
};
use crate::cpp_operator::CppOperator;
use crate::cpp_type::{
    CppBuiltInNumericType, CppFunctionPointerType, CppPointerLikeTypeKind, CppSpecificNumericType,
    CppSpecificNumericTypeKind, CppTemplateParameter, CppType,
};
use crate::database2;
use crate::workspace::Workspace;
use clang::diagnostic::{Diagnostic, Severity};
use clang::*;
use itertools::Itertools;
use log::{debug, trace, warn};
use regex::Regex;
use ritual_common::env_var_names;
use ritual_common::errors::{bail, err_msg, format_err, print_trace, Result, ResultExt};
use ritual_common::file_utils::{
    canonicalize, copy_recursively, create_file, open_file, os_str_to_str, path_to_str,
    remove_dir_all, remove_file,
};
use ritual_common::target::{current_env, current_target, Env, LibraryTarget};
use ritual_common::utils::MapIfOk;
use std::io::Write;

use std::iter::once;
use std::path::{Path, PathBuf};
use std::str::FromStr;

fn convert_type_kind(kind: TypeKind) -> CppBuiltInNumericType {
    match kind {
        TypeKind::Bool => CppBuiltInNumericType::Bool,
        TypeKind::CharS | TypeKind::CharU => CppBuiltInNumericType::Char,
        TypeKind::SChar => CppBuiltInNumericType::SChar,
        TypeKind::UChar => CppBuiltInNumericType::UChar,
        TypeKind::WChar => CppBuiltInNumericType::WChar,
        TypeKind::Char16 => CppBuiltInNumericType::Char16,
        TypeKind::Char32 => CppBuiltInNumericType::Char32,
        TypeKind::Short => CppBuiltInNumericType::Short,
        TypeKind::UShort => CppBuiltInNumericType::UShort,
        TypeKind::Int => CppBuiltInNumericType::Int,
        TypeKind::UInt => CppBuiltInNumericType::UInt,
        TypeKind::Long => CppBuiltInNumericType::Long,
        TypeKind::ULong => CppBuiltInNumericType::ULong,
        TypeKind::LongLong => CppBuiltInNumericType::LongLong,
        TypeKind::ULongLong => CppBuiltInNumericType::ULongLong,
        TypeKind::Int128 => CppBuiltInNumericType::Int128,
        TypeKind::UInt128 => CppBuiltInNumericType::UInt128,
        TypeKind::Float => CppBuiltInNumericType::Float,
        TypeKind::Double => CppBuiltInNumericType::Double,
        TypeKind::LongDouble => CppBuiltInNumericType::LongDouble,
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub struct CppParserOutputItem {
    pub index: usize,
    /// File name of the include file (without full path)
    pub include_file: String,
    /// Exact location of the declaration
    pub origin_location: CppOriginLocation,
}

#[derive(Debug, Default)]
pub struct CppParserOutput(pub Vec<CppParserOutputItem>);

pub struct CppParserContext<'a> {
    pub current_database: &'a mut database2::Database,
    pub dependencies: &'a [&'a database2::Database],
    pub config: &'a Config,
    pub workspace: &'a Workspace,
}

impl CppParserContext<'_> {
    fn all_databases(&self) -> impl Iterator<Item = &database2::Database> {
        once(self.current_database as &_).chain(self.dependencies.iter().copied())
    }

    pub fn all_cpp_items(&self) -> impl Iterator<Item = &CppItem> {
        self.all_databases()
            .flat_map(|d| d.items().iter().map(|i| &i.item))
    }

    fn reborrow(&mut self) -> CppParserContext<'_> {
        CppParserContext {
            current_database: self.current_database,
            dependencies: self.dependencies,
            config: self.config,
            workspace: self.workspace,
        }
    }
}

/// Implementation of the C++ parser that extracts information
/// about the C++ library's API from its headers.
struct CppParser<'a> {
    data: CppParserContext<'a>,
    current_target_paths: Vec<PathBuf>,
    target_index: usize,
    output: CppParserOutput,
}

/// Print representation of `entity` and its children to the log.
/// `level` is current level of recursion.
fn dump_entity(entity: Entity<'_>, level: usize) {
    trace!(
        "[DebugParser] {}{:?}",
        (0..level).map(|_| ". ").join(""),
        entity
    );
    if level <= 5 {
        for child in entity.get_children() {
            dump_entity(child, level + 1);
        }
    }
}

/// Extract `clang`'s location information for `entity` to `CppOriginLocation`.
fn get_origin_location(entity: Entity<'_>) -> Result<CppOriginLocation> {
    match entity.get_location() {
        Some(loc) => {
            let location = loc.get_presumed_location();
            Ok(CppOriginLocation {
                include_file_path: location.0,
                line: location.1,
                column: location.2,
            })
        }
        None => bail!("No info about location."),
    }
}

/// Extract template argument declarations from a class or method definition `entity`.
fn get_template_arguments(entity: Entity<'_>) -> Option<Vec<CppType>> {
    let mut nested_level = 0;
    let mut parent = entity;
    while let Some(parent1) = parent.get_semantic_parent() {
        parent = parent1;
        if let Some(args) = get_template_arguments(parent) {
            let parent_nested_level = if let CppType::TemplateParameter(param) = &args[0] {
                param.nested_level
            } else {
                panic!("this value should always be a template parameter")
            };

            nested_level = parent_nested_level + 1;
            break;
        }
    }
    let args = entity
        .get_children()
        .into_iter()
        .filter(|c| c.get_kind() == EntityKind::TemplateTypeParameter)
        .enumerate()
        .map(|(i, c)| {
            CppType::TemplateParameter(CppTemplateParameter {
                name: c.get_name().unwrap_or_else(|| format!("Type{}", i + 1)),
                index: i,
                nested_level,
            })
        })
        .collect_vec();
    if args.is_empty() {
        None
    } else {
        Some(args)
    }
}

fn get_context_template_args(entity: Entity<'_>) -> Vec<CppType> {
    let mut current_entity = entity;
    let mut args = Vec::new();
    loop {
        args.extend(get_template_arguments(current_entity).into_iter().flatten());
        if let Some(parent) = current_entity.get_semantic_parent() {
            current_entity = parent;
        } else {
            break;
        }
    }
    args
}

fn get_path_item(entity: Entity<'_>) -> Result<CppPathItem> {
    let name = entity.get_name().ok_or_else(|| err_msg("Anonymous type"))?;
    let template_arguments = get_template_arguments(entity);
    Ok(CppPathItem {
        name,
        template_arguments,
    })
}

/// Returns fully qualified name of `entity`.
fn get_path(entity: Entity<'_>) -> Result<CppPath> {
    let mut current_entity = entity;
    let mut parts = vec![get_path_item(entity)?];
    loop {
        let p = current_entity
            .get_semantic_parent()
            .ok_or_else(|| format_err!("failed to get parent for {:?}", current_entity))?;

        match p.get_kind() {
            EntityKind::TranslationUnit => break,
            EntityKind::ClassDecl
            | EntityKind::ClassTemplate
            | EntityKind::StructDecl
            | EntityKind::Namespace
            | EntityKind::EnumDecl
            | EntityKind::ClassTemplatePartialSpecialization => {
                parts.insert(0, get_path_item(p)?);
                current_entity = p;
            }
            EntityKind::Method => {
                bail!("Type nested in a method");
            }
            _ => bail!("get_full_name: unexpected parent kind: {:?}", p),
        }
    }
    if parts.len() > 1 && parts[0].name == "std" && parts[1].name == "__cxx11" {
        // this is an inline namespace (not portable)
        parts.remove(1);
    }
    Ok(CppPath::from_items(parts))
}

fn get_full_name_display(entity: Entity<'_>) -> String {
    match get_path(entity) {
        Ok(name) => name.to_cpp_pseudo_code(),
        Err(_) => "[unnamed]".into(),
    }
}

#[cfg(test)]
fn init_clang() -> Result<Clang> {
    for _ in 0..12000 {
        if let Ok(clang) = Clang::new() {
            return Ok(clang);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Clang::new().map_err(|err| format_err!("clang init failed: {}", err))
}

#[cfg(not(test))]
/// Creates a `Clang` context.
fn init_clang() -> Result<Clang> {
    Clang::new().map_err(|err| format_err!("clang init failed: {}", err))
}

/// Runs `clang` parser with `config`.
/// If `cpp_code` is specified, it's written to the C++ file before parsing it.
/// If successful, calls `f` and passes the topmost entity (the translation unit)
/// as its argument. Returns output value of `f` or an error.
fn run_clang<R, F: FnMut(Entity<'_>) -> Result<R>>(
    config: &Config,
    tmp_path: &Path,
    cpp_code: Option<String>,
    mut f: F,
) -> Result<R> {
    let clang = init_clang()?;
    let index = Index::new(&clang, false, false);

    let global_file_path = tmp_path.join("global.h");
    let mut global_file = create_file(&global_file_path)?;
    writeln!(
        global_file,
        "{}",
        include_str!("../templates/c_lib/global.h"),
    )?;
    write_include_directives(&mut global_file, &all_include_directives(config)?)?;
    drop(global_file);

    let tmp_cpp_path = tmp_path.join("1.cpp");
    let mut tmp_file = create_file(&tmp_cpp_path)?;
    writeln!(tmp_file, "#include \"global.h\"")?;
    if let Some(cpp_code) = cpp_code {
        write!(tmp_file, "{}", cpp_code)?;
    }
    drop(tmp_file);

    if let Some(template_path) = config.crate_template_path() {
        let extra_files_dir = template_path.join("c_lib/extra");
        if extra_files_dir.exists() {
            let destination = tmp_path.join("extra");
            if destination.exists() {
                remove_dir_all(&destination)?;
            }
            copy_recursively(&extra_files_dir, &destination)?;
        }
    }

    let mut args = vec![
        "-Xclang".to_string(),
        "-detailed-preprocessing-record".to_string(),
    ];
    if current_env() != Env::Msvc {
        args.push("-std=c++17".to_string());
    }
    args.extend_from_slice(config.cpp_parser_arguments());
    let mut cpp_build_paths = config.cpp_build_paths().clone();
    cpp_build_paths.apply_env();
    for dir in cpp_build_paths.include_paths() {
        let str = path_to_str(dir)?;
        args.push("-I".to_string());
        args.push(str.to_string());
    }
    if let Ok(path) = ::std::env::var(env_var_names::CLANG_SYSTEM_INCLUDE_PATH) {
        if !Path::new(&path).exists() {
            warn!(
                "{} environment variable is set to \"{}\" \
                 but this path does not exist. This may result in parse errors related to system header includes.",
                env_var_names::CLANG_SYSTEM_INCLUDE_PATH,
                path
            );
        }
        args.push("-isystem".to_string());
        args.push(path);
    } else {
        trace!("{} environment variable is not set. This may result in parse errors related to system header includes.", env_var_names::CLANG_SYSTEM_INCLUDE_PATH);
    }
    for dir in config.cpp_build_paths().framework_paths() {
        let str = path_to_str(dir)?;
        args.push("-F".to_string());
        args.push(str.to_string());
    }
    debug!("clang arguments: {:?}", args);

    let tu = index
        .parser(&tmp_cpp_path)
        .arguments(&args)
        .parse()
        .with_context(|_| "clang parse failed")?;
    let translation_unit = tu.get_entity();
    assert_eq!(translation_unit.get_kind(), EntityKind::TranslationUnit);
    {
        let diagnostics = tu.get_diagnostics();
        if !diagnostics.is_empty() {
            trace!("[DebugParser] Diagnostics:");
            for diag in &diagnostics {
                trace!("[DebugParser] {}", diag);
            }
        }
        let should_print_error = |d: &Diagnostic<'_>| {
            d.get_severity() == Severity::Error || d.get_severity() == Severity::Fatal
        };
        if diagnostics.iter().any(should_print_error) {
            bail!(
                "fatal clang error:\n{}",
                diagnostics.iter().map(ToString::to_string).join("\n")
            );
        }
    }
    let result = f(translation_unit);
    remove_file(&tmp_cpp_path)?;
    remove_file(&global_file_path)?;
    result
}

/// Runs the parser on specified data.
pub fn run(data: CppParserContext<'_>) -> Result<()> {
    debug!("clang version: {}", get_version());
    debug!("Initializing clang");
    let mut parser = CppParser {
        current_target_paths: data
            .config
            .target_include_paths()
            .iter()
            .map_if_ok(canonicalize)?,
        target_index: data.current_database.add_target(LibraryTarget {
            cpp_library_version: data.config.cpp_lib_version().map(ToString::to_string),
            target: current_target(),
        }),
        data,
        output: Default::default(),
    };
    parser
        .current_target_paths
        .push(canonicalize(parser.data.workspace.tmp_path())?.join("extra"));
    run_clang(
        parser.data.config,
        &parser.data.workspace.tmp_path(),
        None,
        |translation_unit| parser.parse(translation_unit),
    )?;

    Ok(())
}

/*
pub fn parse_generated_items(data: CppParserContext<'_>) -> Result<()> {
    let current_target = LibraryTarget {
        cpp_library_version: data.config.cpp_lib_version().map(ToString::to_string),
        target: current_target(),
    };
    for ffi_item_id in data.db.ffi_item_ids().collect_vec() {
        let ffi_item = data.db.ffi_item(&ffi_item_id)?;
        if !ffi_item.item.is_source_item() {
            continue;
        }
        if !data
            .db
            .cpp_checks(&ffi_item_id)?
            .is_success(&current_target)
        {
            continue;
        }
        let code = ffi_item.item.source_item_cpp_code(data.db)?;
        let mut parser = CppParser {
            current_target_paths: vec![canonicalize(data.workspace.tmp_path())?.join("1.cpp")],
            source_id: Some(ffi_item_id),
            data,
            output: Default::default(),
        };
        run_clang(
            parser.data.config,
            &parser.data.workspace.tmp_path(),
            Some(code),
            |translation_unit| {
                parser.parse(translation_unit)?;
                Ok(())
            },
        )?;
    }
    Ok(())
}*/

impl CppParser<'_> {
    fn add_output(
        &mut self,
        include_file: String,
        origin_location: CppOriginLocation,
        item: CppItem,
    ) -> Result<()> {
        if let Some(index) = self.data.current_database.add_item(self.target_index, item) {
            self.output.0.push(CppParserOutputItem {
                index,
                include_file,
                origin_location,
            });
        }
        Ok(())
    }

    /// Search for a C++ type information in the types found by the parser
    /// and in types of the dependencies.
    fn find_type(
        &self,
        mut f: impl FnMut(&CppTypeDeclaration) -> bool,
    ) -> Option<&CppTypeDeclaration> {
        self.data
            .all_cpp_items()
            .filter_map(|item| item.as_type_ref())
            .find(|i| f(i))
    }

    /// Attempts to parse an unexposed type, i.e. a type the used `clang` API
    /// is not able to describe. Either `type1` or `string` must be specified,
    /// and both may be specified at the same time.
    /// Surrounding class and/or
    /// method may be specified in `context_class` and `context_method`.
    #[allow(clippy::cognitive_complexity)]
    fn parse_unexposed_type(
        &self,
        type1: Option<Type<'_>>,
        string: Option<String>,
        context_template_args: &[CppType],
    ) -> Result<CppType> {
        trace!("parse_unexposed_type {:?}, {:?}", type1, string);
        let (is_const, name) = if let Some(type1) = type1 {
            let is_const = type1.is_const_qualified();
            let mut name = type1.get_display_name();
            let is_const_in_name = name.starts_with("const ");
            if is_const != is_const_in_name {
                bail!("const inconsistency: {}, {:?}", is_const, type1);
            }
            if is_const_in_name {
                name = name[6..].to_string();
            }
            if name.starts_with("typename ") {
                name = name[9..].to_string();
            }
            if let Some(declaration) = type1.get_declaration() {
                if declaration.get_kind() == EntityKind::ClassDecl
                    || declaration.get_kind() == EntityKind::ClassTemplate
                    || declaration.get_kind() == EntityKind::StructDecl
                {
                    if declaration
                        .get_accessibility()
                        .unwrap_or(Accessibility::Public)
                        != Accessibility::Public
                    {
                        bail!(
                            "Type uses private class ({})",
                            get_full_name_display(declaration)
                        );
                    }
                    if let Some((_class_name, args)) = parse_template_args(&name) {
                        let mut arg_types = Vec::new();
                        for arg in args {
                            match self.parse_unexposed_type(
                                None,
                                Some(arg.trim().to_string()),
                                context_template_args,
                            ) {
                                Ok(arg_type) => arg_types.push(arg_type),
                                Err(msg) => {
                                    bail!(
                                        "Template argument of unexposed type is not parsed: {}: {}",
                                        arg,
                                        msg
                                    );
                                }
                            }
                        }
                        let mut name = get_path(declaration)?;
                        name.last_mut().template_arguments = Some(arg_types);
                        return Ok(CppType::Class(name));
                    } else {
                        bail!("Can't parse declaration of an unexposed type: {}", name);
                    }
                }
            }
            (is_const, name)
        } else if let Some(mut name) = string {
            let is_const_in_name = name.starts_with("const ");
            if is_const_in_name {
                name = name[6..].to_string();
            }
            (is_const_in_name, name)
        } else {
            bail!("parse_unexposed_type: either type or string must be present");
        };
        let re = Regex::new(r"^type-parameter-(\d+)-(\d+)$")?;
        if let Some(matches) = re.captures(name.as_ref()) {
            if matches.len() < 3 {
                bail!("invalid matches len in regexp");
            }
            return Ok(CppType::TemplateParameter(CppTemplateParameter {
                nested_level: matches[1].parse::<usize>().with_context(|_| {
                    "encountered not a number while parsing type-parameter-X-X"
                })?,
                index: matches[2].parse::<usize>().with_context(|_| {
                    "encountered not a number while parsing type-parameter-X-X"
                })?,
                name: name.clone(),
            }));
        }

        if let Some(arg) = context_template_args
            .iter()
            .find(|t| t.to_cpp_pseudo_code() == name)
        {
            return Ok(arg.clone());
        }

        if name.ends_with(" *") {
            let remaining_name = name[0..name.len() - " *".len()].trim();
            let subtype = self.parse_unexposed_type(
                None,
                Some(remaining_name.to_string()),
                context_template_args,
            )?;
            if let CppType::FunctionPointer(..) = subtype {
                return Ok(subtype);
            } else {
                return Ok(CppType::PointerLike {
                    kind: CppPointerLikeTypeKind::Pointer,
                    is_const,
                    target: Box::new(subtype),
                });
            }
        }

        if name.ends_with(" &") {
            let remaining_name = name[0..name.len() - " &".len()].trim();
            let subtype = self.parse_unexposed_type(
                None,
                Some(remaining_name.to_string()),
                context_template_args,
            )?;
            return Ok(CppType::PointerLike {
                kind: CppPointerLikeTypeKind::Reference,
                is_const,
                target: Box::new(subtype),
            });
        }

        if name == "void" {
            return Ok(CppType::Void);
        }
        if let Some(x) = CppBuiltInNumericType::all()
            .iter()
            .find(|x| x.to_cpp_code() == name)
        {
            return Ok(CppType::BuiltInNumeric(x.clone()));
        }
        if let Some(result) = self.parse_special_typedef(&name) {
            return Ok(result);
        }
        if let Some(type_data) =
            self.find_type(|x| x.path.to_cpp_code().ok().as_ref() == Some(&name))
        {
            let path = CppPath::from_str(&name)?;
            if let Some(hook) = self.data.config.cpp_parser_path_hook() {
                if !hook(&path)? {
                    bail!("blacklisted path: {}", path.to_cpp_pseudo_code());
                }
            }
            match type_data.kind {
                CppTypeDeclarationKind::Enum { .. } => {
                    return Ok(CppType::Enum { path });
                }
                CppTypeDeclarationKind::Class { .. } => {
                    return Ok(CppType::Class(path));
                }
            }
        }

        if let Some((class_text, args)) = parse_template_args(&name) {
            if self
                .find_type(|x| x.kind.is_class() && x.path.to_templateless_string() == class_text)
                .is_some()
            {
                let mut arg_types = Vec::new();
                for arg in args {
                    match self.parse_unexposed_type(
                        None,
                        Some(arg.trim().to_string()),
                        context_template_args,
                    ) {
                        Ok(arg_type) => arg_types.push(arg_type),
                        Err(msg) => {
                            bail!(
                                "Template argument of unexposed type is not parsed: {}: {}",
                                arg,
                                msg
                            );
                        }
                    }
                }
                let mut class_name = CppPath::from_str(&class_text)?;
                class_name.last_mut().template_arguments = Some(arg_types);
                return Ok(CppType::Class(class_name));
            }
        } else {
            bail!("Can't parse declaration of an unexposed type: {}", name);
        }

        bail!("Unrecognized unexposed type: {}", name);
    }

    /// Parses type `type1`.
    /// Surrounding class and/or
    /// method may be specified in `context_class` and `context_method`.
    fn parse_type(&self, type1: Type<'_>, context_template_args: &[CppType]) -> Result<CppType> {
        if type1.is_volatile_qualified() {
            bail!("Volatile type");
        }
        let display_name = type1.get_display_name();
        if display_name == "std::list<T>" {
            bail!(
                "Type blacklisted because it causes crash on Windows: {}",
                display_name
            );
        }
        if display_name == "std::__cxx11::basic_string::const_reference"
            || display_name == "std::vector::const_reference"
        {
            return Ok(CppType::new_reference(
                true,
                CppType::TemplateParameter(CppTemplateParameter {
                    name: "T".into(),
                    nested_level: 0,
                    index: 0,
                }),
            ));
        }
        if display_name == "std::__cxx11::basic_string::reference"
            || display_name == "std::vector::reference"
        {
            return Ok(CppType::new_reference(
                false,
                CppType::TemplateParameter(CppTemplateParameter {
                    name: "T".into(),
                    nested_level: 0,
                    index: 0,
                }),
            ));
        }
        match type1.get_kind() {
            TypeKind::Typedef => {
                let mut name = type1.get_display_name();
                if name.starts_with("const ") {
                    name = name[6..].trim().to_string();
                }
                if let Some(r) = self.parse_special_typedef(&name) {
                    return Ok(r);
                }
                self.parse_type(type1.get_canonical_type(), context_template_args)
            }
            TypeKind::Void => Ok(CppType::Void),
            TypeKind::Bool
            | TypeKind::CharS
            | TypeKind::CharU
            | TypeKind::SChar
            | TypeKind::UChar
            | TypeKind::WChar
            | TypeKind::Char16
            | TypeKind::Char32
            | TypeKind::Short
            | TypeKind::UShort
            | TypeKind::Int
            | TypeKind::UInt
            | TypeKind::Long
            | TypeKind::ULong
            | TypeKind::LongLong
            | TypeKind::ULongLong
            | TypeKind::Int128
            | TypeKind::UInt128
            | TypeKind::Float
            | TypeKind::Double
            | TypeKind::LongDouble => {
                Ok(CppType::BuiltInNumeric(convert_type_kind(type1.get_kind())))
            }
            TypeKind::Enum => {
                if let Some(declaration) = type1.get_declaration() {
                    let path = get_path(declaration)?;
                    if let Some(hook) = self.data.config.cpp_parser_path_hook() {
                        if !hook(&path)? {
                            bail!("blacklisted path: {}", path.to_cpp_pseudo_code());
                        }
                    }
                    Ok(CppType::Enum { path })
                } else {
                    bail!("failed to get enum declaration: {:?}", type1);
                }
            }
            TypeKind::Record => {
                if let Some(declaration) = type1.get_declaration() {
                    if declaration
                        .get_accessibility()
                        .unwrap_or(Accessibility::Public)
                        != Accessibility::Public
                    {
                        bail!(
                            "Type uses private class ({})",
                            get_full_name_display(declaration)
                        );
                    }
                    let mut declaration_name = get_path(declaration)?;
                    if let Some(hook) = self.data.config.cpp_parser_path_hook() {
                        if !hook(&declaration_name)? {
                            bail!(
                                "blacklisted path: {}",
                                declaration_name.to_cpp_pseudo_code()
                            );
                        }
                    }
                    let template_arguments = match type1.get_template_argument_types() {
                        None => None,
                        Some(arg_types) => {
                            let mut r = Vec::new();
                            if arg_types.is_empty() {
                                bail!("arg_types is empty");
                            }
                            for arg_type in arg_types {
                                match arg_type {
                                    None => bail!("Template argument is None"),
                                    Some(arg_type) => {
                                        match self.parse_type(arg_type, context_template_args) {
                                            Ok(parsed_type) => r.push(parsed_type),
                                            Err(msg) => {
                                                bail!(
                                                    "Invalid template argument: {:?}: {}",
                                                    arg_type,
                                                    msg
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            Some(r)
                        }
                    };
                    declaration_name.last_mut().template_arguments = template_arguments;

                    Ok(CppType::Class(declaration_name))
                } else {
                    bail!("failed to get class declaration: {:?}", type1);
                }
            }
            TypeKind::FunctionPrototype => {
                let mut arguments = Vec::new();
                if let Some(argument_types) = type1.get_argument_types() {
                    for arg_type in argument_types {
                        match self.parse_type(arg_type, context_template_args) {
                            Ok(t) => arguments.push(t),
                            Err(msg) => {
                                bail!(
                                    "Failed to parse function type's argument type: {:?}: {}",
                                    arg_type,
                                    msg
                                );
                            }
                        }
                    }
                } else {
                    bail!(
                        "Failed to parse get argument types from function type: {:?}",
                        type1
                    );
                }
                let return_type = if let Some(result_type) = type1.get_result_type() {
                    match self.parse_type(result_type, context_template_args) {
                        Ok(t) => Box::new(t),
                        Err(msg) => {
                            bail!(
                                "Failed to parse function type's argument type: {:?}: {}",
                                result_type,
                                msg
                            );
                        }
                    }
                } else {
                    bail!(
                        "Failed to parse get result type from function type: {:?}",
                        type1
                    );
                };
                Ok(CppType::FunctionPointer(CppFunctionPointerType {
                    return_type,
                    arguments,
                    allows_variadic_arguments: type1.is_variadic(),
                }))
            }
            TypeKind::Pointer | TypeKind::LValueReference | TypeKind::RValueReference => {
                match type1.get_pointee_type() {
                    Some(pointee) => match self.parse_type(pointee, context_template_args) {
                        Ok(subtype) => {
                            let original_type_indirection = match type1.get_kind() {
                                TypeKind::Pointer => CppPointerLikeTypeKind::Pointer,
                                TypeKind::LValueReference => CppPointerLikeTypeKind::Reference,
                                TypeKind::RValueReference => {
                                    CppPointerLikeTypeKind::RValueReference
                                }
                                _ => unreachable!(),
                            };

                            if original_type_indirection == CppPointerLikeTypeKind::Pointer
                                && subtype.is_function_pointer()
                            {
                                Ok(subtype)
                            } else {
                                Ok(CppType::PointerLike {
                                    kind: original_type_indirection,
                                    is_const: pointee.is_const_qualified(),
                                    target: Box::new(subtype),
                                })
                            }
                        }
                        Err(msg) => Err(msg),
                    },
                    None => bail!("can't get pointee type"),
                }
            }
            TypeKind::Elaborated => {
                self.parse_type(type1.get_canonical_type(), context_template_args)
            }
            TypeKind::Unexposed => {
                trace!("found unexposed type: {:?}", type1);
                let canonical = type1.get_canonical_type();
                trace!("canonical type: {:?}", canonical);
                if canonical.get_kind() == TypeKind::Unexposed {
                    self.parse_unexposed_type(Some(type1), None, context_template_args)
                } else {
                    let mut parsed_canonical = self.parse_type(canonical, context_template_args);
                    if let Ok(CppType::Class(path)) =
                        self.parse_unexposed_type(Some(type1), None, context_template_args)
                    {
                        if let Some(template_arguments_unexposed) = &path.last().template_arguments
                        {
                            if template_arguments_unexposed.iter().any(|x| {
                                matches!(
                                    x,
                                    CppType::SpecificNumeric { .. }
                                        | CppType::PointerSizedInteger { .. }
                                )
                            }) {
                                if let Ok(CppType::Class(path)) = &mut parsed_canonical {
                                    let mut last_item = path.last_mut();
                                    if last_item.template_arguments.is_some() {
                                        last_item.template_arguments =
                                            Some(template_arguments_unexposed.clone());
                                    }
                                }
                            }
                        }
                    }
                    parsed_canonical
                }
            }
            _ => bail!("Unsupported kind of type: {:?}", type1.get_kind()),
        }
    }

    /// Checks if the typedef `name` has a special meaning for the parser.
    fn parse_special_typedef(&self, name: &str) -> Option<CppType> {
        match name {
            "qint8" | "int8_t" | "GLbyte" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_good_str(name),
                    bits: 8,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
                }))
            }
            "quint8" | "uint8_t" | "GLubyte" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_good_str(name),
                    bits: 8,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
                }))
            }
            "qint16" | "int16_t" | "GLshort" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_good_str(name),
                    bits: 16,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
                }))
            }
            "quint16" | "uint16_t" | "GLushort" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_good_str(name),
                    bits: 16,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
                }))
            }
            "qint32" | "int32_t" | "GLint" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_good_str(name),
                    bits: 32,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
                }))
            }
            "quint32" | "uint32_t" | "GLuint" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_good_str(name),
                    bits: 32,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
                }))
            }
            "qint64" | "int64_t" | "qlonglong" | "GLint64" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_good_str(name),
                    bits: 64,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
                }))
            }
            "quint64" | "uint64_t" | "qulonglong" | "GLuint64" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_good_str(name),
                    bits: 64,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
                }))
            }
            "qintptr" | "qptrdiff" | "QList::difference_type" => {
                Some(CppType::PointerSizedInteger {
                    path: CppPath::from_good_str(name),
                    is_signed: true,
                })
            }
            "quintptr" | "size_t" | "std::size_t" => Some(CppType::PointerSizedInteger {
                path: CppPath::from_good_str(name),
                is_signed: false,
            }),
            "std::initializer_list::size_type"
            | "std::__cxx11::basic_string::size_type"
            | "std::vector::size_type" => Some(CppType::PointerSizedInteger {
                path: CppPath::from_good_str("size_t"),
                is_signed: false,
            }),
            _ => None,
        }
    }

    /// Parses a function `entity`.
    #[allow(clippy::cognitive_complexity)]
    fn parse_function(&mut self, entity: Entity<'_>) -> Result<()> {
        let class_name = match entity.get_semantic_parent() {
            Some(p) => match p.get_kind() {
                EntityKind::ClassDecl | EntityKind::ClassTemplate | EntityKind::StructDecl => {
                    match get_path(p) {
                        Ok(class_name) => Some(class_name),
                        Err(msg) => {
                            bail!(
                                "function parent is a class but it doesn't have a name: {}",
                                msg
                            );
                        }
                    }
                }
                EntityKind::ClassTemplatePartialSpecialization => {
                    bail!("this function is part of a template partial specialization");
                }
                _ => None,
            },
            None => None,
        };

        let return_type = if let Some(x) = entity.get_type() {
            if let Some(y) = x.get_result_type() {
                y
            } else {
                bail!("failed to get function return type: {:?}", entity);
            }
        } else {
            bail!("failed to get function type: {:?}", entity);
        };
        let context_template_args = get_context_template_args(entity);
        let return_type_parsed = match self.parse_type(return_type, &context_template_args) {
            Ok(x) => x,
            Err(msg) => {
                trace!("return type: {:?}", return_type);
                bail!(
                    "Can't parse return type: {}: {}",
                    return_type.get_display_name(),
                    msg
                );
            }
        };
        let mut arguments = Vec::new();
        let argument_entities = if entity.get_kind() == EntityKind::FunctionTemplate {
            entity
                .get_children()
                .into_iter()
                .filter(|c| c.get_kind() == EntityKind::ParmDecl)
                .collect()
        } else if let Some(args) = entity.get_arguments() {
            args
        } else {
            bail!("failed to get function arguments: {:?}", entity);
        };

        let mut is_signal = false;
        for (argument_number, argument_entity) in argument_entities.into_iter().enumerate() {
            let name = argument_entity
                .get_name()
                .unwrap_or_else(|| format!("arg{}", argument_number + 1));
            let clang_type = argument_entity.get_type().ok_or_else(|| {
                format_err!(
                    "failed to get type from argument entity: {:?}",
                    argument_entity
                )
            })?;
            if clang_type.get_display_name().ends_with("::QPrivateSignal") {
                is_signal = true;
                continue;
            }
            let argument_type = self
                .parse_type(clang_type, &context_template_args)
                .with_context(|_| {
                    format!(
                        "Can't parse argument type: {}: {}",
                        name,
                        clang_type.get_display_name()
                    )
                })?;
            let mut has_default_value = false;
            for token in argument_entity
                .get_range()
                .ok_or_else(|| {
                    format_err!(
                        "failed to get range from argument entity: {:?}",
                        argument_entity
                    )
                })?
                .tokenize()
            {
                let spelling = token.get_spelling();
                if spelling == "=" {
                    has_default_value = true;
                    break;
                }
                if spelling == "{" {
                    // clang sometimes reports incorrect range for arguments
                    break;
                }
            }
            arguments.push(CppFunctionArgument {
                name,
                argument_type,
                has_default_value,
            });
        }

        let mut name_with_namespace = get_path(entity)?;

        let mut name = entity
            .get_name()
            .ok_or_else(|| err_msg("failed to get function name"))?;
        if name.contains('<') {
            let regex = Regex::new(r"^([\w~]+)<[^<>]+>$")?;
            if let Some(matches) = regex.captures(name.as_ref()) {
                trace!("[DebugParser] Fixing malformed method name: {}", name);
                name = matches
                    .get(1)
                    .ok_or_else(|| err_msg("invalid matches count"))?
                    .as_str()
                    .to_string();
            }
        }

        let template_arguments = match entity.get_kind() {
            EntityKind::FunctionTemplate => {
                if entity
                    .get_children()
                    .into_iter()
                    .any(|c| c.get_kind() == EntityKind::NonTypeTemplateParameter)
                {
                    bail!("Non-type template parameter is not supported");
                }
                get_template_arguments(entity)
            }
            _ => None,
        };

        let allows_variadic_arguments = entity.is_variadic();
        let has_this_argument = class_name.is_some() && !entity.is_static_method();
        let real_arguments_count = arguments.len() + if has_this_argument { 1 } else { 0 };
        let mut method_operator = None;
        if let Some(name_suffix) = name.strip_prefix("operator") {
            let name_suffix = name_suffix.trim();
            let mut name_matches = false;
            for operator in CppOperator::all() {
                let info = operator.info();
                if let Some(s) = info.function_name_suffix {
                    if s == name_suffix {
                        name_matches = true;
                        if info.allows_variadic_arguments
                            || info.arguments_count == real_arguments_count
                        {
                            method_operator = Some(operator);
                            break;
                        }
                    }
                }
            }
            if method_operator.is_none() && name_matches {
                bail!(
                    "This method is recognized as operator but arguments do not match \
                     its signature."
                );
            }
        }

        dump_entity(entity, 0);

        if method_operator.is_none() && name.starts_with("operator ") {
            method_operator = Some(CppOperator::Conversion(return_type_parsed.clone()));
            if let Ok(text) = return_type_parsed.to_cpp_code(None) {
                name = format!("operator {}", text);
            } else {
                name = format!("operator {}", return_type_parsed.to_cpp_pseudo_code());
            }
        }

        name_with_namespace.last_mut().name = name;
        name_with_namespace.last_mut().template_arguments = template_arguments;

        let source_range = entity
            .get_range()
            .ok_or_else(|| err_msg("failed to get range of the function"))?;
        let tokens = source_range.tokenize();
        let declaration_code = if tokens.is_empty() {
            trace!(
                "[DebugParser] Failed to tokenize method {} at {:?}",
                name_with_namespace.to_cpp_pseudo_code(),
                source_range
            );
            let start = source_range.get_start().get_file_location();
            let end = source_range.get_end().get_file_location();
            let file_path = start
                .file
                .ok_or_else(|| err_msg("no file in source location"))?
                .get_path();
            let file = open_file(&file_path)?;
            let mut result = String::new();
            let range_line1 = (start.line - 1) as usize;
            let range_line2 = (end.line - 1) as usize;
            let range_col1 = (start.column - 1) as usize;
            let range_col2 = (end.column - 1) as usize;
            for (line_num, line) in file.lines().enumerate() {
                let line = line.with_context(|_| {
                    format!("failed while reading lines from {}", file_path.display())
                })?;
                if line_num >= range_line1 && line_num <= range_line2 {
                    let start_column = if line_num == range_line1 {
                        range_col1
                    } else {
                        0
                    };
                    let end_column = if line_num == range_line2 {
                        range_col2
                    } else {
                        line.len()
                    };
                    result.push_str(&line[start_column..end_column]);
                    if line_num >= range_line2 {
                        break;
                    }
                }
            }
            if let Some(index) = result.find('{') {
                result = result[0..index].to_string();
            }
            if let Some(index) = result.find(';') {
                result = result[0..index].to_string();
            }
            trace!(
                "[DebugParser] The code extracted directly from header: {:?}",
                result
            );
            if result.contains("volatile") {
                trace!("[DebugParser] Warning: volatile method is detected based on source code");
                bail!("Probably a volatile method.");
            }
            Some(result)
        } else {
            let mut token_strings = Vec::new();
            for token in tokens {
                let text = token.get_spelling();
                if text == "{" || text == ";" {
                    break;
                }
                if text == "volatile" {
                    bail!("A volatile method.");
                }
                token_strings.push(text);
            }
            Some(token_strings.join(" "))
        };

        let function = CppFunction {
            path: name_with_namespace,
            operator: method_operator,
            member: if class_name.is_some() {
                Some(CppFunctionMemberData {
                    kind: match entity.get_kind() {
                        EntityKind::Constructor => CppFunctionKind::Constructor,
                        EntityKind::Destructor => CppFunctionKind::Destructor,
                        _ => CppFunctionKind::Regular,
                    },
                    is_virtual: entity.is_virtual_method(),
                    is_pure_virtual: entity.is_pure_virtual_method(),
                    is_const: entity.is_const_method(),
                    is_static: entity.is_static_method(),
                    visibility: match entity.get_accessibility().unwrap_or(Accessibility::Public) {
                        Accessibility::Public => CppVisibility::Public,
                        Accessibility::Protected => CppVisibility::Protected,
                        Accessibility::Private => CppVisibility::Private,
                    },
                    // not all signals are detected here! see CppData::detect_signals_and_slots
                    is_signal,
                    is_slot: false,
                })
            } else {
                None
            },
            arguments,
            allows_variadic_arguments,
            return_type: return_type_parsed,
            cast: None,
            declaration_code,
        };

        self.add_output(
            self.entity_include_file(entity)?,
            get_origin_location(entity)?,
            CppItem::Function(function),
        )?;

        Ok(())
    }

    /// Parses an enum `entity`.
    fn parse_enum(&mut self, entity: Entity<'_>) -> Result<()> {
        let include_file = self.entity_include_file(entity).with_context(|_| {
            format!(
                "Origin of type is unknown: {}; entity: {:?}",
                get_full_name_display(entity),
                entity
            )
        })?;
        let enum_name = get_path(entity)?;
        self.add_output(
            include_file.clone(),
            get_origin_location(entity)?,
            CppItem::Type(CppTypeDeclaration {
                kind: CppTypeDeclarationKind::Enum,
                path: enum_name.clone(),
            }),
        )?;
        for child in entity.get_children() {
            if child.get_kind() == EntityKind::EnumConstantDecl {
                let val = child
                    .get_enum_constant_value()
                    .ok_or_else(|| err_msg("failed to get value of enum variant"))?;

                let value_name = child
                    .get_name()
                    .ok_or_else(|| err_msg("failed to get name of enum variant"))?;
                self.add_output(
                    include_file.clone(),
                    get_origin_location(child)?,
                    CppItem::EnumValue(CppEnumValue {
                        path: enum_name.join(CppPathItem::from_good_str(&value_name)),
                        value: val.0,
                    }),
                )?;
            }
        }
        Ok(())
    }

    /// Parses a class field `entity`.
    fn parse_class_field(&mut self, entity: Entity<'_>, class_type: &CppPath) -> Result<()> {
        let include_file = self
            .entity_include_file(entity)
            .with_context(|_| err_msg("Origin of class field is unknown"))?;
        let field_name = entity
            .get_name()
            .ok_or_else(|| err_msg("failed to get field name"))?;
        let field_clang_type = entity
            .get_type()
            .ok_or_else(|| err_msg("failed to get field type"))?;
        let field_type = self
            .parse_type(field_clang_type, &get_context_template_args(entity))
            .with_context(|_| err_msg("failed to parse field type"))?;
        self.add_output(
            include_file,
            get_origin_location(entity)?,
            CppItem::ClassField(CppClassField {
                path: class_type.join(CppPathItem::from_good_str(&field_name)),
                field_type,
                visibility: match entity.get_accessibility().unwrap_or(Accessibility::Public) {
                    Accessibility::Public => CppVisibility::Public,
                    Accessibility::Protected => CppVisibility::Protected,
                    Accessibility::Private => CppVisibility::Private,
                },
                is_static: entity.get_kind() == EntityKind::VarDecl,
            }),
        )?;

        Ok(())
    }

    // we pass parent manually because both lexical and semantic parent are missing for these
    // entities for some reason
    fn parse_class_base(
        &mut self,
        entity: Entity<'_>,
        base_index: usize,
        parent: Entity<'_>,
    ) -> Result<()> {
        let base_type = self
            .parse_type(
                entity.get_type().unwrap(),
                &get_context_template_args(parent),
            )
            .with_context(|_| "Can't parse base class type")?;
        if let CppType::Class(base_type) = &base_type {
            self.add_output(
                self.entity_include_file(entity)?,
                get_origin_location(entity).unwrap(),
                CppItem::ClassBase(CppBaseSpecifier {
                    base_class_type: base_type.clone(),
                    is_virtual: entity.is_virtual_base(),
                    visibility: match entity.get_accessibility().unwrap_or(Accessibility::Public) {
                        Accessibility::Public => CppVisibility::Public,
                        Accessibility::Protected => CppVisibility::Protected,
                        Accessibility::Private => CppVisibility::Private,
                    },
                    base_index,
                    derived_class_type: get_path(parent)?,
                }),
            )?;
        } else {
            bail!("base type is not a class: {:?}", base_type);
        }
        Ok(())
    }

    /// Parses a class or a struct `entity`.
    fn parse_class(&mut self, entity: Entity<'_>) -> Result<()> {
        let include_file = self.entity_include_file(entity).with_context(|_| {
            format!(
                "Origin of type is unknown: {}; entity: {:?}",
                get_full_name_display(entity),
                entity
            )
        })?;
        let full_name = get_path(entity)?;
        let template_arguments = get_template_arguments(entity);
        if entity.get_kind() == EntityKind::ClassTemplate {
            if entity
                .get_children()
                .into_iter()
                .any(|c| c.get_kind() == EntityKind::NonTypeTemplateParameter)
            {
                bail!("Non-type template parameter is not supported");
            }

            if template_arguments.is_none() {
                dump_entity(entity, 0);
                bail!("missing template arguments");
            }
        } else if template_arguments.is_some() {
            bail!("unexpected template arguments");
        }
        let mut current_base_index = 0;
        for child in entity.get_children() {
            if child.get_kind() == EntityKind::FieldDecl || child.get_kind() == EntityKind::VarDecl
            {
                if let Err(err) = self.parse_class_field(child, &full_name) {
                    debug!(
                        "failed to parse class field: {}: {}",
                        get_full_name_display(child),
                        err
                    );
                    trace!("entity: {:?}", entity);
                }
            }
            if child.get_kind() == EntityKind::BaseSpecifier {
                if let Err(err) = self.parse_class_base(child, current_base_index, entity) {
                    debug!(
                        "failed to parse class base: {}: {}",
                        get_full_name_display(entity),
                        err
                    );
                }
                current_base_index += 1;
            }
            if child.get_kind() == EntityKind::NonTypeTemplateParameter {
                bail!("Non-type template parameter is not supported");
            }
        }
        self.add_output(
            include_file,
            get_origin_location(entity).unwrap(),
            CppItem::Type(CppTypeDeclaration {
                kind: CppTypeDeclarationKind::Class,
                path: full_name,
            }),
        )?;
        Ok(())
    }

    /// Determines file path of the include file this `entity` is located in.
    fn entity_include_path(&self, entity: Entity<'_>) -> Result<PathBuf> {
        if let Some(location) = entity.get_location() {
            let file_path = location.get_presumed_location().0;
            if file_path.is_empty() {
                bail!("empty file path")
            } else {
                Ok(canonicalize(file_path)?)
            }
        } else {
            bail!("no location for entity")
        }
    }

    /// Determines file name of the include file this `entity` is located in.
    fn entity_include_file(&self, entity: Entity<'_>) -> Result<String> {
        let file_path_buf = self.entity_include_path(entity)?;
        let file_name = file_path_buf
            .file_name()
            .ok_or_else(|| err_msg("no file name in file path"))?;
        Ok(os_str_to_str(file_name)?.to_string())
    }

    /// Returns false if this `entity` was blacklisted in some way.
    fn should_process_entity(&self, entity: Entity<'_>) -> Result<bool> {
        if entity.get_kind() == EntityKind::TranslationUnit {
            return Ok(true);
        }
        if let Ok(file_path) = self.entity_include_path(entity) {
            let file_path = canonicalize(Path::new(&file_path))?;
            if !self.current_target_paths.is_empty()
                && !self
                    .current_target_paths
                    .iter()
                    .any(|x| file_path.starts_with(x))
            {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
        if let Ok(full_name) = get_path(entity) {
            if let Some(hook) = self.data.config.cpp_parser_path_hook() {
                if !hook(&full_name)? {
                    return Ok(false);
                }
            }
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    fn parse(&mut self, entity: Entity<'_>) -> Result<()> {
        debug!("Parsing types");
        self.parse_types(entity)?;
        debug!("Parsing functions");
        self.parse_functions(entity)?;
        for hook in self.data.config.after_cpp_parser_hooks() {
            hook(self.data.reborrow(), &self.output)?;
        }
        Ok(())
    }

    /// Parses type declarations in translation unit `entity`
    /// and saves them to `self`.
    fn parse_types(&mut self, entity: Entity<'_>) -> Result<()> {
        if !self.should_process_entity(entity)? {
            return Ok(());
        }
        match entity.get_kind() {
            EntityKind::EnumDecl => {
                if entity.get_accessibility() == Some(Accessibility::Private) {
                    return Ok(()); // skipping private stuff
                }
                if entity.get_name().is_some() && entity.is_definition() {
                    if let Err(error) = self.parse_enum(entity) {
                        debug!(
                            "failed to parse enum: {}: {}",
                            get_full_name_display(entity),
                            error
                        );
                        trace!("entity: {:?}", entity);
                    }
                }
            }
            EntityKind::ClassDecl | EntityKind::ClassTemplate | EntityKind::StructDecl => {
                if entity.get_accessibility() == Some(Accessibility::Private) {
                    return Ok(()); // skipping private stuff
                }
                let ok = entity.get_name().is_some() && // not an anonymous struct
                    entity.is_definition() && // not a forward declaration
                    entity.get_template().is_none(); // not a template specialization
                if ok {
                    if let Err(error) = self.parse_class(entity) {
                        debug!(
                            "failed to parse class: {}: {}",
                            get_full_name_display(entity),
                            error
                        );
                        trace!("entity: {:?}", entity);
                    }
                }
            }
            EntityKind::Namespace => match get_path(entity) {
                Ok(path) => {
                    self.add_output(
                        self.entity_include_file(entity)?,
                        get_origin_location(entity).unwrap(),
                        CppItem::Namespace(CppNamespace { path }),
                    )?;
                }
                Err(error) => debug!("failed to get namespace name: {}", error),
            },
            _ => {}
        }
        match entity.get_kind() {
            EntityKind::TranslationUnit
            | EntityKind::Namespace
            | EntityKind::StructDecl
            | EntityKind::ClassDecl
            | EntityKind::UnexposedDecl
            | EntityKind::ClassTemplate => {
                for c in entity.get_children() {
                    self.parse_types(c)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Parses methods in translation unit `entity`.
    fn parse_functions(&mut self, entity: Entity<'_>) -> Result<()> {
        if !self.should_process_entity(entity)? {
            return Ok(());
        }
        match entity.get_kind() {
            EntityKind::FunctionDecl
            | EntityKind::Method
            | EntityKind::Constructor
            | EntityKind::Destructor
            | EntityKind::ConversionFunction
            | EntityKind::FunctionTemplate => {
                if let Err(error) = self.parse_function(entity) {
                    debug!(
                        "failed to parse function: {}: {}",
                        get_full_name_display(entity),
                        error
                    );
                    print_trace(&error, Some(log::Level::Trace));
                    trace!("entity: {:?}", entity);
                }
            }
            EntityKind::StructDecl
            | EntityKind::ClassDecl
            | EntityKind::ClassTemplate
            | EntityKind::ClassTemplatePartialSpecialization => {
                if let Some(name) = entity.get_display_name() {
                    if let Ok(CppType::Class(parent_type_path)) = self.parse_unexposed_type(
                        None,
                        Some(name),
                        &get_context_template_args(entity),
                    ) {
                        if let Some(template_arguments) =
                            &parent_type_path.last().template_arguments
                        {
                            if template_arguments
                                .iter()
                                .any(|x| !x.is_template_parameter())
                            {
                                trace!(
                                    "skipping template partial specialization: {}",
                                    get_full_name_display(entity),
                                );
                                trace!("entity: {:?}", entity);
                                return Ok(());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        match entity.get_kind() {
            EntityKind::TranslationUnit
            | EntityKind::Namespace
            | EntityKind::StructDecl
            | EntityKind::ClassDecl
            | EntityKind::UnexposedDecl
            | EntityKind::ClassTemplate => {
                for c in entity.get_children() {
                    self.parse_functions(c)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn parse_template_args(str: &str) -> Option<(String, Vec<String>)> {
    let mut level = 0;
    let mut current_str = String::new();
    let mut name = String::new();
    let mut args = Vec::new();
    let mut chars = str.chars().peekable();
    while let Some(char) = chars.next() {
        match char {
            '<' => {
                if level == 0 {
                    name = std::mem::take(&mut current_str);
                } else {
                    current_str.push(char);
                }
                level += 1;
            }
            '>' => {
                level -= 1;
                if level == 0 {
                    args.push(std::mem::take(&mut current_str));
                    if chars.peek().is_some() {
                        return None;
                    } else {
                        return Some((name, args));
                    }
                } else {
                    current_str.push(char);
                }
            }
            ',' => {
                if level == 0 {
                    return None;
                } else if level == 1 {
                    args.push(std::mem::take(&mut current_str));
                } else {
                    current_str.push(char);
                }
            }
            c => current_str.push(c),
        }
    }
    None
}

#[test]
fn should_parse_template_args_works() {
    assert_eq!(
        parse_template_args("name<arg, arg2>"),
        Some((
            "name".to_string(),
            vec!["arg".to_string(), " arg2".to_string()]
        ))
    );
    assert_eq!(
        parse_template_args("name<arg, name2<arg2, arg3>>"),
        Some((
            "name".to_string(),
            vec!["arg".to_string(), " name2<arg2, arg3>".to_string()]
        ))
    );
    assert_eq!(parse_template_args("name<arg, arg2>bad"), None);
    assert_eq!(parse_template_args("name<arg,arg2"), None);
    assert_eq!(parse_template_args("name<arg<arg3,arg4>,arg2"), None);
}
