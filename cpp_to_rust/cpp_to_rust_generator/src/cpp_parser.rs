use crate::common::errors::{
    bail, err_msg, should_panic_on_unexpected, unexpected, Result, ResultExt,
};
use crate::common::file_utils::{create_file, open_file, os_str_to_str, path_to_str, remove_file};
use crate::common::log;
use crate::common::string_utils::JoinWithSeparator;
use crate::cpp_data::{
    CppBaseSpecifier, CppClassField, CppEnumValue, CppOriginLocation, CppTypeData, CppVisibility,
};
use crate::cpp_function::{
    CppFunction, CppFunctionArgument, CppFunctionKind, CppFunctionMemberData,
};
use crate::cpp_operator::CppOperator;
use crate::cpp_type::{
    CppBuiltInNumericType, CppFunctionPointerType, CppSpecificNumericType,
    CppSpecificNumericTypeKind, CppType,
};
use crate::database::CppItemData;
use std::str::FromStr;

use clang;
use clang::*;

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::cpp_data::CppTypeDataKind;
use crate::database::DatabaseItemSource;

use crate::cpp_data::CppPath;
use crate::cpp_data::CppPathItem;
use crate::cpp_type::CppPointerLikeTypeKind;
use crate::processor::ProcessingStep;
use crate::processor::ProcessorData;
use regex::Regex;
use std::collections::HashSet;
use std::iter::once;

fn entity_log_representation(entity: Entity) -> String {
    format!("{}; {:?}", get_full_name_display(entity), entity)
}

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

/// Implementation of the C++ parser that extracts information
/// about the C++ library's API from its headers.
struct CppParser<'b, 'a: 'b> {
    data: &'b mut ProcessorData<'a>,
}

/// Print representation of `entity` and its children to the log.
/// `level` is current level of recursion.
#[allow(dead_code)]
fn dump_entity(entity: Entity, level: usize) {
    log::llog(log::DebugParser, || {
        format!("{}{:?}", (0..level).map(|_| ". ").join(""), entity)
    });
    if level <= 5 {
        for child in entity.get_children() {
            dump_entity(child, level + 1);
        }
    }
}

/// Extract `clang`'s location information for `entity` to `CppOriginLocation`.
fn get_origin_location(entity: Entity) -> Result<CppOriginLocation> {
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
fn get_template_arguments(entity: Entity) -> Option<Vec<CppType>> {
    let mut nested_level = 0;
    if let Some(parent) = entity.get_semantic_parent() {
        if let Some(args) = get_template_arguments(parent) {
            let parent_nested_level =
                if let CppType::TemplateParameter { nested_level, .. } = args[0] {
                    nested_level
                } else {
                    panic!("this value should always be a template parameter")
                };

            nested_level = parent_nested_level + 1;
        }
    }
    let args: Vec<_> = entity
        .get_children()
        .into_iter()
        .filter(|c| c.get_kind() == EntityKind::TemplateTypeParameter)
        .enumerate()
        .map(|(i, c)| CppType::TemplateParameter {
            name: c.get_name().unwrap_or_else(|| format!("Type{}", i + 1)),
            index: i,
            nested_level,
        })
        .collect();
    if args.is_empty() {
        None
    } else {
        Some(args)
    }
}

fn get_path_item(entity: Entity) -> Result<CppPathItem> {
    let name = entity.get_name().ok_or_else(|| err_msg("Anonymous type"))?;
    let template_arguments = get_template_arguments(entity);
    Ok(CppPathItem {
        name,
        template_arguments,
    })
}

/// Returns fully qualified name of `entity`.
fn get_full_name(entity: Entity) -> Result<CppPath> {
    let mut current_entity = entity;
    let mut parts = vec![get_path_item(entity)?];
    while let Some(p) = current_entity.get_semantic_parent() {
        match p.get_kind() {
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
            _ => break, // TODO: panic?
        }
    }
    Ok(CppPath { items: parts })
}

fn get_full_name_display(entity: Entity) -> String {
    match get_full_name(entity) {
        Ok(name) => name.to_string(),
        Err(_) => "[unnamed]".into(),
    }
}

#[cfg(test)]
fn init_clang() -> Result<Clang> {
    use std;
    for _ in 0..600 {
        if let Ok(clang) = Clang::new() {
            return Ok(clang);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Clang::new().map_err(|err| err_msg(format!("clang init failed: {}", err)))
}

#[cfg(not(test))]
/// Creates a `Clang` context.
fn init_clang() -> Result<Clang> {
    Clang::new().map_err(|err| err_msg(format!("clang init failed: {}", err)))
}

/// Runs `clang` parser with `config`.
/// If `cpp_code` is specified, it's written to the C++ file before parsing it.
/// If successful, calls `f` and passes the topmost entity (the translation unit)
/// as its argument. Returns output value of `f` or an error.
#[allow(clippy::block_in_if_condition_stmt)]
fn run_clang<R, F: FnMut(Entity) -> Result<R>>(
    config: &Config,
    tmp_path: &Path,
    cpp_code: Option<String>,
    mut f: F,
) -> Result<R> {
    let clang = init_clang()?;
    let index = Index::new(&clang, false, false);
    let tmp_cpp_path = tmp_path.join("1.cpp");
    {
        let mut tmp_file = create_file(&tmp_cpp_path)?;
        for directive in config.include_directives() {
            tmp_file.write(format!("#include \"{}\"\n", path_to_str(directive)?))?;
        }
        if let Some(cpp_code) = cpp_code {
            tmp_file.write(cpp_code)?;
        }
    }
    let mut args = vec![
        "-Xclang".to_string(),
        "-detailed-preprocessing-record".to_string(),
    ];
    args.extend_from_slice(config.cpp_parser_arguments());
    for dir in config.cpp_build_paths().include_paths() {
        let str = path_to_str(dir)?;
        args.push("-I".to_string());
        args.push(str.to_string());
    }
    if let Ok(path) = ::std::env::var("CLANG_SYSTEM_INCLUDE_PATH") {
        if !Path::new(&path).exists() {
            log::error(format!(
                "Warning: CLANG_SYSTEM_INCLUDE_PATH environment variable is set to \"{}\" \
                 but this path does not exist.",
                path
            ));
            log::error("This may result in parse errors related to system header includes.");
        }
        args.push("-isystem".to_string());
        args.push(path);
    } else {
        log::error("Warning: CLANG_SYSTEM_INCLUDE_PATH environment variable is not set.");
        log::error("This may result in parse errors related to system header includes.");
    }
    for dir in config.cpp_build_paths().framework_paths() {
        let str = path_to_str(dir)?;
        args.push("-F".to_string());
        args.push(str.to_string());
    }
    log::status(format!("clang arguments: {:?}", args));

    let tu = index
        .parser(&tmp_cpp_path)
        .arguments(&args)
        .parse()
        .with_context(|_| "clang parse failed")?;
    let translation_unit = tu.get_entity();
    assert!(translation_unit.get_kind() == EntityKind::TranslationUnit);
    {
        let diagnostics = tu.get_diagnostics();
        if !diagnostics.is_empty() {
            log::llog(log::DebugParser, || "Diagnostics:");
            for diag in &diagnostics {
                log::llog(log::DebugParser, || format!("{}", diag));
            }
        }
        if diagnostics.iter().any(|d| {
            d.get_severity() == clang::diagnostic::Severity::Error
                || d.get_severity() == clang::diagnostic::Severity::Fatal
        }) {
            bail!(
                "fatal clang error:\n{}",
                diagnostics.iter().map(|d| d.to_string()).join("\n")
            );
        }
    }
    let result = f(translation_unit);
    remove_file(&tmp_cpp_path)?;
    result
}

fn add_namespaces(data: &mut ProcessorData) -> Result<()> {
    let mut namespaces = HashSet::new();
    for item in &data.current_database.items {
        let name = match item.cpp_data {
            CppItemData::Type(ref t) => &t.path,
            CppItemData::Function(ref f) => &f.path,
            _ => continue,
        };
        if name.items.len() == 1 {
            continue;
        }
        let mut namespace_name = name.clone();
        namespace_name.items.pop().expect("name is empty");
        namespaces.insert(namespace_name.clone());
        while let Some(_) = namespace_name.items.pop() {
            if !namespace_name.items.is_empty() {
                namespaces.insert(namespace_name.clone());
            }
        }
    }
    for item in &data.current_database.items {
        if let CppItemData::Type(ref t) = item.cpp_data {
            namespaces.remove(&t.path);
        }
    }
    for name in namespaces {
        let item = CppItemData::Namespace(name);
        data.current_database
            .add_cpp_data(DatabaseItemSource::NamespaceInfering, item);
    }
    Ok(())
}

/// Runs the parser on specified data.
fn run(data: &mut ProcessorData) -> Result<()> {
    log::status(get_version());
    log::status("Initializing clang...");
    //let (mut parser, methods) =
    let mut parser = CppParser { data };
    parser.data.html_logger.add_header(&["Item", "Status"])?;
    run_clang(
        &parser.data.config,
        &parser.data.workspace.tmp_path()?,
        None,
        |translation_unit| {
            log::status("Parsing types");
            parser.parse_types(translation_unit)?;
            log::status("Parsing methods");
            parser.parse_functions(translation_unit)?;
            Ok(())
        },
    )?;
    add_namespaces(parser.data)?;
    Ok(())
}

pub fn cpp_parser_step() -> ProcessingStep {
    ProcessingStep::new("cpp_parser", Vec::new(), run)
}

impl CppParser<'_, '_> {
    /// Search for a C++ type information in the types found by the parser
    /// and in types of the dependencies.
    fn find_type<F: Fn(&CppTypeData) -> bool>(&self, f: F) -> Option<&CppTypeData> {
        let databases =
            once(self.data.current_database as &_).chain(self.data.dep_databases.iter());
        for database in databases {
            for item in database.items() {
                if let CppItemData::Type(ref info) = item.cpp_data {
                    if f(info) {
                        return Some(info);
                    }
                }
            }
        }
        None
    }

    /// Attempts to parse an unexposed type, i.e. a type the used `clang` API
    /// is not able to describe. Either `type1` or `string` must be specified,
    /// and both may be specified at the same time.
    /// Surrounding class and/or
    /// method may be specified in `context_class` and `context_method`.
    #[allow(clippy::cyclomatic_complexity)]
    fn parse_unexposed_type(
        &self,
        type1: Option<Type>,
        string: Option<String>,
        context_class: Option<Entity>,
        context_method: Option<Entity>,
    ) -> Result<CppType> {
        let template_class_regex = Regex::new(r"^([\w:]+)<(.+)>$")?;
        let (is_const, name) = if let Some(type1) = type1 {
            let is_const = type1.is_const_qualified();
            let mut name = type1.get_display_name();
            let is_const_in_name = name.starts_with("const ");
            if is_const != is_const_in_name {
                unexpected!("const inconsistency: {}, {:?}", is_const, type1);
            }
            if is_const_in_name {
                name = name[6..].to_string();
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
                    if let Some(matches) = template_class_regex.captures(name.as_ref()) {
                        let mut arg_types = Vec::new();
                        if let Some(items) = matches.get(2) {
                            for arg in items.as_str().split(',') {
                                match self.parse_unexposed_type(
                                    None,
                                    Some(arg.trim().to_string()),
                                    context_class,
                                    context_method,
                                ) {
                                    Ok(arg_type) => arg_types.push(arg_type),
                                    Err(msg) => {
                                        bail!("Template argument of unexposed type is not parsed: {}: {}",                        arg, msg
                      );
                                    }
                                }
                            }
                        } else {
                            unexpected!("invalid matches count in regexp");
                        }
                        let mut name = get_full_name(declaration)?;
                        let mut last_item = name.items.pop().expect("CppPath can't be empty");
                        last_item.template_arguments = Some(arg_types);
                        name.items.push(last_item);
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
            return Ok(CppType::TemplateParameter {
                nested_level: matches[1].parse::<usize>().with_context(|_| {
                    "encountered not a number while parsing type-parameter-X-X"
                })?,
                index: matches[2].parse::<usize>().with_context(|_| {
                    "encountered not a number while parsing type-parameter-X-X"
                })?,
                name: name.clone(),
            });
        }
        if let Some(e) = context_method {
            if let Some(args) = get_template_arguments(e) {
                if let Some(arg) = args.iter().find(|t| t.to_cpp_pseudo_code() == name) {
                    return Ok(arg.clone());
                }
            }
        }
        if let Some(e) = context_class {
            if let Some(args) = get_template_arguments(e) {
                if let Some(arg) = args.iter().find(|t| t.to_cpp_pseudo_code() == name) {
                    return Ok(arg.clone());
                }
            }
        }

        if name.ends_with(" *") {
            let remaining_name = name[0..name.len() - " *".len()].trim();
            let subtype = self.parse_unexposed_type(
                None,
                Some(remaining_name.to_string()),
                context_class,
                context_method,
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
                context_class,
                context_method,
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
        if let Some(type_data) = self.find_type(|x| x.path.to_string() == name) {
            match type_data.kind {
                CppTypeDataKind::Enum { .. } => {
                    return Ok(CppType::Enum {
                        path: CppPath::from_str(&name)?,
                    });
                }
                CppTypeDataKind::Class { .. } => {
                    return Ok(CppType::Class(CppPath::from_str(&name)?));
                }
            }
        }

        if let Some(matches) = template_class_regex.captures(&name) {
            if matches.len() < 3 {
                bail!("invalid matches len in regexp");
            }
            let mut class_name = CppPath::from_str(&matches[1])?;
            if self
                .find_type(|x| x.path == class_name && x.kind.is_class())
                .is_some()
            {
                let mut arg_types = Vec::new();
                for arg in matches[2].split(',') {
                    match self.parse_unexposed_type(
                        None,
                        Some(arg.trim().to_string()),
                        context_class,
                        context_method,
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
                let mut last_part = class_name.items.pop().expect("CppPath can't be empty");
                last_part.template_arguments = Some(arg_types);
                println!("pushing1 {:?}", last_part);
                class_name.items.push(last_part);
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
    fn parse_type(
        &self,
        type1: Type,
        context_class: Option<Entity>,
        context_method: Option<Entity>,
    ) -> Result<CppType> {
        println!(
            "parse_type {:?} {:?} {:?}",
            type1, context_class, context_method
        );
        if type1.is_volatile_qualified() {
            bail!("Volatile type");
        }
        let display_name = type1.get_display_name();
        if &display_name == "std::list<T>" {
            bail!(
                "Type blacklisted because it causes crash on Windows: {}",
                display_name
            );
        }
        match type1.get_kind() {
            TypeKind::Typedef => {
                let parsed =
                    self.parse_type(type1.get_canonical_type(), context_class, context_method)?;
                if let CppType::BuiltInNumeric(..) = parsed {
                    let mut name = type1.get_display_name();
                    if name.starts_with("const ") {
                        name = name[6..].trim().to_string();
                    }
                    if let Some(r) = self.parse_special_typedef(&name) {
                        return Ok(r);
                    }
                }
                Ok(parsed)
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
                    Ok(CppType::Enum {
                        path: get_full_name(declaration)?,
                    })
                } else {
                    bail!("failed to get enum declaration: {:?}", type1);
                }
            }
            TypeKind::Elaborated | TypeKind::Record => {
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
                    let mut declaration_name = get_full_name(declaration)?;
                    let template_arguments = match type1.get_template_argument_types() {
                        None => None,
                        Some(arg_types) => {
                            let mut r = Vec::new();
                            if arg_types.is_empty() {
                                unexpected!("arg_types is empty");
                            }
                            for arg_type in arg_types {
                                match arg_type {
                                    None => bail!("Template argument is None"),
                                    Some(arg_type) => {
                                        match self.parse_type(
                                            arg_type,
                                            context_class,
                                            context_method,
                                        ) {
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
                    let mut last_part = declaration_name
                        .items
                        .pop()
                        .expect("CppPath can't be empty");
                    last_part.template_arguments = template_arguments;
                    println!("pushing2 {:?}", last_part);
                    declaration_name.items.push(last_part);

                    Ok(CppType::Class(declaration_name))
                } else {
                    bail!("failed to get class declaration: {:?}", type1);
                }
            }
            TypeKind::FunctionPrototype => {
                let mut arguments = Vec::new();
                if let Some(argument_types) = type1.get_argument_types() {
                    for arg_type in argument_types {
                        match self.parse_type(arg_type, context_class, context_method) {
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
                    match self.parse_type(result_type, context_class, context_method) {
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
                    Some(pointee) => {
                        match self.parse_type(pointee, context_class, context_method) {
                            Ok(subtype) => {
                                let original_type_indirection = match type1.get_kind() {
                                    TypeKind::Pointer => CppPointerLikeTypeKind::Pointer,
                                    TypeKind::LValueReference => CppPointerLikeTypeKind::Reference,
                                    TypeKind::RValueReference => {
                                        CppPointerLikeTypeKind::RValueReference
                                    }
                                    _ => unreachable!(),
                                };

                                Ok(CppType::PointerLike {
                                    kind: original_type_indirection,
                                    is_const: pointee.is_const_qualified(),
                                    target: Box::new(subtype),
                                })
                            }
                            Err(msg) => Err(msg),
                        }
                    }
                    None => bail!("can't get pointee type"),
                }
            }
            TypeKind::Unexposed => {
                println!("OKKK?");
                let canonical = type1.get_canonical_type();
                if canonical.get_kind() == TypeKind::Unexposed {
                    self.parse_unexposed_type(Some(type1), None, context_class, context_method)
                } else {
                    let mut parsed_canonical =
                        self.parse_type(canonical, context_class, context_method);
                    if let Ok(parsed_unexposed) =
                        self.parse_unexposed_type(Some(type1), None, context_class, context_method)
                    {
                        if let CppType::Class(path) = parsed_unexposed {
                            if let Some(ref template_arguments_unexposed) =
                                path.last().template_arguments
                            {
                                if template_arguments_unexposed.iter().any(|x| match x {
                                    CppType::SpecificNumeric { .. }
                                    | CppType::PointerSizedInteger { .. } => true,
                                    _ => false,
                                }) {
                                    if let Ok(ref mut parsed_canonical) = parsed_canonical {
                                        if let CppType::Class(ref mut path) = parsed_canonical {
                                            let mut last_item =
                                                path.items.pop().expect("CppPath can't be empty");
                                            if last_item.template_arguments.is_some() {
                                                last_item.template_arguments =
                                                    Some(template_arguments_unexposed.clone());
                                            }
                                            path.items.push(last_item);
                                        }
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
                    path: CppPath::from_str_unchecked(name),
                    bits: 8,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
                }))
            }
            "quint8" | "uint8_t" | "GLubyte" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_str_unchecked(name),
                    bits: 8,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
                }))
            }
            "qint16" | "int16_t" | "GLshort" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_str_unchecked(name),
                    bits: 16,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
                }))
            }
            "quint16" | "uint16_t" | "GLushort" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_str_unchecked(name),
                    bits: 16,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
                }))
            }
            "qint32" | "int32_t" | "GLint" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_str_unchecked(name),
                    bits: 32,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
                }))
            }
            "quint32" | "uint32_t" | "GLuint" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_str_unchecked(name),
                    bits: 32,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
                }))
            }
            "qint64" | "int64_t" | "qlonglong" | "GLint64" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_str_unchecked(name),
                    bits: 64,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
                }))
            }
            "quint64" | "uint64_t" | "qulonglong" | "GLuint64" => {
                Some(CppType::SpecificNumeric(CppSpecificNumericType {
                    path: CppPath::from_str_unchecked(name),
                    bits: 64,
                    kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
                }))
            }
            "qintptr" | "qptrdiff" | "QList::difference_type" => {
                Some(CppType::PointerSizedInteger {
                    path: CppPath::from_str_unchecked(name),
                    is_signed: true,
                })
            }
            "quintptr" => Some(CppType::PointerSizedInteger {
                path: CppPath::from_str_unchecked(name),
                is_signed: false,
            }),
            _ => None,
        }
    }

    /// Parses a function `entity`.
    #[allow(clippy::cyclomatic_complexity)]
    fn parse_function(&self, entity: Entity) -> Result<(CppFunction, DatabaseItemSource)> {
        let (class_name, class_entity) = match entity.get_semantic_parent() {
            Some(p) => match p.get_kind() {
                EntityKind::ClassDecl | EntityKind::ClassTemplate | EntityKind::StructDecl => {
                    match get_full_name(p) {
                        Ok(class_name) => (Some(class_name), Some(p)),
                        Err(msg) => {
                            bail!(
                                "function parent is a class but it doesn't have a name: {}",
                                msg
                            );
                        }
                    }
                }
                EntityKind::ClassTemplatePartialSpecialization => {
                    bail!("this function is part of a template partial specialization");;
                }
                _ => (None, None),
            },
            None => (None, None),
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
        let return_type_parsed = match self.parse_type(return_type, class_entity, Some(entity)) {
            Ok(x) => x,
            Err(msg) => {
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
                err_msg(format!(
                    "failed to get type from argument entity: {:?}",
                    argument_entity
                ))
            })?;
            if clang_type.get_display_name().ends_with("::QPrivateSignal") {
                is_signal = true;
                continue;
            }
            let argument_type = self
                .parse_type(clang_type, class_entity, Some(entity))
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
                    err_msg(format!(
                        "failed to get range from argument entity: {:?}",
                        argument_entity
                    ))
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

        let mut name_with_namespace = get_full_name(entity)?;

        let mut name = entity
            .get_name()
            .ok_or_else(|| err_msg("failed to get function name"))?;
        if name.contains('<') {
            let regex = Regex::new(r"^([\w~]+)<[^<>]+>$")?;
            if let Some(matches) = regex.captures(name.clone().as_ref()) {
                log::llog(log::DebugParser, || {
                    format!("Fixing malformed method name: {}", name)
                });
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

        name_with_namespace.last_mut().name = name.clone();
        name_with_namespace.last_mut().template_arguments = template_arguments;

        let allows_variadic_arguments = entity.is_variadic();
        let has_this_argument = class_name.is_some() && !entity.is_static_method();
        let real_arguments_count = arguments.len() + if has_this_argument { 1 } else { 0 };
        let mut method_operator = None;
        if name.starts_with("operator") {
            let name_suffix = name["operator".len()..].trim();
            let mut name_matches = false;
            for operator in CppOperator::all() {
                let info = operator.info();
                if let Some(s) = info.function_name_suffix {
                    if s == name_suffix {
                        name_matches = true;
                        if info.allows_variadic_arguments
                            || info.arguments_count == real_arguments_count
                        {
                            method_operator = Some(operator.clone());
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

        if method_operator.is_none() && name.starts_with("operator ") {
            let op = name["operator ".len()..].trim();
            match self.parse_unexposed_type(None, Some(op.to_string()), class_entity, Some(entity))
            {
                Ok(t) => method_operator = Some(CppOperator::Conversion(t)),
                Err(_) => bail!("Unknown type in conversion operator: '{}'", op),
            }
        }
        let source_range = entity
            .get_range()
            .ok_or_else(|| err_msg("failed to get range of the function"))?;
        let tokens = source_range.tokenize();
        let declaration_code = if tokens.is_empty() {
            log::llog(log::DebugParser, || {
                format!(
                    "Failed to tokenize method {} at {:?}",
                    name_with_namespace, source_range
                )
            });
            let start = source_range.get_start().get_file_location();
            let end = source_range.get_end().get_file_location();
            let file_path = start
                .file
                .ok_or_else(|| err_msg("no file in source location"))?
                .get_path();
            let file = open_file(&file_path)?;
            let reader = BufReader::new(file.into_file());
            let mut result = String::new();
            let range_line1 = (start.line - 1) as usize;
            let range_line2 = (end.line - 1) as usize;
            let range_col1 = (start.column - 1) as usize;
            let range_col2 = (end.column - 1) as usize;
            for (line_num, line) in reader.lines().enumerate() {
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
            log::llog(log::DebugParser, || {
                format!("The code extracted directly from header: {:?}", result)
            });
            if result.contains("volatile") {
                log::llog(log::DebugParser, || {
                    "Warning: volatile method is detected based on source code".to_string()
                });
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
        Ok((
            CppFunction {
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
                        visibility: match entity
                            .get_accessibility()
                            .unwrap_or(Accessibility::Public)
                        {
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
                declaration_code,
                doc: None,
            },
            DatabaseItemSource::CppParser {
                include_file: self.entity_include_file(entity)?,
                origin_location: get_origin_location(entity)?,
            },
        ))
    }

    /// Parses an enum `entity`.
    fn parse_enum(&mut self, entity: Entity) -> Result<()> {
        let include_file = self.entity_include_file(entity).with_context(|_| {
            format!(
                "Origin of type is unknown: {}; entity: {:?}",
                get_full_name_display(entity),
                entity
            )
        })?;
        let enum_name = get_full_name(entity)?;
        self.data.current_database.add_cpp_data(
            DatabaseItemSource::CppParser {
                include_file: include_file.clone(),
                origin_location: get_origin_location(entity)?,
            },
            CppItemData::Type(CppTypeData {
                kind: CppTypeDataKind::Enum,
                path: enum_name.clone(),
                doc: None,
                is_movable: false,
            }),
        );
        for child in entity.get_children() {
            if child.get_kind() == EntityKind::EnumConstantDecl {
                let val = child
                    .get_enum_constant_value()
                    .ok_or_else(|| err_msg("failed to get value of enum variant"))?;

                self.data.current_database.add_cpp_data(
                    DatabaseItemSource::CppParser {
                        include_file: include_file.clone(),
                        origin_location: get_origin_location(child)?,
                    },
                    CppItemData::EnumValue(CppEnumValue {
                        name: child
                            .get_name()
                            .ok_or_else(|| err_msg("failed to get name of enum variant"))?,
                        value: val.1,
                        enum_path: enum_name.clone(),
                        doc: None,
                    }),
                );
            }
        }
        Ok(())
    }

    /// Parses a class field `entity`.
    fn parse_class_field(&mut self, entity: Entity, class_type: CppPath) -> Result<()> {
        let include_file = self.entity_include_file(entity).with_context(|_| {
            format!(
                "Origin of class field is unknown: {}; entity: {:?}",
                // TODO: add function for this
                get_full_name_display(entity),
                entity
            )
        })?;
        let field_name = entity
            .get_name()
            .ok_or_else(|| err_msg("failed to get field name"))?;
        let field_clang_type = entity
            .get_type()
            .ok_or_else(|| err_msg("failed to get field type"))?;
        let field_type = self
            .parse_type(field_clang_type, Some(entity), None)
            .with_context(|_| {
                format!(
                    "failed to parse field type: {}::{}",
                    entity_log_representation(entity),
                    field_name
                )
            })?;
        self.data.current_database.add_cpp_data(
            DatabaseItemSource::CppParser {
                include_file,
                origin_location: get_origin_location(entity)?,
            },
            CppItemData::ClassField(CppClassField {
                //        size: match field_clang_type.get_sizeof() {
                //          Ok(size) => Some(size),
                //          Err(_) => None,
                //        },
                name: field_name,
                field_type,
                class_type,
                visibility: match entity.get_accessibility().unwrap_or(Accessibility::Public) {
                    Accessibility::Public => CppVisibility::Public,
                    Accessibility::Protected => CppVisibility::Protected,
                    Accessibility::Private => CppVisibility::Private,
                },
                // TODO: determine `is_const` and `is_static` (switch to a newer clang?)
                is_const: false,
                is_static: false,
            }),
        );

        Ok(())
    }

    /// Parses a class or a struct `entity`.
    fn parse_class(&mut self, entity: Entity) -> Result<()> {
        let include_file = self.entity_include_file(entity).with_context(|_| {
            format!(
                "Origin of type is unknown: {}; entity: {:?}",
                get_full_name_display(entity),
                entity
            )
        })?;
        let full_name = get_full_name(entity)?;
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
                unexpected!(
                    "missing template arguments for {}",
                    entity_log_representation(entity)
                );
            }
        } else if template_arguments.is_some() {
            unexpected!("unexpected template arguments");
        }
        //    let size = match entity.get_type() {
        //      Some(type1) => type1.get_sizeof().ok(),
        //      None => None,
        //    };
        //    if template_arguments.is_none() && size.is_none() {
        //      bail!("Failed to request size, but the class is not a template class");
        //    }
        if let Some(parent) = entity.get_semantic_parent() {
            if get_template_arguments(parent).is_some() {
                bail!("Types nested into template types are not supported");
            }
        }
        let mut current_base_index = 0;
        for child in entity.get_children() {
            if child.get_kind() == EntityKind::FieldDecl {
                if let Err(err) = self.parse_class_field(child, full_name.clone()) {
                    self.data.html_logger.add(
                        &[
                            entity_log_representation(child),
                            format!("failed to parse class field: {}", err),
                        ],
                        "cpp_parser_error",
                    )?;
                }
            }
            if child.get_kind() == EntityKind::BaseSpecifier {
                let base_type = self
                    .parse_type(child.get_type().unwrap(), Some(entity), None)
                    .with_context(|_| "Can't parse base class type")?;
                if let CppType::Class(ref base_type) = base_type {
                    self.data.current_database.add_cpp_data(
                        DatabaseItemSource::CppParser {
                            include_file: include_file.clone(),
                            origin_location: get_origin_location(entity).unwrap(),
                        },
                        CppItemData::ClassBase(CppBaseSpecifier {
                            base_class_type: base_type.clone(),
                            is_virtual: child.is_virtual_base(),
                            visibility: match child
                                .get_accessibility()
                                .unwrap_or(Accessibility::Public)
                            {
                                Accessibility::Public => CppVisibility::Public,
                                Accessibility::Protected => CppVisibility::Protected,
                                Accessibility::Private => CppVisibility::Private,
                            },
                            base_index: current_base_index,
                            derived_class_type: full_name.clone(),
                        }),
                    );
                    current_base_index += 1;
                } else {
                    bail!("base type is not a class: {:?}", base_type);
                }
            }
            if child.get_kind() == EntityKind::NonTypeTemplateParameter {
                bail!("Non-type template parameter is not supported");
            }
        }
        self.data.current_database.add_cpp_data(
            DatabaseItemSource::CppParser {
                include_file,
                origin_location: get_origin_location(entity).unwrap(),
            },
            CppItemData::Type(CppTypeData {
                kind: CppTypeDataKind::Class,
                path: full_name,
                doc: None,
                is_movable: false,
            }),
        );
        Ok(())
    }

    /// Determines file path of the include file this `entity` is located in.
    fn entity_include_path(&self, entity: Entity) -> Result<String> {
        if let Some(location) = entity.get_location() {
            let file_path = location.get_presumed_location().0;
            if file_path.is_empty() {
                bail!("empty file path")
            } else {
                Ok(file_path)
            }
        } else {
            bail!("no location for entity")
        }
    }

    /// Determines file name of the include file this `entity` is located in.
    fn entity_include_file(&self, entity: Entity) -> Result<String> {
        let file_path_buf = PathBuf::from(self.entity_include_path(entity)?);
        let file_name = file_path_buf
            .file_name()
            .ok_or_else(|| err_msg("no file name in file path"))?;
        Ok(os_str_to_str(file_name)?.to_string())
    }

    /// Returns false if this `entity` was blacklisted in some way.
    fn should_process_entity(&self, entity: Entity) -> bool {
        if let Ok(full_name) = get_full_name(entity) {
            if let Ok(file_path) = self.entity_include_path(entity) {
                let file_path_buf = PathBuf::from(&file_path);
                if !self.data.config.target_include_paths().is_empty()
                    && !self
                        .data
                        .config
                        .target_include_paths()
                        .iter()
                        .any(|x| file_path_buf.starts_with(x))
                {
                    return false;
                }
            }
            if self
                .data
                .config
                .cpp_parser_blocked_names()
                .iter()
                .any(|x| x == &full_name)
            {
                return false;
            }
        }
        true
    }

    /// Parses type declarations in translation unit `entity`
    /// and saves them to `self`.
    fn parse_types(&mut self, entity: Entity) -> Result<()> {
        if !self.should_process_entity(entity) {
            return Ok(());
        }
        match entity.get_kind() {
            EntityKind::EnumDecl => {
                if entity.get_accessibility() == Some(Accessibility::Private) {
                    return Ok(()); // skipping private stuff
                }
                if entity.get_name().is_some() && entity.is_definition() {
                    if let Err(error) = self.parse_enum(entity) {
                        self.data.html_logger.add(
                            &[
                                entity_log_representation(entity),
                                format!("failed to parse enum: {}", error),
                            ],
                            "cpp_parser_error",
                        )?;
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
                        self.data.html_logger.add(
                            &[
                                entity_log_representation(entity),
                                format!("failed to parse class: {}", error),
                            ],
                            "cpp_parser_error",
                        )?;
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
                    self.parse_types(c)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Parses methods in translation unit `entity`.
    fn parse_functions(&mut self, entity: Entity) -> Result<()> {
        if !self.should_process_entity(entity) {
            return Ok(());
        }
        match entity.get_kind() {
            EntityKind::FunctionDecl
            | EntityKind::Method
            | EntityKind::Constructor
            | EntityKind::Destructor
            | EntityKind::ConversionFunction
            | EntityKind::FunctionTemplate => {
                if entity.get_canonical_entity() == entity {
                    match self.parse_function(entity) {
                        Ok((r, info)) => {
                            self.data
                                .current_database
                                .add_cpp_data(info, CppItemData::Function(r));
                        }
                        Err(error) => {
                            self.data.html_logger.add(
                                &[
                                    entity_log_representation(entity),
                                    format!("failed to parse class: {}", error),
                                ],
                                "cpp_parser_error",
                            )?;
                        }
                    }
                }
            }
            EntityKind::StructDecl
            | EntityKind::ClassDecl
            | EntityKind::ClassTemplate
            | EntityKind::ClassTemplatePartialSpecialization => {
                if let Some(name) = entity.get_display_name() {
                    if let Ok(parent_type) =
                        self.parse_unexposed_type(None, Some(name.clone()), None, None)
                    {
                        if let CppType::Class(path) = parent_type {
                            if let Some(ref template_arguments) = path.last().template_arguments {
                                if template_arguments
                                    .iter()
                                    .any(|x| !x.is_template_parameter())
                                {
                                    self.data.html_logger.add(
                                        &[
                                            entity_log_representation(entity),
                                            "skipping template partial specialization".into(),
                                        ],
                                        "cpp_parser_skip",
                                    )?;
                                    return Ok(());
                                }
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
