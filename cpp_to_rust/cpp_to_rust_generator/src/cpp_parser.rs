use cpp_data::{CppBaseSpecifier, CppClassField, CppClassUsingDirective, CppData, CppEnumValue,
               CppOriginLocation, CppTypeData, CppTypeKind, CppVisibility, ParserCppData,
               TemplateArgumentsDeclaration};
use cpp_method::{CppMethod, CppMethodArgument, CppMethodClassMembership, CppMethodKind};
use cpp_operator::CppOperator;
use cpp_type::{CppBuiltInNumericType, CppFunctionPointerType, CppSpecificNumericType,
               CppSpecificNumericTypeKind, CppType, CppTypeBase, CppTypeClassBase,
               CppTypeIndirection};
use common::errors::{unexpected, ChainErr, Result};
use common::file_utils::{create_file, open_file, os_str_to_str, path_to_str, remove_file};
use common::string_utils::JoinWithSeparator;
use common::log;
use new_impl::database::{CppItemData, Database};

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use common::target::current_target;
use clang::*;
use clang;
use common::file_utils::PathBufWithAdded;

use regex::Regex;
use std::iter::once;
use new_impl::database::DataEnvInfo;
use new_impl::database::DataEnv;
use new_impl::database::DataSource;
use new_impl::database::DatabaseItem;
use new_impl::html_logger::HtmlLogger;
use new_impl::database::DatabaseUpdateResultType;
use config::Config;
use new_impl::processor::ProcessorData;

fn entity_log_representation(entity: Entity) -> String {
  format!(
    "{}; {:?}",
    get_full_name(entity).unwrap_or("?".into()),
    entity
  )
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
struct CppParser<'a> {
  data: ProcessorData<'a>,
}

/// Print representation of `entity` and its children to the log.
/// `level` is current level of recursion.
#[allow(dead_code)]
fn dump_entity(entity: Entity, level: usize) {
  log::llog(log::DebugParser, || {
    format!(
      "{}{:?}",
      (0..level).into_iter().map(|_| ". ").join(""),
      entity
    )
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
    None => Err("No info about location.".into()),
  }
}

/// Extract template argument declarations from a class or method definition `entity`.
fn get_template_arguments(entity: Entity) -> Option<Vec<CppType>> {
  let mut nested_level = 0;
  if let Some(parent) = entity.get_semantic_parent() {
    if let Some(args) = get_template_arguments(parent) {
      let parent_nested_level =
        if let CppTypeBase::TemplateParameter { nested_level, .. } = args[0].base {
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
    .map(|(i, c)| CppType {
      base: CppTypeBase::TemplateParameter {
        name: c.get_name().unwrap_or_else(|| format!("Type{}", i + 1)),
        index: i,
        nested_level: nested_level,
      },
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
    })
    .collect();
  if args.is_empty() {
    None
  } else {
    Some(args)
  }
}

/// Returns fully qualified name of `entity`.
fn get_full_name(entity: Entity) -> Result<String> {
  let mut current_entity = entity;
  if let Some(mut s) = entity.get_name() {
    while let Some(p) = current_entity.get_semantic_parent() {
      match p.get_kind() {
        EntityKind::ClassDecl
        | EntityKind::ClassTemplate
        | EntityKind::StructDecl
        | EntityKind::Namespace
        | EntityKind::EnumDecl
        | EntityKind::ClassTemplatePartialSpecialization => {
          match p.get_name() {
            Some(p_name) => s = format!("{}::{}", p_name, s),
            None => return Err("Anonymous nested type".into()),
          }
          current_entity = p;
        }
        EntityKind::Method => {
          return Err("Type nested in a method".into());
        }
        _ => break,
      }
    }
    Ok(s)
  } else {
    Err("Anonymous type".into())
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
  Clang::new().map_err(|err| format!("clang init failed: {}", err).into())
}

#[cfg(not(test))]
/// Creates a `Clang` context.
fn init_clang() -> Result<Clang> {
  Clang::new().map_err(|err| format!("clang init failed: {}", err).into())
}

/// Runs `clang` parser with `config`.
/// If `cpp_code` is specified, it's written to the C++ file before parsing it.
/// If successful, calls `f` and passes the topmost entity (the translation unit)
/// as its argument. Returns output value of `f` or an error.
#[cfg_attr(feature = "clippy", allow(block_in_if_condition_stmt))]
fn run_clang<R, F: FnMut(Entity) -> Result<R>>(
  config: &Config,
  tmp_path: &Path,
  cpp_code: Option<String>,
  mut f: F,
) -> Result<R> {
  let clang = init_clang()?;
  let index = Index::new(&clang, false, false);
  let tmp_cpp_path = tmp_path.with_added("1.cpp");
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
    .map_err(|err| format!("clang parse failed: {:?}", err))?;
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
      return Err(
        format!(
          "fatal clang error:\n{}",
          diagnostics.iter().map(|d| d.to_string()).join("\n")
        ).into(),
      );
    }
  }
  let result = f(translation_unit);
  remove_file(&tmp_cpp_path)?;
  result
}

/// Runs the parser on specified data.
pub fn run(data: ProcessorData) -> Result<()> {
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
      parser.parse_types(translation_unit);
      log::status("Parsing methods");
      //let methods =
      parser.parse_methods(translation_unit)?;
      //Ok((parser, methods))
      Ok(())
    },
  )?;
  Ok(())

  //  log::status("Checking data integrity");
  //  let (good_methods, good_types) = parser.check_integrity(methods);
  //  parser.types = good_types;
  //  log::status("Searching for template instantiations");
  //  Ok(ParserCppData {
  //    types: parser.types,
  //    methods: good_methods,
  //  })
}

impl<'a> CppParser<'a> {
  /// Search for a C++ type information in the types found by the parser
  /// and in types of the dependencies.
  fn find_type<F: Fn(&CppTypeData) -> bool>(&self, f: F) -> Option<&CppTypeData> {
    let databases = once(self.data.current_database as &_).chain(self.data.dep_databases.iter());
    for database in databases {
      for item in database.items() {
        if let CppItemData::Type(ref info) = item.cpp_data {
          if f(info) {
            return Some(info);
          }
        }
      }
    }
    return None;
  }

  /// Attempts to parse an unexposed type, i.e. a type the used `clang` API
  /// is not able to describe. Either `type1` or `string` must be specified,
  /// and both may be specified at the same time.
  /// Surrounding class and/or
  /// method may be specified in `context_class` and `context_method`.
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
        return Err(unexpected(format!("const inconsistency: {}, {:?}", is_const, type1)).into());
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
            .unwrap_or(Accessibility::Public) != Accessibility::Public
          {
            return Err(
              format!(
                "Type uses private class ({})",
                get_full_name(declaration).unwrap_or("?".into())
              ).into(),
            );
          }
          if let Some(matches) = template_class_regex.captures(name.as_ref()) {
            let mut arg_types = Vec::new();
            if let Some(items) = matches.at(2) {
              for arg in items.split(',') {
                match self.parse_unexposed_type(
                  None,
                  Some(arg.trim().to_string()),
                  context_class,
                  context_method,
                ) {
                  Ok(arg_type) => arg_types.push(arg_type),
                  Err(msg) => {
                    return Err(
                      format!(
                        "Template argument of unexposed type is not parsed: {}: {}",
                        arg, msg
                      ).into(),
                    )
                  }
                }
              }
            } else {
              return Err(unexpected("invalid matches count in regexp").into());
            }
            return Ok(CppType {
              base: CppTypeBase::Class(CppTypeClassBase {
                name: get_full_name(declaration)?,
                template_arguments: Some(arg_types),
              }),
              is_const: is_const,
              is_const2: false,
              indirection: CppTypeIndirection::None,
            });
          } else {
            return Err(format!("Can't parse declaration of an unexposed type: {}", name).into());
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
      return Err("parse_unexposed_type: either type or string must be present".into());
    };
    let re = Regex::new(r"^type-parameter-(\d+)-(\d+)$")?;
    if let Some(matches) = re.captures(name.as_ref()) {
      if matches.len() < 3 {
        return Err("invalid matches len in regexp".into());
      }
      return Ok(CppType {
        base: CppTypeBase::TemplateParameter {
          nested_level: matches[1]
            .parse()
            .chain_err(|| "encountered not a number while parsing type-parameter-X-X")?,
          index: matches[2]
            .parse()
            .chain_err(|| "encountered not a number while parsing type-parameter-X-X")?,
          name: name.clone(),
        },
        is_const: is_const,
        is_const2: false,
        indirection: CppTypeIndirection::None,
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
    let mut remaining_name: &str = name.as_ref();
    let mut result_type = CppType {
      is_const: is_const,
      is_const2: false,
      indirection: CppTypeIndirection::None,
      base: CppTypeBase::Void,
    };
    if remaining_name.ends_with(" *") {
      result_type.indirection = CppTypeIndirection::Ptr;
      remaining_name = remaining_name[0..remaining_name.len() - " *".len()].trim();
    }
    if remaining_name.ends_with(" &") {
      result_type.indirection = CppTypeIndirection::Ref;
      remaining_name = remaining_name[0..remaining_name.len() - " &".len()].trim();
    }
    if remaining_name == "void" {
      return Ok(result_type);
    }
    if let Some(x) = CppBuiltInNumericType::all()
      .iter()
      .find(|x| x.to_cpp_code() == remaining_name)
    {
      result_type.base = CppTypeBase::BuiltInNumeric(x.clone());
      return Ok(result_type);
    }
    if let Some(base) = self.parse_special_typedef(remaining_name) {
      result_type.base = base;
      return Ok(result_type);
    }
    if result_type.indirection == CppTypeIndirection::Ptr
      || result_type.indirection == CppTypeIndirection::Ref
    {
      if let Ok(subtype) = self.parse_unexposed_type(
        None,
        Some(remaining_name.to_string()),
        context_class,
        context_method,
      ) {
        let mut new_indirection =
          CppTypeIndirection::combine(&subtype.indirection, &result_type.indirection)
            .map_err(|e| e.to_string())?;
        if new_indirection == CppTypeIndirection::Ptr {
          if let CppTypeBase::FunctionPointer(..) = subtype.base {
            new_indirection = CppTypeIndirection::None;
          }
        }
        let new_is_const2 = if new_indirection == CppTypeIndirection::PtrPtr {
          remaining_name.trim().ends_with(" const")
        } else {
          false
        };
        return Ok(CppType {
          base: subtype.base,
          is_const: subtype.is_const,
          is_const2: new_is_const2,
          indirection: new_indirection,
        });
      }
    }
    if let Some(type_data) = self.find_type(|x| &x.name == remaining_name) {
      match type_data.kind {
        CppTypeKind::Enum { .. } => {
          result_type.base = CppTypeBase::Enum {
            name: remaining_name.to_string(),
          }
        }
        CppTypeKind::Class { .. } => {
          result_type.base = CppTypeBase::Class(CppTypeClassBase {
            name: remaining_name.to_string(),
            template_arguments: None,
          })
        }
      }
      return Ok(result_type);
    }

    if let Some(matches) = template_class_regex.captures(remaining_name) {
      if matches.len() < 3 {
        return Err("invalid matches len in regexp".into());
      }
      let class_name = &matches[1];
      if self
        .find_type(|x| &x.name == class_name && x.is_class())
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
              return Err(
                format!(
                  "Template argument of unexposed type is not parsed: {}: {}",
                  arg, msg
                ).into(),
              )
            }
          }
        }
        result_type.base = CppTypeBase::Class(CppTypeClassBase {
          name: class_name.to_string(),
          template_arguments: Some(arg_types),
        });
        return Ok(result_type);
      }
    } else {
      return Err(format!("Can't parse declaration of an unexposed type: {}", name).into());
    }

    Err(format!("Unrecognized unexposed type: {}", name).into())
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
    if type1.is_volatile_qualified() {
      return Err("Volatile type".into());
    }
    let display_name = type1.get_display_name();
    if &display_name == "std::list<T>" {
      return Err(
        format!(
          "Type blacklisted because it causes crash on Windows: {}",
          display_name
        ).into(),
      );
    }
    let is_const = type1.is_const_qualified();
    match type1.get_kind() {
      TypeKind::Typedef => {
        let parsed = self.parse_type(type1.get_canonical_type(), context_class, context_method)?;
        if let CppTypeBase::BuiltInNumeric(..) = parsed.base {
          if parsed.indirection == CppTypeIndirection::None {
            let mut name = type1.get_display_name();
            if name.starts_with("const ") {
              name = name[6..].trim().to_string();
            }
            if let Some(r) = self.parse_special_typedef(&name) {
              return Ok(CppType {
                base: r,
                indirection: parsed.indirection,
                is_const: parsed.is_const,
                is_const2: parsed.is_const2,
              });
            }
          }
        }
        Ok(parsed)
      }
      TypeKind::Void => Ok(CppType {
        base: CppTypeBase::Void,
        is_const: is_const,
        is_const2: false,
        indirection: CppTypeIndirection::None,
      }),
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
      | TypeKind::LongDouble => Ok(CppType {
        base: CppTypeBase::BuiltInNumeric(convert_type_kind(type1.get_kind())),
        is_const: is_const,
        is_const2: false,
        indirection: CppTypeIndirection::None,
      }),
      TypeKind::Enum => {
        if let Some(declaration) = type1.get_declaration() {
          Ok(CppType {
            base: CppTypeBase::Enum {
              name: get_full_name(declaration)?,
            },
            is_const: is_const,
            is_const2: false,
            indirection: CppTypeIndirection::None,
          })
        } else {
          return Err(format!("failed to get enum declaration: {:?}", type1).into());
        }
      }
      TypeKind::Elaborated | TypeKind::Record => {
        if let Some(declaration) = type1.get_declaration() {
          if declaration
            .get_accessibility()
            .unwrap_or(Accessibility::Public) != Accessibility::Public
          {
            return Err(
              format!(
                "Type uses private class ({})",
                get_full_name(declaration).unwrap_or("unnamed".to_string())
              ).into(),
            );
          }
          let declaration_name = get_full_name(declaration)?;
          let template_arguments = match type1.get_template_argument_types() {
            None => None,
            Some(arg_types) => {
              let mut r = Vec::new();
              if arg_types.is_empty() {
                return Err(unexpected("arg_types is empty").into());
              }
              for arg_type in arg_types {
                match arg_type {
                  None => return Err("Template argument is None".into()),
                  Some(arg_type) => {
                    match self.parse_type(arg_type, context_class, context_method) {
                      Ok(parsed_type) => r.push(parsed_type),
                      Err(msg) => {
                        return Err(
                          format!("Invalid template argument: {:?}: {}", arg_type, msg).into(),
                        )
                      }
                    }
                  }
                }
              }
              Some(r)
            }
          };

          Ok(CppType {
            base: CppTypeBase::Class(CppTypeClassBase {
              name: declaration_name,
              template_arguments: template_arguments,
            }),
            is_const: is_const,
            is_const2: false,
            indirection: CppTypeIndirection::None,
          })
        } else {
          return Err(format!("failed to get class declaration: {:?}", type1).into());
        }
      }
      TypeKind::FunctionPrototype => {
        let mut arguments = Vec::new();
        if let Some(argument_types) = type1.get_argument_types() {
          for arg_type in argument_types {
            match self.parse_type(arg_type, context_class, context_method) {
              Ok(t) => arguments.push(t),
              Err(msg) => {
                return Err(
                  format!(
                    "Failed to parse function type's argument type: {:?}: {}",
                    arg_type, msg
                  ).into(),
                )
              }
            }
          }
        } else {
          return Err(
            format!(
              "Failed to parse get argument types from function type: {:?}",
              type1
            ).into(),
          );
        }
        let return_type = if let Some(result_type) = type1.get_result_type() {
          match self.parse_type(result_type, context_class, context_method) {
            Ok(t) => Box::new(t),
            Err(msg) => {
              return Err(
                format!(
                  "Failed to parse function type's argument type: {:?}: {}",
                  result_type, msg
                ).into(),
              )
            }
          }
        } else {
          return Err(
            format!(
              "Failed to parse get result type from function type: {:?}",
              type1
            ).into(),
          );
        };
        Ok(CppType {
          base: CppTypeBase::FunctionPointer(CppFunctionPointerType {
            return_type: return_type,
            arguments: arguments,
            allows_variadic_arguments: type1.is_variadic(),
          }),
          is_const: is_const,
          is_const2: false,
          indirection: CppTypeIndirection::None,
        })
      }
      TypeKind::Pointer | TypeKind::LValueReference | TypeKind::RValueReference => {
        match type1.get_pointee_type() {
          Some(pointee) => match self.parse_type(pointee, context_class, context_method) {
            Ok(subtype) => {
              let original_type_indirection = match type1.get_kind() {
                TypeKind::Pointer => CppTypeIndirection::Ptr,
                TypeKind::LValueReference => CppTypeIndirection::Ref,
                TypeKind::RValueReference => CppTypeIndirection::RValueRef,
                _ => unreachable!(),
              };

              let mut new_indirection =
                CppTypeIndirection::combine(&subtype.indirection, &original_type_indirection)
                  .map_err(|e| e.to_string())?;
              if new_indirection == CppTypeIndirection::Ptr {
                if let CppTypeBase::FunctionPointer(..) = subtype.base {
                  new_indirection = CppTypeIndirection::None;
                }
              }
              let new_is_const2 = if new_indirection == CppTypeIndirection::PtrPtr {
                pointee.is_const_qualified()
              } else {
                false
              };
              Ok(CppType {
                indirection: new_indirection,
                base: subtype.base,
                is_const: subtype.is_const,
                is_const2: new_is_const2,
              })
            }
            Err(msg) => Err(msg),
          },
          None => Err("can't get pointee type".into()),
        }
      }
      TypeKind::Unexposed => {
        let canonical = type1.get_canonical_type();
        if canonical.get_kind() != TypeKind::Unexposed {
          let mut parsed_canonical = self.parse_type(canonical, context_class, context_method);
          if let Ok(parsed_unexposed) =
            self.parse_unexposed_type(Some(type1), None, context_class, context_method)
          {
            if let CppTypeBase::Class(CppTypeClassBase {
              ref template_arguments,
              ..
            }) = parsed_unexposed.base
            {
              if let Some(ref template_arguments) = *template_arguments {
                let template_arguments_unexposed = template_arguments;
                if template_arguments_unexposed.iter().any(|x| match x.base {
                  CppTypeBase::SpecificNumeric { .. } | CppTypeBase::PointerSizedInteger { .. } => {
                    true
                  }
                  _ => false,
                }) {
                  if let Ok(ref mut parsed_canonical) = parsed_canonical {
                    if let CppTypeBase::Class(CppTypeClassBase {
                      ref mut template_arguments,
                      ..
                    }) = parsed_canonical.base
                    {
                      if let Some(ref mut template_arguments) = *template_arguments {
                        template_arguments.clone_from(template_arguments_unexposed);
                      }
                    }
                  }
                }
              }
            }
          }
          parsed_canonical
        } else {
          self.parse_unexposed_type(Some(type1), None, context_class, context_method)
        }
      }
      _ => Err(format!("Unsupported kind of type: {:?}", type1.get_kind()).into()),
    }
  }

  /// Checks if the typedef `name` has a special meaning for the parser.
  fn parse_special_typedef(&self, name: &str) -> Option<CppTypeBase> {
    match name {
      "qint8" | "int8_t" | "GLbyte" => Some(CppTypeBase::SpecificNumeric(CppSpecificNumericType {
        name: name.to_string(),
        bits: 8,
        kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
      })),
      "quint8" | "uint8_t" | "GLubyte" => {
        Some(CppTypeBase::SpecificNumeric(CppSpecificNumericType {
          name: name.to_string(),
          bits: 8,
          kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
        }))
      }
      "qint16" | "int16_t" | "GLshort" => {
        Some(CppTypeBase::SpecificNumeric(CppSpecificNumericType {
          name: name.to_string(),
          bits: 16,
          kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
        }))
      }
      "quint16" | "uint16_t" | "GLushort" => {
        Some(CppTypeBase::SpecificNumeric(CppSpecificNumericType {
          name: name.to_string(),
          bits: 16,
          kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
        }))
      }
      "qint32" | "int32_t" | "GLint" => {
        Some(CppTypeBase::SpecificNumeric(CppSpecificNumericType {
          name: name.to_string(),
          bits: 32,
          kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
        }))
      }
      "quint32" | "uint32_t" | "GLuint" => {
        Some(CppTypeBase::SpecificNumeric(CppSpecificNumericType {
          name: name.to_string(),
          bits: 32,
          kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
        }))
      }
      "qint64" | "int64_t" | "qlonglong" | "GLint64" => {
        Some(CppTypeBase::SpecificNumeric(CppSpecificNumericType {
          name: name.to_string(),
          bits: 64,
          kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
        }))
      }
      "quint64" | "uint64_t" | "qulonglong" | "GLuint64" => {
        Some(CppTypeBase::SpecificNumeric(CppSpecificNumericType {
          name: name.to_string(),
          bits: 64,
          kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
        }))
      }
      "qintptr" | "qptrdiff" | "QList::difference_type" => Some(CppTypeBase::PointerSizedInteger {
        name: name.to_string(),
        is_signed: true,
      }),
      "quintptr" => Some(CppTypeBase::PointerSizedInteger {
        name: name.to_string(),
        is_signed: false,
      }),
      _ => None,
    }
  }

  /// Parses a function `entity`.
  #[cfg_attr(feature = "clippy", allow(cyclomatic_complexity))]
  fn parse_function(&self, entity: Entity) -> Result<(CppMethod, DataEnvInfo)> {
    let (class_name, class_entity) = match entity.get_semantic_parent() {
      Some(p) => match p.get_kind() {
        EntityKind::ClassDecl | EntityKind::ClassTemplate | EntityKind::StructDecl => {
          match get_full_name(p) {
            Ok(class_name) => (Some(class_name), Some(p)),
            Err(msg) => {
              return Err(
                format!(
                  "function parent is a class but it doesn't have a name: {}",
                  msg
                ).into(),
              );
            }
          }
        }
        EntityKind::ClassTemplatePartialSpecialization => {
          return Err("this function is part of a template partial specialization".into());
        }
        _ => (None, None),
      },
      None => (None, None),
    };

    let return_type = if let Some(x) = entity.get_type() {
      if let Some(y) = x.get_result_type() {
        y
      } else {
        return Err(format!("failed to get function return type: {:?}", entity).into());
      }
    } else {
      return Err(format!("failed to get function type: {:?}", entity).into());
    };
    let return_type_parsed = match self.parse_type(return_type, class_entity, Some(entity)) {
      Ok(x) => x,
      Err(msg) => {
        return Err(
          format!(
            "Can't parse return type: {}: {}",
            return_type.get_display_name(),
            msg
          ).into(),
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
      return Err(format!("failed to get function arguments: {:?}", entity).into());
    };
    let template_arguments = match entity.get_kind() {
      EntityKind::FunctionTemplate => {
        if entity
          .get_children()
          .into_iter()
          .any(|c| c.get_kind() == EntityKind::NonTypeTemplateParameter)
        {
          return Err("Non-type template parameter is not supported".into());
        }
        get_template_arguments(entity)
      }
      _ => None,
    };
    let mut is_signal = false;
    for (argument_number, argument_entity) in argument_entities.into_iter().enumerate() {
      let name = argument_entity
        .get_name()
        .unwrap_or_else(|| format!("arg{}", argument_number + 1));
      let clang_type = argument_entity.get_type().chain_err(|| {
        format!(
          "failed to get type from argument entity: {:?}",
          argument_entity
        )
      })?;
      if clang_type.get_display_name().ends_with("::QPrivateSignal") {
        is_signal = true;
        continue;
      }
      let argument_type = self
        .parse_type(clang_type, class_entity, Some(entity))
        .chain_err(|| {
          format!(
            "Can't parse argument type: {}: {}",
            name,
            clang_type.get_display_name()
          )
        })?;
      let mut has_default_value = false;
      for token in argument_entity
        .get_range()
        .chain_err(|| {
          format!(
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
      arguments.push(CppMethodArgument {
        name: name,
        argument_type: argument_type,
        has_default_value: has_default_value,
      });
    }
    let mut name = entity
      .get_name()
      .chain_err(|| "failed to get function name")?;
    if name.contains('<') {
      let regex = Regex::new(r"^([\w~]+)<[^<>]+>$")?;
      if let Some(matches) = regex.captures(name.clone().as_ref()) {
        log::llog(log::DebugParser, || {
          format!("Fixing malformed method name: {}", name)
        });
        name = matches
          .at(1)
          .chain_err(|| "invalid matches count")?
          .to_string();
      }
    }
    let mut name_with_namespace = name.clone();
    if let Some(parent) = entity.get_semantic_parent() {
      if parent.get_kind() == EntityKind::Namespace {
        name_with_namespace = format!(
          "{}::{}",
          get_full_name(parent).chain_err(|| "failed to get full name of parent entity")?,
          name
        );
      }
    }
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
            if info.allows_variadic_arguments || info.arguments_count == real_arguments_count {
              method_operator = Some(operator.clone());
              break;
            }
          }
        }
      }
      if method_operator.is_none() && name_matches {
        return Err(
          "This method is recognized as operator but arguments do not match \
           its signature."
            .into(),
        );
      }
    }

    if method_operator.is_none() && name.starts_with("operator ") {
      let op = name["operator ".len()..].trim();
      match self.parse_unexposed_type(None, Some(op.to_string()), class_entity, Some(entity)) {
        Ok(t) => method_operator = Some(CppOperator::Conversion(t)),
        Err(_) => return Err(format!("Unknown type in conversion operator: '{}'", op).into()),
      }
    }
    let source_range = entity
      .get_range()
      .chain_err(|| "failed to get range of the function")?;
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
        .chain_err(|| "no file in source location")?
        .get_path();
      let file = open_file(&file_path)?;
      let reader = BufReader::new(file.into_file());
      let mut result = String::new();
      let range_line1 = (start.line - 1) as usize;
      let range_line2 = (end.line - 1) as usize;
      let range_col1 = (start.column - 1) as usize;
      let range_col2 = (end.column - 1) as usize;
      for (line_num, line) in reader.lines().enumerate() {
        let line =
          line.chain_err(|| format!("failed while reading lines from {}", file_path.display()))?;
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
        return Err("Probably a volatile method.".into());
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
          return Err("A volatile method.".into());
        }
        token_strings.push(text);
      }
      Some(token_strings.join(" "))
    };
    Ok((
      CppMethod {
        name: name_with_namespace,
        operator: method_operator,
        class_membership: match class_name {
          Some(class_name) => {
            Some(CppMethodClassMembership {
              kind: match entity.get_kind() {
                EntityKind::Constructor => CppMethodKind::Constructor,
                EntityKind::Destructor => CppMethodKind::Destructor,
                _ => CppMethodKind::Regular,
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
              is_signal: is_signal,
              is_slot: false,
              class_type: CppTypeClassBase {
                name: class_name.to_string(),
                template_arguments: get_template_arguments(class_entity.unwrap()),
              },
            })
          }
          None => None,
        },
        arguments: arguments,
        allows_variadic_arguments: allows_variadic_arguments,
        return_type: return_type_parsed,
        template_arguments: template_arguments,
        declaration_code: declaration_code,
      },
      DataEnvInfo {
        is_success: true,
        include_file: Some(self.entity_include_file(entity)?),
        origin_location: Some(get_origin_location(entity)?),
        error: None,
        is_invalidated: false,
      },
    ))
  }

  /// Parses an enum `entity`.
  fn parse_enum(&mut self, entity: Entity) -> Result<()> {
    let include_file = self.entity_include_file(entity).chain_err(|| {
      format!(
        "Origin of type is unknown: {}; entity: {:?}",
        get_full_name(entity).unwrap_or("?".into()),
        entity
      )
    })?;
    let enum_name = get_full_name(entity)?;
    self.data.add_cpp_data(
      &CppItemData::Type(CppTypeData {
        name: enum_name.clone(),
        kind: CppTypeKind::Enum,
      }),
      DataEnvInfo {
        is_success: true,
        include_file: Some(include_file.clone()),
        origin_location: Some(get_origin_location(entity)?),
        error: None,
        is_invalidated: false,
      },
    );
    for child in entity.get_children() {
      if child.get_kind() == EntityKind::EnumConstantDecl {
        let val = child
          .get_enum_constant_value()
          .chain_err(|| "failed to get value of enum variant")?;

        self.data.add_cpp_data(
          &CppItemData::EnumValue(CppEnumValue {
            name: child
              .get_name()
              .chain_err(|| "failed to get name of enum variant")?,
            value: val.1,
            enum_name: enum_name.clone(),
          }),
          DataEnvInfo {
            is_success: true,
            include_file: Some(include_file.clone()),
            origin_location: Some(get_origin_location(child)?),
            error: None,
            is_invalidated: false,
          },
        );
      }
    }
    Ok(())
  }

  /// Parses a class field `entity`.
  fn parse_class_field(&mut self, entity: Entity, class_type: CppTypeClassBase) -> Result<()> {
    let include_file = self.entity_include_file(entity).chain_err(|| {
      format!(
        "Origin of class field is unknown: {}; entity: {:?}",
        get_full_name(entity).unwrap_or("?".into()),
        entity
      )
    })?;
    let field_name = entity.get_name().chain_err(|| "failed to get field name")?;
    let field_clang_type = entity.get_type().chain_err(|| "failed to get field type")?;
    let field_type = self
      .parse_type(field_clang_type, Some(entity), None)
      .chain_err(|| {
        format!(
          "failed to parse field type: {}::{}",
          entity_log_representation(entity),
          field_name
        )
      })?;
    self.data.add_cpp_data(
      &CppItemData::ClassField(CppClassField {
        //        size: match field_clang_type.get_sizeof() {
        //          Ok(size) => Some(size),
        //          Err(_) => None,
        //        },
        name: field_name,
        field_type: field_type,
        class_type: class_type,
        visibility: match entity.get_accessibility().unwrap_or(Accessibility::Public) {
          Accessibility::Public => CppVisibility::Public,
          Accessibility::Protected => CppVisibility::Protected,
          Accessibility::Private => CppVisibility::Private,
        },
      }),
      DataEnvInfo {
        is_success: true,
        include_file: Some(include_file),
        origin_location: Some(get_origin_location(entity)?),
        error: None,
        is_invalidated: false,
      },
    );

    Ok(())
  }

  /// Parses a class or a struct `entity`.
  fn parse_class(&mut self, entity: Entity) -> Result<()> {
    let include_file = self.entity_include_file(entity).chain_err(|| {
      format!(
        "Origin of type is unknown: {}; entity: {:?}",
        get_full_name(entity).unwrap_or("?".into()),
        entity
      )
    })?;
    let full_name = get_full_name(entity)?;
    /*
    let using_directives = entity
      .get_children()
      .into_iter()
      .filter(|c| c.get_kind() == EntityKind::UsingDeclaration)
      .filter_map(|child| {
        let type_ref =
          if let Some(x) = child.get_children().into_iter().find(|c| {
            c.get_kind() == EntityKind::TypeRef || c.get_kind() == EntityKind::TemplateRef
          }) {
            x
          } else {
            log::llog(log::DebugParser, || {
              "Failed to parse UsingDeclaration: class type not found".to_string()
            });
            dump_entity(child, 0);
            return None;
          };
        let type_def = type_ref
          .get_definition()
          .expect("TypeRef definition not found");
        Some(CppClassUsingDirective {
          class_name: get_full_name(type_def).expect("class_name get_full_name failed"),
          method_name: child.get_name().expect("method_name failed"),
        })
      })
      .collect(); */
    let template_arguments = get_template_arguments(entity);
    if entity.get_kind() == EntityKind::ClassTemplate {
      if entity
        .get_children()
        .into_iter()
        .any(|c| c.get_kind() == EntityKind::NonTypeTemplateParameter)
      {
        return Err("Non-type template parameter is not supported".into());
      }

      if template_arguments.is_none() {
        dump_entity(entity, 0);
        return Err(
          unexpected(format!(
            "missing template arguments for {}",
            entity_log_representation(entity)
          )).into(),
        );
      }
    } else if template_arguments.is_some() {
      return Err(unexpected("unexpected template arguments").into());
    }
    //    let size = match entity.get_type() {
    //      Some(type1) => type1.get_sizeof().ok(),
    //      None => None,
    //    };
    //    if template_arguments.is_none() && size.is_none() {
    //      return Err("Failed to request size, but the class is not a template class".into());
    //    }
    if let Some(parent) = entity.get_semantic_parent() {
      if get_template_arguments(parent).is_some() {
        return Err("Types nested into template types are not supported".into());
      }
    }
    let class_type_for_field = CppTypeClassBase {
      name: full_name.clone(),
      template_arguments: template_arguments.clone(),
    };
    let mut current_base_index = 0;
    for child in entity.get_children() {
      if child.get_kind() == EntityKind::FieldDecl {
        if let Err(err) = self.parse_class_field(child, class_type_for_field.clone()) {
          self.data.html_logger.add(
            &[
              entity_log_representation(child),
              format!("failed to parse class field: {}", err),
            ],
            "cpp_parser_error",
          );
          err.discard_expected();
        }
      }
      if child.get_kind() == EntityKind::BaseSpecifier {
        let base_type = match self.parse_type(child.get_type().unwrap(), Some(entity), None) {
          Ok(r) => r,
          Err(msg) => return Err(format!("Can't parse base class type: {}", msg).into()),
        };
        if base_type.indirection != CppTypeIndirection::None || base_type.is_const
          || base_type.is_const2
        {
          return Err(
            format!(
              "Invalid indirection or constness in base type: {:?}",
              base_type
            ).into(),
          );
        }
        if let CppTypeBase::Class(ref base_type) = base_type.base {
          self.data.add_cpp_data(
            &CppItemData::ClassBase(CppBaseSpecifier {
              base_class_type: base_type.clone(),
              is_virtual: child.is_virtual_base(),
              visibility: match child.get_accessibility().unwrap_or(Accessibility::Public) {
                Accessibility::Public => CppVisibility::Public,
                Accessibility::Protected => CppVisibility::Protected,
                Accessibility::Private => CppVisibility::Private,
              },
              base_index: current_base_index,
              derived_class_type: class_type_for_field.clone(),
            }),
            DataEnvInfo {
              is_success: true,
              error: None,
              include_file: Some(include_file.clone()),
              origin_location: Some(get_origin_location(entity).unwrap()),
              is_invalidated: false,
            },
          );
          current_base_index += 1;
        } else {
          return Err(format!("base type is not a class: {:?}", base_type).into());
        }
      }
      if child.get_kind() == EntityKind::NonTypeTemplateParameter {
        return Err("Non-type template parameter is not supported".into());
      }
    }
    self.data.add_cpp_data(
      &CppItemData::Type(CppTypeData {
        name: full_name,
        kind: CppTypeKind::Class {
          template_arguments: template_arguments,
        },
      }),
      DataEnvInfo {
        is_success: true,
        error: None,
        include_file: Some(include_file),
        origin_location: Some(get_origin_location(entity).unwrap()),
        is_invalidated: false,
      },
    );
    Ok(())
  }

  /// Determines file path of the include file this `entity` is located in.
  fn entity_include_path(&self, entity: Entity) -> Result<String> {
    if let Some(location) = entity.get_location() {
      let file_path = location.get_presumed_location().0;
      if file_path.is_empty() {
        Err("empty file path".into())
      } else {
        Ok(file_path)
      }
    } else {
      Err("no location for entity".into())
    }
  }

  /// Determines file name of the include file this `entity` is located in.
  fn entity_include_file(&self, entity: Entity) -> Result<String> {
    let file_path_buf = PathBuf::from(self.entity_include_path(entity)?);
    let file_name = file_path_buf
      .file_name()
      .chain_err(|| "no file name in file path")?;
    Ok(os_str_to_str(file_name)?.to_string())
  }

  /// Returns false if this `entity` was blacklisted in some way.
  fn should_process_entity(&self, entity: Entity) -> bool {
    if let Ok(full_name) = get_full_name(entity) {
      if full_name == "AllFields" {
        return true; //our special class
      }
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
    if let Some(name) = entity.get_name() {
      if self
        .data
        .config
        .cpp_parser_blocked_names()
        .iter()
        .any(|x| x == &name)
      {
        return false;
      }
    }
    true
  }

  /// Parses type declarations in translation unit `entity`
  /// and saves them to `self`.
  fn parse_types(&mut self, entity: Entity) {
    if !self.should_process_entity(entity) {
      return;
    }
    match entity.get_kind() {
      EntityKind::EnumDecl => {
        if entity.get_accessibility() == Some(Accessibility::Private) {
          return; // skipping private stuff
        }
        if entity.get_name().is_some() && entity.is_definition() {
          if let Err(error) = self.parse_enum(entity) {
            self.data.html_logger.add(
              &[
                entity_log_representation(entity),
                format!("failed to parse enum: {}", error),
              ],
              "cpp_parser_error",
            );
            error.discard_expected();
          }
        }
      }
      EntityKind::ClassDecl | EntityKind::ClassTemplate | EntityKind::StructDecl => {
        if entity.get_accessibility() == Some(Accessibility::Private) {
          return; // skipping private stuff
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
            );
            error.discard_expected();
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
      | EntityKind::ClassTemplate => for c in entity.get_children() {
        self.parse_types(c);
      },
      _ => {}
    }
  }

  /// Parses methods in translation unit `entity`.
  fn parse_methods(&mut self, entity: Entity) -> Result<()> {
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
              self.data.add_cpp_data(&CppItemData::Method(r), info);
            }
            Err(error) => {
              self.data.html_logger.add(
                &[
                  entity_log_representation(entity),
                  format!("failed to parse class: {}", error),
                ],
                "cpp_parser_error",
              )?;
              error.discard_expected();
            }
          }
        }
      }
      EntityKind::StructDecl
      | EntityKind::ClassDecl
      | EntityKind::ClassTemplate
      | EntityKind::ClassTemplatePartialSpecialization => {
        if let Some(name) = entity.get_display_name() {
          if let Ok(parent_type) = self.parse_unexposed_type(None, Some(name.clone()), None, None) {
            if let CppTypeBase::Class(CppTypeClassBase {
              ref template_arguments,
              ..
            }) = parent_type.base
            {
              if let Some(ref template_arguments) = *template_arguments {
                if template_arguments
                  .iter()
                  .any(|x| !x.base.is_template_parameter())
                {
                  self.data.html_logger.add(
                    &[
                      entity_log_representation(entity),
                      format!("skipping template partial specialization"),
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
      | EntityKind::ClassTemplate => for c in entity.get_children() {
        self.parse_methods(c)?;
      },
      _ => {}
    }
    Ok(())
  }

  /// Returns `Err` if `type1` or any of its components refer to
  /// an unknown type.
  fn check_type_integrity(&self, type1: &CppType) -> Result<()> {
    match type1.base {
      CppTypeBase::Void
      | CppTypeBase::BuiltInNumeric(..)
      | CppTypeBase::SpecificNumeric { .. }
      | CppTypeBase::PointerSizedInteger { .. }
      | CppTypeBase::TemplateParameter { .. } => {}
      CppTypeBase::Enum { ref name } => {
        if self.find_type(|x| &x.name == name).is_none() {
          return Err(format!("unknown type: {}", name).into());
        }
      }
      CppTypeBase::Class(CppTypeClassBase {
        ref name,
        ref template_arguments,
      }) => {
        if self.find_type(|x| &x.name == name).is_none() {
          return Err(format!("unknown type: {}", name).into());
        }
        if let Some(ref args) = *template_arguments {
          for arg in args {
            if let Err(msg) = self.check_type_integrity(arg) {
              return Err(msg);
            }
          }
        }
      }
      CppTypeBase::FunctionPointer(CppFunctionPointerType {
        ref return_type,
        ref arguments,
        ..
      }) => {
        if let Err(msg) = self.check_type_integrity(return_type) {
          return Err(msg);
        }
        for arg in arguments {
          if let Err(msg) = self.check_type_integrity(arg) {
            return Err(msg);
          }
        }
      }
    }
    Ok(())
  }

  /*
  /// Returns types and methods that don't refer to any unknown types.
  fn check_integrity(&self, methods: Vec<CppMethod>) -> (Vec<CppMethod>, Vec<CppTypeData>) {
    let good_methods = methods
      .into_iter()
      .filter(|method| {
        if let Err(msg) = self.check_type_integrity(&method.return_type.clone()) {
          log::llog(log::DebugParserSkips, || {
            format!("Method is removed: {}: {}", method.short_text(), msg)
          });
          return false;
        }
        for arg in &method.arguments {
          if let Err(msg) = self.check_type_integrity(&arg.argument_type) {
            log::llog(log::DebugParserSkips, || {
              format!("Method is removed: {}: {}", method.short_text(), msg)
            });
            return false;
          }
        }
        true
      })
      .collect();

    let mut good_types = Vec::new();
    for t in &self.types {
      let mut good_type = t.clone();
      if let CppTypeKind::Class {
        ref mut bases,
        ref mut fields,
        ..
      } = good_type.kind
      {
        let mut valid_bases = Vec::new();
        for base in bases.iter() {
          if let Err(msg) = self.check_type_integrity(&base.base_type) {
            log::llog(log::DebugParserSkips, || {
              format!(
                "Class {}: base class removed because type is not available: {:?}: {}",
                t.name, base, msg
              )
            });
          } else {
            valid_bases.push(base.clone());
          }
        }
        bases.clear();
        bases.append(&mut valid_bases);

        let mut valid_fields = Vec::new();
        for field in fields.iter() {
          if let Err(msg) = self.check_type_integrity(&field.field_type) {
            log::llog(log::DebugParserSkips, || {
              format!(
                "Class {}: field removed because type is not available: {:?}: {}",
                t.name, field, msg
              )
            });
          } else {
            valid_fields.push(field.clone());
          }
        }
        fields.clear();
        fields.append(&mut valid_fields);
      }
      good_types.push(good_type);
    }
    (good_methods, good_types)
  } */
}
