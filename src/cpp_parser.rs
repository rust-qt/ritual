extern crate clang;
use self::clang::*;

extern crate regex;
use self::regex::Regex;

use log;
use std::path::PathBuf;
use std::fs;
use std::fs::File;

use cpp_data::{CppData, CppTypeData, CppTypeKind, CppClassField, EnumValue, CppOriginLocation,
               CppVisibility, CppTemplateInstantiation, CppTemplateInstantiations,
               CppClassUsingDirective, CppBaseSpecifier, TemplateArgumentsDeclaration};
use cpp_method::{CppMethod, CppFunctionArgument, CppMethodKind, CppMethodClassMembership};
use cpp_type::{CppType, CppTypeBase, CppBuiltInNumericType, CppTypeIndirection,
               CppSpecificNumericTypeKind, CppTypeClassBase};
use cpp_operator::CppOperator;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::env;
use utils::is_msvc;

struct CppParser {
  config: CppParserConfig,
  types: Vec<CppTypeData>,
  dependency_types: Vec<CppTypeData>,
}

#[allow(dead_code)]
fn inspect_method(entity: Entity) {
  println!("{:?}", entity.get_display_name());
  println!("type: {:?}", entity.get_type());
  println!("return type: {:?}",
           entity.get_type().unwrap().get_result_type());
  println!("args:");
  for c in entity.get_arguments().unwrap() {
    println!("arg: name={} type={:?}",
             c.get_name().unwrap_or("[no name]".to_string()),
             c.get_type());
  }
}

#[allow(dead_code)]
fn dump_entity(entity: Entity, level: i32) {
  for _ in 0..level {
    print!(". ");
  }
  println!("{:?}", entity);
  if level <= 5 {
    for child in entity.get_children() {
      dump_entity(child, level + 1);
    }
  }
}

fn get_origin_location(entity: Entity) -> Result<CppOriginLocation, String> {
  match entity.get_location() {
    Some(loc) => {
      let location = loc.get_presumed_location();
      Ok(CppOriginLocation {
        include_file_path: location.0,
        line: location.1,
        column: location.2,
      })
    }
    None => Err("No info about location.".to_string()),
  }
}

fn get_template_arguments(entity: Entity) -> Option<TemplateArgumentsDeclaration> {
  let mut nested_level = 0;
  if let Some(parent) = entity.get_semantic_parent() {
    if let Some(args) = get_template_arguments(parent) {
      nested_level = args.nested_level + 1;
    }
  }
  let names: Vec<_> = entity.get_children()
    .into_iter()
    .filter(|c| c.get_kind() == EntityKind::TemplateTypeParameter)
    .enumerate()
    .map(|(i, c)| c.get_name().unwrap_or_else(|| format!("Type{}", i + 1)))
    .collect();
  if names.is_empty() {
    None
  } else {
    Some(TemplateArgumentsDeclaration {
      nested_level: nested_level,
      names: names,
    })
  }
}


fn get_full_name(entity: Entity) -> Result<String, String> {
  let mut current_entity = entity;
  if let Some(mut s) = entity.get_name() {
    while let Some(p) = current_entity.get_semantic_parent() {
      match p.get_kind() {
        EntityKind::ClassDecl |
        EntityKind::ClassTemplate |
        EntityKind::StructDecl |
        EntityKind::Namespace |
        EntityKind::EnumDecl |
        EntityKind::ClassTemplatePartialSpecialization => {
          match p.get_name() {
            Some(p_name) => s = format!("{}::{}", p_name, s),
            None => return Err("Anonymous nested type".to_string()),
          }
          current_entity = p;
        }
        EntityKind::Method => {
          return Err("Type nested in a method".to_string());
        }
        _ => break,
      }
    }
    Ok(s)
  } else {
    Err("Anonymous type".to_string())
  }
}

#[derive(Clone, Debug)]
pub struct CppParserConfig {
  /// Include dirs passed to clang
  pub include_dirs: Vec<PathBuf>,
  /// Frameworks passed to clang
  pub framework_dirs: Vec<PathBuf>,
  /// Header name used in #include statement
  pub header_name: String,
  /// Directory containing headers of the target library.
  /// Only entities declared within this directory will be processed.
  pub target_include_dirs: Option<Vec<PathBuf>>,
  pub tmp_cpp_path: PathBuf,
  pub name_blacklist: Vec<String>,
}

#[cfg(test)]
fn init_clang() -> Clang {
  use std;
  for _ in 0..20 {
    if let Ok(clang) = Clang::new() {
      return clang;
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
  }
  panic!("clang init failed");
}

#[cfg(not(test))]
fn init_clang() -> Clang {
  Clang::new().unwrap_or_else(|err| panic!("clang init failed: {:?}", err))
}



#[cfg_attr(feature="clippy", allow(block_in_if_condition_stmt))]
fn run_clang<R, F: Fn(Entity) -> R>(config: &CppParserConfig, cpp_code: Option<String>, f: F) -> R {
  let clang = init_clang();
  let index = Index::new(&clang, false, false);
  {
    let mut tmp_file = File::create(&config.tmp_cpp_path).unwrap();
    write!(tmp_file, "#include \"{}\"\n", config.header_name).unwrap();
    if let Some(cpp_code) = cpp_code {
      tmp_file.write(cpp_code.as_bytes()).unwrap();
    }
  }
  // TODO: PIC and additional args should be moved to lib spec (#13)
  let mut args = vec!["-fPIC".to_string(),
                      "-fcxx-exceptions".to_string(),
                      "-Xclang".to_string(),
                      "-detailed-preprocessing-record".to_string()];
  if is_msvc() {
    args.push("-std=c++14".to_string());
  } else {
    args.push("-std=gnu++11".to_string());
  }
  for dir in &config.include_dirs {
    args.push("-I".to_string());
    args.push(dir.to_str().unwrap().to_string());
  }
  if let Ok(path) = env::var("CLANG_SYSTEM_INCLUDE_PATH") {
    args.push("-isystem".to_string());
    args.push(path);
  }
  for dir in &config.framework_dirs {
    args.push("-F".to_string());
    args.push(dir.to_str().unwrap().to_string());
  }
  log::info(format!("clang arguments: {:?}", args));

  let tu = index.parser(&config.tmp_cpp_path)
    .arguments(&args)
    .parse()
    .unwrap_or_else(|err| panic!("clang parse failed: {:?}", err));
  let translation_unit = tu.get_entity();
  assert!(translation_unit.get_kind() == EntityKind::TranslationUnit);
  {
    let diagnostics = tu.get_diagnostics();
    if !diagnostics.is_empty() {
      log::warning("Diagnostics:");
      for diag in &diagnostics {
        log::warning(format!("{}", diag));
      }
    }
    if diagnostics.iter()
      .any(|d| {
        d.get_severity() == clang::diagnostic::Severity::Error ||
        d.get_severity() == clang::diagnostic::Severity::Fatal
      }) {
      panic!("terminated because of clang errors");
    }
  }
  let result = f(translation_unit);
  fs::remove_file(&config.tmp_cpp_path).unwrap();
  result
}

// TODO: use &[&CppTypeData]
pub fn run(config: CppParserConfig, dependency_types: &[CppTypeData]) -> CppData {
  log::info(get_version());
  log::info("Initializing clang...");
  let (mut parser, methods) = run_clang(&config, None, |translation_unit| {
    let mut parser = CppParser {
      types: Vec::new(),
      config: config.clone(),
      dependency_types: Vec::from(dependency_types),
    };
    log::info("Parsing types...");
    parser.parse_types(translation_unit);
    log::info("Parsing methods...");
    let methods = parser.parse_methods(translation_unit);
    (parser, methods)
  });
  log::info("Checking integrity...");
  let (good_methods, good_types) = parser.check_integrity(methods);
  parser.types = good_types;
  log::info("Searching for template instantiations...");
  let template_instantiations = parser.find_template_instantiations(&good_methods);
  log::info("Determining type sizes of template instantiations...");
  let mut cpp_code = "class AllFields {\npublic:\n".to_string();
  for (field_num, &(ref class_name, ref template_args)) in template_instantiations.iter()
    .enumerate() {
    cpp_code = cpp_code +
               &format!("  {} field{};\n",
                        CppTypeClassBase {
                            name: class_name.clone(),
                            template_arguments: Some(template_args.clone()),
                          }
                          .to_cpp_code()
                          .unwrap(),
                        field_num);
  }
  cpp_code = cpp_code + "};\n";
  let final_template_instantiations = run_clang(&config, Some(cpp_code), |translation_unit| {
    let last_entity = {
      let mut top_entities = translation_unit.get_children();
      if top_entities.is_empty() {
        panic!("AllFields not found: no entities");
      }
      top_entities.pop().unwrap()
    };
    if let Some(name) = last_entity.get_name() {
      if name != "AllFields" {
        panic!("AllFields not found: entity name mismatch: '{}'", name);
      }
    } else {
      panic!("AllFields not found: entity has no name");
    }
    let mut parser2 = CppParser {
      types: Vec::new(),
      config: config.clone(),
      dependency_types: Vec::from(dependency_types),
    };
    parser2.parse_types(last_entity);
    if parser2.types.len() != 1 {
      panic!("AllFields parse result: expected 1 type");
    }
    let mut final_template_instantiations = Vec::<CppTemplateInstantiations>::new();
    if let CppTypeKind::Class { ref fields, .. } = parser2.types[0].kind {
      if fields.len() != template_instantiations.len() {
        panic!("AllFields parse result: fields count mismatch");
      }
      for (field_num, &(ref class_name, ref template_args)) in template_instantiations.iter()
        .enumerate() {
        let size = fields[field_num].size;
        if size.is_none() {
          panic!("AllFields parse result: failed to get size of {}<{:?}>",
                 class_name,
                 template_args);
        }
        if final_template_instantiations.iter().find(|x| &x.class_name == class_name).is_none() {
          // first encounter of this template class
          if let Some(type_info) = parser.find_type(|x| &x.name == class_name) {
            if let CppTypeKind::Class { ref template_arguments, .. } = type_info.kind {
              if template_arguments.is_none() {
                panic!("Invalid instantiation: type {} is not a template class",
                       class_name);
              }
            } else {
              panic!("Invalid instantiation: type {} is not a class", class_name);
            }
            final_template_instantiations.push(CppTemplateInstantiations {
              class_name: class_name.clone(),
              include_file: type_info.include_file.clone(),
              instantiations: Vec::new(),
            });
          } else {
            panic!("Invalid instantiation: unknown class type: {}", class_name);
          }
        }
        // TODO: reimplement checking template args count
        // TODO: pass inst. of dependencies here and skip duplicates
        let result_item =
          final_template_instantiations.iter_mut().find(|x| &x.class_name == class_name).unwrap();
        result_item.instantiations.push(CppTemplateInstantiation {
          template_arguments: template_args.clone(),
          size: size.unwrap(),
        });
      }
    } else {
      panic!("AllFields parse result: type is not a class");
    }
    final_template_instantiations
  });

  log::info("C++ parser finished.");
  CppData {
    types: parser.types,
    methods: good_methods,
    template_instantiations: final_template_instantiations,
  }


}

impl CppParser {
  fn find_type<F: Fn(&CppTypeData) -> bool>(&self, f: F) -> Option<&CppTypeData> {
    if let Some(r) = self.types.iter().find(|x| f(x)) {
      return Some(r);
    }
    if let Some(r) = self.dependency_types.iter().find(|x| f(x)) {
      return Some(r);
    }
    None
  }

  fn parse_unexposed_type(&self,
                          type1: Option<Type>,
                          string: Option<String>,
                          context_class: Option<Entity>,
                          context_method: Option<Entity>)
                          -> Result<CppType, String> {
    let template_class_regex = Regex::new(r"^([\w:]+)<(.+)>$").unwrap();
    let (is_const, name) = if let Some(type1) = type1 {
      let is_const = type1.is_const_qualified();
      let mut name = type1.get_display_name();
      let is_const_in_name = name.starts_with("const ");
      if is_const != is_const_in_name {
        panic!("const inconsistency: {}, {:?}", is_const, type1);
      }
      if is_const_in_name {
        name = name[6..].to_string();
      }
      if let Some(declaration) = type1.get_declaration() {
        if declaration.get_kind() == EntityKind::ClassDecl ||
           declaration.get_kind() == EntityKind::ClassTemplate ||
           declaration.get_kind() == EntityKind::StructDecl {
          if declaration.get_accessibility().unwrap_or(Accessibility::Public) !=
             Accessibility::Public {
            return Err(format!("Type uses private class ({})",
                               get_full_name(declaration).unwrap()));
          }
          if let Some(matches) = template_class_regex.captures(name.as_ref()) {
            let mut arg_types = Vec::new();
            for arg in matches.at(2).unwrap().split(',') {
              match self.parse_unexposed_type(None,
                                              Some(arg.trim().to_string()),
                                              context_class,
                                              context_method) {
                Ok(arg_type) => arg_types.push(arg_type),
                Err(msg) => {
                  return Err(format!("Template argument of unexposed type is not parsed: {}: {}",
                                     arg,
                                     msg))
                }
              }
            }
            return Ok(CppType {
              base: CppTypeBase::Class(CppTypeClassBase {
                name: get_full_name(declaration).unwrap(),
                template_arguments: Some(arg_types),
              }),
              is_const: is_const,
              is_const2: false,
              indirection: CppTypeIndirection::None,
            });
          } else {
            return Err(format!("Unexposed type has a declaration but is too complex: {}",
                               name));
          }
        }
      }
      (is_const, name)
    } else {
      let mut name = string.unwrap();
      let is_const_in_name = name.starts_with("const ");
      if is_const_in_name {
        name = name[6..].to_string();
      }
      (is_const_in_name, name)
    };
    let re = Regex::new(r"^type-parameter-(\d+)-(\d+)$").unwrap();
    if let Some(matches) = re.captures(name.as_ref()) {
      return Ok(CppType {
        base: CppTypeBase::TemplateParameter {
          nested_level: matches.at(1).unwrap().parse().unwrap(),
          index: matches.at(2).unwrap().parse().unwrap(),
        },
        is_const: is_const,
        is_const2: false,
        indirection: CppTypeIndirection::None,
      });
    }
    if let Some(e) = context_method {
      if let Some(args) = get_template_arguments(e) {
        if let Some(index) = args.names.iter().position(|x| *x == name) {
          return Ok(CppType {
            base: CppTypeBase::TemplateParameter {
              nested_level: args.nested_level,
              index: index as i32,
            },
            is_const: is_const,
            is_const2: false,
            indirection: CppTypeIndirection::None,
          });
        }
      }
    }
    if let Some(e) = context_class {
      if let Some(args) = get_template_arguments(e) {
        if let Some(index) = args.names.iter().position(|x| *x == name) {
          return Ok(CppType {
            base: CppTypeBase::TemplateParameter {
              nested_level: args.nested_level,
              index: index as i32,
            },
            is_const: is_const,
            is_const2: false,
            indirection: CppTypeIndirection::None,
          });
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
      .find(|x| x.to_cpp_code() == remaining_name) {
      result_type.base = CppTypeBase::BuiltInNumeric(x.clone());
      return Ok(result_type);
    }
    if result_type.indirection == CppTypeIndirection::Ptr ||
       result_type.indirection == CppTypeIndirection::Ref {
      if let Ok(subtype) = self.parse_unexposed_type(None,
                                                     Some(remaining_name.to_string()),
                                                     context_class,
                                                     context_method) {
        let mut new_indirection = try!(CppTypeIndirection::combine(&subtype.indirection,
                                                                   &result_type.indirection)
          .map_err(|e| e.to_string()));
        if new_indirection == CppTypeIndirection::Ptr {
          if let CppTypeBase::FunctionPointer { .. } = subtype.base {
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
          result_type.base = CppTypeBase::Enum { name: remaining_name.to_string() }
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
      let class_name = matches.at(1).unwrap();
      if self.find_type(|x| &x.name == class_name && x.is_class()).is_some() {
        let mut arg_types = Vec::new();
        for arg in matches.at(2).unwrap().split(',') {
          match self.parse_unexposed_type(None,
                                          Some(arg.trim().to_string()),
                                          context_class,
                                          context_method) {
            Ok(arg_type) => arg_types.push(arg_type),
            Err(msg) => {
              return Err(format!("Template argument of unexposed type is not parsed: {}: {}",
                                 arg,
                                 msg))
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
      return Err(format!("Unexposed type has a declaration but is too complex: {}",
                         name));
    }

    Err(format!("Unrecognized unexposed type: {}", name))
  }

  fn parse_type(&self,
                type1: Type,
                context_class: Option<Entity>,
                context_method: Option<Entity>)
                -> Result<CppType, String> {
    let display_name = type1.get_display_name();
    if &display_name == "std::list<T>" {
      return Err(format!("Type blacklisted because it causes crash on Windows: {}",
                         display_name));
    }

    let parsed =
      try!(self.parse_canonical_type(type1.get_canonical_type(), context_class, context_method));
    if let CppTypeBase::BuiltInNumeric(..) = parsed.base {
      if parsed.indirection == CppTypeIndirection::None {
        let mut name = type1.get_display_name();
        if name.starts_with("const ") {
          name = name[6..].trim().to_string();
        }
        let real_type = match name.as_ref() {
          "qint8" | "int8_t" => {
            Some(CppTypeBase::SpecificNumeric {
              name: name.to_string(),
              bits: 8,
              kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
            })
          }
          "quint8" | "uint8_t" => {
            Some(CppTypeBase::SpecificNumeric {
              name: name.to_string(),
              bits: 8,
              kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
            })
          }
          "qint16" | "int16_t" => {
            Some(CppTypeBase::SpecificNumeric {
              name: name.to_string(),
              bits: 16,
              kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
            })
          }
          "quint16" | "uint16_t" => {
            Some(CppTypeBase::SpecificNumeric {
              name: name.to_string(),
              bits: 16,
              kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
            })
          }
          "qint32" | "int32_t" => {
            Some(CppTypeBase::SpecificNumeric {
              name: name.to_string(),
              bits: 32,
              kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
            })
          }
          "quint32" | "uint32_t" => {
            Some(CppTypeBase::SpecificNumeric {
              name: name.to_string(),
              bits: 32,
              kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
            })
          }
          "qint64" | "int64_t" | "qlonglong" => {
            Some(CppTypeBase::SpecificNumeric {
              name: name.to_string(),
              bits: 64,
              kind: CppSpecificNumericTypeKind::Integer { is_signed: true },
            })
          }
          "quint64" | "uint64_t" | "qulonglong" => {
            Some(CppTypeBase::SpecificNumeric {
              name: name.to_string(),
              bits: 64,
              kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
            })
          }
          "qintptr" |
          "qptrdiff" |
          "QList_difference_type" => {
            Some(CppTypeBase::PointerSizedInteger {
              name: name.to_string(),
              is_signed: true,
            })
          }
          "quintptr" => {
            Some(CppTypeBase::PointerSizedInteger {
              name: name.to_string(),
              is_signed: false,
            })
          }
          _ => None,
        };
        if let Some(real_type) = real_type {
          return Ok(CppType {
            base: real_type,
            indirection: parsed.indirection,
            is_const: parsed.is_const,
            is_const2: parsed.is_const2,
          });
        }
      }
    }
    Ok(parsed)
  }


  fn parse_canonical_type(&self,
                          type1: Type,
                          context_class: Option<Entity>,
                          context_method: Option<Entity>)
                          -> Result<CppType, String> {
    if type1.is_volatile_qualified() {
      return Err("Volatile type".to_string());
    }
    let is_const = type1.is_const_qualified();
    match type1.get_kind() {
      TypeKind::Void => {
        Ok(CppType {
          base: CppTypeBase::Void,
          is_const: is_const,
          is_const2: false,
          indirection: CppTypeIndirection::None,
        })
      }
      TypeKind::Bool |
      TypeKind::CharS |
      TypeKind::CharU |
      TypeKind::SChar |
      TypeKind::UChar |
      TypeKind::WChar |
      TypeKind::Char16 |
      TypeKind::Char32 |
      TypeKind::Short |
      TypeKind::UShort |
      TypeKind::Int |
      TypeKind::UInt |
      TypeKind::Long |
      TypeKind::ULong |
      TypeKind::LongLong |
      TypeKind::ULongLong |
      TypeKind::Int128 |
      TypeKind::UInt128 |
      TypeKind::Float |
      TypeKind::Double |
      TypeKind::LongDouble => {
        Ok(CppType {
          base: CppTypeBase::BuiltInNumeric(match type1.get_kind() {
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
          }),
          is_const: is_const,
          is_const2: false,
          indirection: CppTypeIndirection::None,
        })
      }
      TypeKind::Enum => {
        Ok(CppType {
          base: CppTypeBase::Enum { name: try!(get_full_name(type1.get_declaration().unwrap())) },
          is_const: is_const,
          is_const2: false,
          indirection: CppTypeIndirection::None,
        })
      }
      TypeKind::Record => {
        let declaration = type1.get_declaration().unwrap();
        if declaration.get_accessibility().unwrap_or(Accessibility::Public) !=
           Accessibility::Public {
          return Err(format!("Type uses private class ({})",
                             get_full_name(declaration).unwrap_or("unnamed".to_string())));
        }
        match get_full_name(declaration) {
          Ok(declaration_name) => {
            let template_arguments = match type1.get_template_argument_types() {
              None => None,
              Some(arg_types) => {
                let mut r = Vec::new();
                if arg_types.is_empty() {
                  panic!("arg_types is empty");
                }
                for arg_type in arg_types {
                  match arg_type {
                    None => return Err("Template argument is None".to_string()),
                    Some(arg_type) => {
                      match self.parse_type(arg_type, context_class, context_method) {
                        Ok(parsed_type) => r.push(parsed_type),
                        Err(msg) => {
                          return Err(format!("Invalid template argument: {:?}: {}", arg_type, msg))
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

          }
          Err(msg) => Err(format!("get_full_name failed: {}", msg)),
        }
      }
      TypeKind::FunctionPrototype => {
        let mut arguments = Vec::new();
        for arg_type in type1.get_argument_types().unwrap() {
          match self.parse_type(arg_type, context_class, context_method) {
            Ok(t) => arguments.push(t),
            Err(msg) => {
              return Err(format!("Failed to parse function type's argument type: {:?}: {}",
                                 arg_type,
                                 msg))
            }
          }
        }
        let return_type = match self.parse_type(type1.get_result_type().unwrap(),
                                                context_class,
                                                context_method) {
          Ok(t) => Box::new(t),
          Err(msg) => {
            return Err(format!("Failed to parse function type's argument type: {:?}: {}",
                               type1.get_result_type().unwrap(),
                               msg))
          }
        };
        Ok(CppType {
          base: CppTypeBase::FunctionPointer {
            return_type: return_type,
            arguments: arguments,
            allows_variadic_arguments: type1.is_variadic(),
          },
          is_const: is_const,
          is_const2: false,
          indirection: CppTypeIndirection::None,
        })
      }
      TypeKind::Pointer |
      TypeKind::LValueReference |
      TypeKind::RValueReference => {
        match type1.get_pointee_type() {
          Some(pointee) => {
            match self.parse_type(pointee, context_class, context_method) {
              Ok(subtype) => {
                let original_type_indirection = match type1.get_kind() {
                  TypeKind::Pointer => CppTypeIndirection::Ptr,
                  TypeKind::LValueReference => CppTypeIndirection::Ref,
                  TypeKind::RValueReference => CppTypeIndirection::RValueRef,
                  _ => unreachable!(),
                };

                let mut new_indirection =
                  try!(CppTypeIndirection::combine(&subtype.indirection,
                                                   &original_type_indirection)
                    .map_err(|e| e.to_string()));
                if new_indirection == CppTypeIndirection::Ptr {
                  if let CppTypeBase::FunctionPointer { .. } = subtype.base {
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
            }
          }
          None => Err("can't get pointee type".to_string()),
        }
      }
      TypeKind::Unexposed => {
        self.parse_unexposed_type(Some(type1), None, context_class, context_method)
      }
      _ => Err(format!("Unsupported kind of type: {:?}", type1.get_kind())),
    }
  }

  // TODO: simplify this function
  #[cfg_attr(feature="clippy", allow(cyclomatic_complexity))]
  fn parse_function(&self, entity: Entity) -> Result<CppMethod, String> {
    let (class_name, class_entity) = match entity.get_semantic_parent() {
      Some(p) => {
        match p.get_kind() {
          EntityKind::ClassDecl |
          EntityKind::ClassTemplate |
          EntityKind::StructDecl => {
            match get_full_name(p) {
              Ok(class_name) => (Some(class_name), Some(p)),
              Err(msg) => {
                panic!("function parent is a class but it doesn't have a name: {}",
                       msg)
              }
            }
          }
          EntityKind::ClassTemplatePartialSpecialization => {
            return Err("this function is part of a template partial specialization".to_string());
          }
          _ => (None, None),
        }
      }
      None => (None, None),
    };

    let return_type = entity.get_type()
      .unwrap_or_else(|| panic!("failed to get function type"))
      .get_result_type()
      .unwrap_or_else(|| panic!("failed to get function return type"));
    let return_type_parsed = match self.parse_type(return_type, class_entity, Some(entity)) {
      Ok(x) => x,
      Err(msg) => {
        return Err(format!("Can't parse return type: {}: {}",
                           return_type.get_display_name(),
                           msg));
      }
    };
    let mut arguments = Vec::new();
    let argument_entities = match entity.get_kind() {
      EntityKind::FunctionTemplate => {
        entity.get_children().into_iter().filter(|c| c.get_kind() == EntityKind::ParmDecl).collect()
      }
      _ => entity.get_arguments().unwrap(),
    };
    let template_arguments = match entity.get_kind() {
      EntityKind::FunctionTemplate => {
        if entity.get_children()
          .into_iter()
          .any(|c| c.get_kind() == EntityKind::NonTypeTemplateParameter) {
          return Err("Non-type template parameter is not supported".to_string());
        }
        get_template_arguments(entity)
      }
      _ => None,
    };

    for (argument_number, argument_entity) in argument_entities.into_iter()
      .enumerate() {
      let name = argument_entity.get_name()
        .unwrap_or_else(|| format!("arg{}", argument_number + 1));
      let type1 = self.parse_type(argument_entity.get_type().unwrap(),
                                  class_entity,
                                  Some(entity));

      match type1 {
        Ok(argument_type) => {
          let mut has_default_value = false;
          for token in argument_entity.get_range().unwrap().tokenize() {
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
            name: name,
            argument_type: argument_type,
            has_default_value: has_default_value,
          });
        }
        Err(msg) => {
          return Err(format!("Can't parse argument type: {}: {}: {}",
                             name,
                             argument_entity.get_type().unwrap().get_display_name(),
                             msg));
        }
      }
    }
    let mut name = entity.get_name().unwrap_or_else(|| panic!("failed to get function name"));
    if name.contains('<') {
      let regex = Regex::new(r"^([\w~]+)<[^<>]+>$").unwrap();
      if let Some(matches) = regex.captures(name.clone().as_ref()) {
        log::warning(format!("Fixing malformed method name: {}", name));
        name = matches.at(1).unwrap().to_string();
      }
    }
    let mut name_with_namespace = name.clone();
    if let Some(parent) = entity.get_semantic_parent() {
      if parent.get_kind() == EntityKind::Namespace {
        name_with_namespace = format!("{}::{}", get_full_name(parent).unwrap(), name);
      }
    }
    let allows_variadic_arguments = entity.is_variadic();
    let has_this_argument = class_name.is_some() && !entity.is_static_method();
    let real_arguments_count = arguments.len() as i32 + if has_this_argument { 1 } else { 0 };
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
        return Err("This method is recognized as operator but arguments do not match \
                            its signature."
          .to_string());
      }
    }

    if method_operator.is_none() && name.starts_with("operator ") {
      let op = name["operator ".len()..].trim();
      match self.parse_unexposed_type(None, Some(op.to_string()), class_entity, Some(entity)) {
        Ok(t) => method_operator = Some(CppOperator::Conversion(t)),
        Err(_) => return Err(format!("Unknown type in conversion operator: '{}'", op)),

      }
    }
    let source_range = entity.get_range().unwrap();
    let tokens = source_range.tokenize();
    let declaration_code = if tokens.is_empty() {
      log::noisy(format!("Failed to tokenize method {} at {:?}",
                         name_with_namespace,
                         entity.get_range().unwrap()));
      let start = source_range.get_start().get_file_location();
      let end = source_range.get_end().get_file_location();
      let file = File::open(start.file.get_path()).unwrap();
      let reader = BufReader::new(file);
      let mut result = String::new();
      let range_line1 = (start.line - 1) as usize;
      let range_line2 = (end.line - 1) as usize;
      let range_col1 = (start.column - 1) as usize;
      let range_col2 = (end.column - 1) as usize;
      for (line_num, line) in reader.lines().enumerate() {
        let line = line.unwrap();
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
      log::noisy(format!("The code extracted directly from header: {:?}", result));
      if result.contains("volatile") {
        log::warning("Warning: volatile method is detected based on source code".to_string());
        return Err("Probably a volatile method.".to_string());
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
          return Err("A volatile method.".to_string());
        }
        token_strings.push(text);
      }
      Some(token_strings.join(" "))
    };
    Ok(CppMethod {
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
            is_signal: false, // TODO: parse signals and slots (#7)
            class_type: match self.find_type(|x| &x.name == &class_name) {
              Some(info) => info.default_class_type(),
              None => return Err(format!("Unknown class type: {}", class_name)),
            },
          })
        }
        None => None,
      },
      arguments: arguments,
      arguments_before_omitting: None,
      allows_variadic_arguments: allows_variadic_arguments,
      return_type: return_type_parsed,
      include_file: self.entity_include_file(entity).unwrap(),
      origin_location: Some(get_origin_location(entity).unwrap()),
      template_arguments: template_arguments,
      template_arguments_values: None,
      declaration_code: declaration_code,
      inherited_from: None,
      inheritance_chain: Vec::new(),
    })
  }

  fn parse_enum(&self, entity: Entity) -> Result<CppTypeData, String> {
    let mut values = Vec::new();
    for child in entity.get_children() {
      if child.get_kind() == EntityKind::EnumConstantDecl {
        values.push(EnumValue {
          name: child.get_name().unwrap(),
          value: child.get_enum_constant_value().unwrap().0,
        });
      }
    }
    Ok(CppTypeData {
      name: get_full_name(entity).unwrap(),
      include_file: if let Some(x) = self.entity_include_file(entity) {
        x.clone()
      } else {
        return Err(format!("Origin of type is unknown: {}\nentity: {:?}\n",
                           get_full_name(entity).unwrap(),
                           entity));
      },
      origin_location: get_origin_location(entity).unwrap(),
      kind: CppTypeKind::Enum { values: values },
    })
  }

  fn parse_class(&self, entity: Entity) -> Result<CppTypeData, String> {
    let full_name = try!(get_full_name(entity));
    let mut fields = Vec::new();
    let mut bases = Vec::new();
    let using_directives =
      entity.get_children()
        .into_iter()
        .filter(|c| c.get_kind() == EntityKind::UsingDeclaration)
        .filter_map(|child| {
          let type_ref =
            if let Some(x) = child.get_children().into_iter().find(|c| {
              c.get_kind() == EntityKind::TypeRef || c.get_kind() == EntityKind::TemplateRef
            }) {
              x
            } else {
              log::warning("Failed to parse UsingDeclaration: class type not found".to_string());
              dump_entity(child, 0);
              return None;
            };
          let type_def = type_ref.get_definition().expect("TypeRef definition not found");
          Some(CppClassUsingDirective {
            class_name: get_full_name(type_def).expect("class_name get_full_name failed"),
            method_name: child.get_name().expect("method_name failed"),
          })
        })
        .collect();
    for child in entity.get_children() {
      if child.get_kind() == EntityKind::FieldDecl {
        let field_clang_type = child.get_type().unwrap();
        match self.parse_type(field_clang_type, Some(entity), None) {
          Ok(field_type) => {
            fields.push(CppClassField {
              size: match field_clang_type.get_sizeof() {
                Ok(size) => Some(size as i32),
                Err(_) => None,
              },
              name: child.get_name().unwrap(),
              field_type: field_type,
              visibility: match entity.get_accessibility().unwrap_or(Accessibility::Public) {
                Accessibility::Public => CppVisibility::Public,
                Accessibility::Protected => CppVisibility::Protected,
                Accessibility::Private => CppVisibility::Private,
              },
            });
          }
          Err(msg) => {
            log::warning(format!("Can't parse field type: {}::{}: {}",
                                 get_full_name(entity).unwrap_or_else(|msg| format!("[{}]", msg)),
                                 child.get_name().unwrap(),
                                 msg))
          }
        };
      }
      if child.get_kind() == EntityKind::BaseSpecifier {
        let base_type = match self.parse_type(child.get_type().unwrap(), None, None) {
          Ok(r) => r,
          Err(msg) => return Err(format!("Can't parse base class type: {}", msg)),
        };
        bases.push(CppBaseSpecifier {
          base_type: base_type,
          is_virtual: child.is_virtual_base(),
          visibility: match child.get_accessibility().unwrap_or(Accessibility::Public) {
            Accessibility::Public => CppVisibility::Public,
            Accessibility::Protected => CppVisibility::Protected,
            Accessibility::Private => CppVisibility::Private,
          },
        });
      }
      if child.get_kind() == EntityKind::NonTypeTemplateParameter {
        return Err("Non-type template parameter is not supported".to_string());
      }
    }
    let template_arguments = get_template_arguments(entity);
    if entity.get_kind() == EntityKind::ClassTemplate {
      if template_arguments.is_none() {
        panic!("missing template arguments");
      }
    } else if template_arguments.is_some() {
      panic!("unexpected template arguments");
    }
    let size = match entity.get_type() {
      Some(type1) => type1.get_sizeof().ok().map(|x| x as i32),
      None => None,
    };
    if template_arguments.is_none() && size.is_none() {
      return Err("Failed to request size, but the class is not a template class".to_string());
    }
    if let Some(parent) = entity.get_semantic_parent() {
      if get_template_arguments(parent).is_some() {
        return Err("Types nested into template types are not supported".to_string());
      }
    }
    Ok(CppTypeData {
      name: full_name,
      include_file: if let Some(x) = self.entity_include_file(entity) {
        x.clone()
      } else {
        return Err(format!("Origin of type is unknown: {}\nentity: {:?}\n",
                           get_full_name(entity).unwrap(),
                           entity));
      },
      origin_location: get_origin_location(entity).unwrap(),
      kind: CppTypeKind::Class {
        size: size,
        bases: bases,
        fields: fields,
        using_directives: using_directives,
        template_arguments: template_arguments,
      },
    })
  }

  fn entity_include_path(&self, entity: Entity) -> Option<String> {
    if let Some(location) = entity.get_location() {
      let file_path = location.get_presumed_location().0;
      if file_path.is_empty() {
        log::noisy(format!("empty file path: {:?}", entity.get_kind()));
        None
      } else {
        Some(file_path)
      }
    } else {
      None
    }
  }

  fn entity_include_file(&self, entity: Entity) -> Option<String> {
    match self.entity_include_path(entity) {
      Some(file_path) => {
        let file_path_buf = PathBuf::from(file_path.clone());
        Some(file_path_buf.file_name()
          .unwrap()
          .to_str()
          .unwrap()
          .to_string())
      }
      None => None,
    }
  }

  fn should_process_entity(&self, entity: Entity) -> bool {
    if let Ok(full_name) = get_full_name(entity) {
      if full_name == "AllFields" {
        return true; //our special class
      }
      if let Some(file_path) = self.entity_include_path(entity) {
        let file_path_buf = PathBuf::from(&file_path);
        if let Some(ref target_include_dirs) = self.config.target_include_dirs {
          if target_include_dirs.iter().find(|x| file_path_buf.starts_with(x)).is_none() {
            log::noisy(format!("skipping entities from {}", file_path));
            return false;
          }
        }
      }
      if self.config.name_blacklist.iter().any(|x| x == &full_name) {
        log::info(format!("Skipping blacklisted entity: {}", full_name));
        return false;
      }
    }
    if let Some(name) = entity.get_name() {
      if self.config.name_blacklist.iter().any(|x| x == &name) {
        log::info(format!("Skipping blacklisted entity: {}",
                          get_full_name(entity).unwrap()));
        return false;
      }
    }
    true
  }


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
          match self.parse_enum(entity) {
            Ok(r) => {
              if let Some(info) = self.find_type(|x| x.name == r.name).cloned() {
                log::warning(format!("repeating enum declaration: {:?}\nold declaration: {:?}",
                                     entity,
                                     info));
              } else {
                self.types.push(r);
              }
            }
            Err(msg) => {
              log::warning(format!("Failed to parse enum: {}\nentity: {:?}\nerror: {}\n",
                                   get_full_name(entity).unwrap(),
                                   entity,
                                   msg));
            }
          }
        }
      }
      EntityKind::ClassDecl |
      EntityKind::ClassTemplate |
      EntityKind::StructDecl => {
        if entity.get_accessibility() == Some(Accessibility::Private) {
          return; // skipping private stuff
        }
        let ok = entity.get_name().is_some() && // not an anonymous struct
        entity.is_definition() && // not a forward declaration
        entity.get_template().is_none(); // not a template specialization
        if ok {
          match self.parse_class(entity) {
            Ok(r) => {
              if let Some(info) = self.find_type(|x| x.name == r.name).cloned() {
                log::warning(format!("repeating class declaration: {:?}\nold declaration: {:?}",
                                     entity,
                                     info));
              } else {
                self.types.push(r);
              }
            }
            Err(msg) => {
              log::warning(format!("Failed to parse class: {}\nentity: {:?}\nerror: {}\n",
                                   get_full_name(entity)
                                     .unwrap_or_else(|msg| format!("[{}]", msg)),
                                   entity,
                                   msg));
            }
          }
        }
      }
      _ => {}
    }
    for c in entity.get_children() {
      if c.get_kind() == EntityKind::BinaryOperator && c.get_location() == entity.get_location() {
        log::warning("get_children refers to itself!");
        continue;
      }
      self.parse_types(c);
    }
  }

  fn parse_methods(&self, entity: Entity) -> Vec<CppMethod> {
    let mut methods = Vec::new();
    if !self.should_process_entity(entity) {
      return methods;
    }
    match entity.get_kind() {
      EntityKind::FunctionDecl |
      EntityKind::Method |
      EntityKind::Constructor |
      EntityKind::Destructor |
      EntityKind::ConversionFunction |
      EntityKind::FunctionTemplate => {
        if entity.get_canonical_entity() == entity {
          match self.parse_function(entity) {
            Ok(r) => {
              methods.push(r);
            }
            Err(msg) => {
              let full_name = get_full_name(entity).unwrap();
              let message = format!("Failed to parse method: {}\nentity: {:?}\nerror: {}\n",
                                    full_name,
                                    entity,
                                    msg);
              log::warning(message.as_ref());
            }
          }
        }
      }
      EntityKind::StructDecl |
      EntityKind::ClassDecl |
      EntityKind::ClassTemplate |
      EntityKind::ClassTemplatePartialSpecialization => {
        if let Some(name) = entity.get_display_name() {
          if let Ok(parent_type) = self.parse_unexposed_type(None, Some(name.clone()), None, None) {
            if let CppTypeBase::Class(CppTypeClassBase { ref template_arguments, .. }) =
                   parent_type.base {
              if let Some(ref template_arguments) = *template_arguments {
                if template_arguments.iter().any(|x| !x.base.is_template_parameter()) {
                  log::warning(format!("skipping template partial specialization: {}", name));
                  return methods;
                }
              }
            }
          }
        }
      }
      _ => {}
    }
    // TODO: check children only if it makes sense for the entity kind
    for c in entity.get_children() {
      if c.get_kind() == EntityKind::BinaryOperator && c.get_location() == entity.get_location() {
        log::warning("get_children refers to itself!");
        continue;
      }
      methods.append(&mut self.parse_methods(c));
    }
    methods
  }

  fn check_type_integrity(&self, type1: &CppType) -> Result<(), String> {
    match type1.base {
      CppTypeBase::Void |
      CppTypeBase::BuiltInNumeric(..) |
      CppTypeBase::SpecificNumeric { .. } |
      CppTypeBase::PointerSizedInteger { .. } |
      CppTypeBase::TemplateParameter { .. } => {}
      CppTypeBase::Enum { ref name } => {
        if self.find_type(|x| &x.name == name).is_none() {
          return Err(format!("unknown type: {}", name));
        }
      }
      CppTypeBase::Class(CppTypeClassBase { ref name, ref template_arguments }) => {
        if self.find_type(|x| &x.name == name).is_none() {
          return Err(format!("unknown type: {}", name));
        }
        if let Some(ref args) = *template_arguments {
          for arg in args {
            if let Err(msg) = self.check_type_integrity(arg) {
              return Err(msg);
            }
          }
        }
      }
      CppTypeBase::FunctionPointer { ref return_type, ref arguments, .. } => {
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

  fn check_integrity(&self, methods: Vec<CppMethod>) -> (Vec<CppMethod>, Vec<CppTypeData>) {
    log::info("Checking data integrity");
    let good_methods = methods.into_iter()
      .filter(|method| {
        if let Err(msg) = self.check_type_integrity(&method.return_type
          .clone()) {
          log::warning(format!("Method is removed: {}: {}", method.short_text(), msg));
          return false;
        }
        for arg in &method.arguments {
          if let Err(msg) = self.check_type_integrity(&arg.argument_type) {
            log::warning(format!("Method is removed: {}: {}", method.short_text(), msg));
            return false;
          }
        }
        true
      })
      .collect();

    let mut good_types = Vec::new();
    for t in &self.types {
      let mut good_type = t.clone();
      if let CppTypeKind::Class { ref mut bases, .. } = good_type.kind {
        let mut valid_bases = Vec::new();
        for base in bases.iter() {
          if let Err(msg) = self.check_type_integrity(&base.base_type) {
            log::warning(format!("Class {}: base class removed because type is not available: \
                                  {:?}: {}",
                                 t.name,
                                 base,
                                 msg));
          } else {
            valid_bases.push(base.clone());
          }
        }
        bases.clear();
        bases.append(&mut valid_bases);
      }
      good_types.push(good_type);
    }
    (good_methods, good_types)
  }

  fn find_template_instantiations(&self, methods: &[CppMethod]) -> Vec<(String, Vec<CppType>)> {

    fn check_type(type1: &CppType, result: &mut Vec<(String, Vec<CppType>)>) {
      if let CppTypeBase::Class(CppTypeClassBase { ref name, ref template_arguments }) =
             type1.base {
        if let Some(ref template_arguments) = *template_arguments {
          if !template_arguments.iter().any(|x| x.base.is_or_contains_template_parameter()) &&
             !result.iter().any(|x| &x.0 == name && &x.1 == template_arguments) {
            log::noisy(format!("Found template instantiation: {}<{:?}>",
                               name,
                               template_arguments));
            result.push((name.clone(), template_arguments.clone()));
          }
          for arg in template_arguments {
            check_type(arg, result);
          }
        }
      }
    }
    let mut result = Vec::new();
    for m in methods {
      check_type(&m.return_type, &mut result);
      for arg in &m.arguments {
        check_type(&arg.argument_type, &mut result);
      }
    }
    for t in &self.types {
      if let CppTypeKind::Class { ref bases, .. } = t.kind {
        for base in bases {
          check_type(&base.base_type, &mut result);
        }
      }
    }
    result
  }
}
