extern crate clang;
use self::clang::*;

extern crate regex;
use self::regex::Regex;

use log;
use std::collections::HashSet;
use std::path::PathBuf;
// use std::ffi::OsStr;

use clang_cpp_data::{CLangCppData, CLangCppTypeData, CLangCppTypeKind, CLangClassField};
use cpp_type_map::EnumValue;
// use cpp_type_map::CppTypeInfo;
use cpp_method::{CppMethod, CppFunctionArgument};
use enums::{CppMethodScope, CppTypeOrigin, CppTypeIndirection};
use cpp_type::{CppType, CppTypeBase, CppBuiltInNumericType};

#[derive(Default)]
pub struct CppParserStats {
  pub total_methods: i32,
  pub failed_methods: i32,
  pub success_methods: i32,
  pub total_types: i32,
  pub failed_types: i32,
  pub success_types: i32,
}




pub struct CppParser {
  library_include_dir: PathBuf,
  entity_kinds: HashSet<EntityKind>,
  files: HashSet<String>,
  data: CLangCppData,
  stats: CppParserStats,
}

#[allow(dead_code)]
fn inspect_method(entity: Entity) {
  println!("{:?}", entity.get_display_name());
  //  println!("children:");
  //  for c in entity.get_children() {
  //    println!("child {:?}", c);
  //  }
  println!("type: {:?}", entity.get_type());
  println!("return type: {:?}",
           entity.get_type().unwrap().get_result_type());
  println!("args:");
  for c in entity.get_arguments().unwrap() {
    // println!("arg: {:?}", c);
    println!("arg: name={} type={:?}",
             c.get_name().unwrap_or("[no name]".to_string()),
             c.get_type());
  }
}

#[allow(dead_code)]
fn dump_entity(entity: &Entity, level: i32) {
  for _ in 0..level {
    print!(". ");
  }
  println!("{:?}", entity);
  for child in entity.get_children() {
    dump_entity(&child, level + 1);
  }
}

fn get_template_arguments(entity: Entity) -> Vec<String> {
  entity.get_children()
        .into_iter()
        .filter(|c| c.get_kind() == EntityKind::TemplateTypeParameter)
        .enumerate()
        .map(|(i, c)| c.get_name().unwrap_or_else(|| format!("Type{}", i + 1)))
        .collect()
}


fn get_full_name(entity: Entity) -> Result<String, String> {
  let mut current_entity = entity;
  if let Some(mut s) = entity.get_name() {
    loop {
      if let Some(p) = current_entity.get_semantic_parent() {
        if p.get_kind() == EntityKind::ClassDecl || p.get_kind() == EntityKind::ClassTemplate ||
           p.get_kind() == EntityKind::StructDecl ||
           p.get_kind() == EntityKind::Namespace ||
           p.get_kind() == EntityKind::EnumDecl || p.get_kind() == EntityKind::Method ||
           p.get_kind() == EntityKind::ClassTemplatePartialSpecialization {
          match p.get_name() {
            Some(p_name) => s = format!("{}::{}", p_name, s),
            None => return Err(format!("Anonymous nested type")),
          }
          current_entity = p;
        } else {
          break;
        }
      }
    }
    Ok(s)
  } else {
    Err(format!("Anonymous type"))
  }
}

#[derive(Clone)]
struct EntityContext {
  includes: Vec<String>,
  level: i32,
}

impl EntityContext {
  fn new() -> Self {
    EntityContext {
      level: 0,
      includes: Vec::new(),
    }
  }
}


impl CppParser {
  pub fn new() -> Self {
    CppParser {
      library_include_dir: PathBuf::from("/home/ri/bin/Qt/5.5/gcc_64/include/QtCore"),
      entity_kinds: HashSet::new(),
      files: HashSet::new(),
      data: CLangCppData {
        methods: Vec::new(),
        types: Vec::new(),
      },
      stats: Default::default(),
    }
  }

  pub fn get_data(self) -> CLangCppData {
    self.data
  }

  fn parse_unexposed_type(&mut self,
                          type1: Option<Type>,
                          string: Option<String>,
                          context_class: Option<Entity>,
                          context_method: Option<Entity>)
                          -> Result<CppType, String> {
    let (is_const, name) = match type1 {
      Some(type1) => {
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
          // println!("declaration: {:?}", type1.get_declaration());
          if declaration.get_kind() == EntityKind::ClassDecl ||
             declaration.get_kind() == EntityKind::ClassTemplate ||
             declaration.get_kind() == EntityKind::StructDecl {
            if declaration.get_accessibility().unwrap_or(Accessibility::Public) ==
               Accessibility::Private {
              return Err(format!("Type uses private class ({})",
                                 get_full_name(declaration).unwrap()));
            }
            let re1 = Regex::new(r"^[\w:]+<([^<>]+)>$").unwrap();
            if let Some(matches) = re1.captures(name.as_ref()) {
              let mut arg_types = Vec::new();
              for arg in matches.at(1).unwrap().split(",") {
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
                base: CppTypeBase::Class {
                  name: get_full_name(declaration).unwrap(),
                  template_arguments: Some(arg_types),
                },
                is_const: is_const,
                indirection: CppTypeIndirection::None,
              });
            } else {
              return Err(format!("Unexposed type has a declaration but is too complex: {}",
                                 name));
            }
          }
        }
        (is_const, name)
      }
      None => {
        let mut name = string.unwrap();
        let is_const_in_name = name.starts_with("const ");
        if is_const_in_name {
          name = name[6..].to_string();
        }
        (is_const_in_name, name)
      }
    };
    let re = Regex::new(r"^type-parameter-(\d+)-(\d+)$").unwrap();
    if let Some(matches) = re.captures(name.as_ref()) {
      return Ok(CppType {
        base: CppTypeBase::TemplateParameter {
          index1: matches.at(1).unwrap().parse().unwrap(),
          index2: matches.at(2).unwrap().parse().unwrap(),
        },
        is_const: is_const,
        indirection: CppTypeIndirection::None,
      });
    }
    if let Some(e) = context_class {
      if let Some(index) = get_template_arguments(e).iter().position(|x| *x == name) {
        return Ok(CppType {
          base: CppTypeBase::TemplateParameter {
            index1: 0, // TODO: not sure what this index means
            index2: index as i32,
          },
          is_const: is_const,
          indirection: CppTypeIndirection::None,
        });
      }
    }
    if let Some(e) = context_method {
      if let Some(index) = get_template_arguments(e).iter().position(|x| *x == name) {
        return Ok(CppType {
          base: CppTypeBase::TemplateParameter {
            index1: 0, // TODO: not sure what this index means
            index2: index as i32,
          },
          is_const: is_const,
          indirection: CppTypeIndirection::None,
        });
      }
    }
    // println!("type is unexposed: {:?}", type1);

    return Err(format!("Unrecognized unexposed type: {}", name));
  }


  fn parse_type(&mut self,
                type1: Type,
                context_class: Option<Entity>,
                context_method: Option<Entity>)
                -> Result<CppType, String> {
    let is_const = type1.is_const_qualified();
    match type1.get_kind() {
      TypeKind::Void => {
        Ok(CppType {
          base: CppTypeBase::Void,
          is_const: is_const,
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
            TypeKind::CharS => CppBuiltInNumericType::CharS,
            TypeKind::CharU => CppBuiltInNumericType::CharU,
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
          indirection: CppTypeIndirection::None,
        })
      }
      TypeKind::Enum => {
        Ok(CppType {
          base: CppTypeBase::Enum {
            name: get_full_name(type1.get_declaration().unwrap()).unwrap(),
          },
          is_const: is_const,
          indirection: CppTypeIndirection::None,
        })
      }
      TypeKind::Record => {
        let declaration = type1.get_declaration().unwrap();
        if declaration.get_accessibility().unwrap_or(Accessibility::Public) ==
           Accessibility::Private {
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
                    None => return Err(format!("Template argument is None")),
                    Some(arg_type) => {
                      match self.parse_type(arg_type.get_canonical_type(),
                                            context_class,
                                            context_method) {
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
              base: CppTypeBase::Class {
                name: declaration_name,
                template_arguments: template_arguments,
              },
              is_const: is_const,
              indirection: CppTypeIndirection::None,
            })

          }
          Err(msg) => Err(format!("get_full_name failed: {}", msg)),
        }
      }
      TypeKind::Pointer |
      TypeKind::LValueReference |
      TypeKind::RValueReference => {
        match type1.get_pointee_type() {
          Some(pointee) => {
            match self.parse_type(pointee.get_canonical_type(), context_class, context_method) {
              Ok(result) => {
                let new_indirection = match type1.get_kind() {
                  TypeKind::Pointer => {
                    match result.indirection {
                      CppTypeIndirection::None => Ok(CppTypeIndirection::Ptr),
                      CppTypeIndirection::Ptr => Ok(CppTypeIndirection::PtrPtr),
                      _ => {
                        Err(format!("Unsupported level of indirection: pointer to {:?}",
                                    result.indirection))
                      }
                    }
                  }
                  TypeKind::LValueReference => {
                    match result.indirection {
                      CppTypeIndirection::None => Ok(CppTypeIndirection::Ref),
                      CppTypeIndirection::Ptr => Ok(CppTypeIndirection::PtrRef),
                      _ => {
                        Err(format!("Unsupported level of indirection: reference to {:?}",
                                    result.indirection))
                      }
                    }
                  }
                  TypeKind::RValueReference => {
                    if result.indirection == CppTypeIndirection::None {
                      Ok(CppTypeIndirection::Ref)
                    } else {
                      Err(format!("Unsupported level of indirection: r-value reference to {:?}",
                                  result.indirection))
                    }
                  }
                  _ => unreachable!(),
                };
                match new_indirection {
                  Ok(new_indirection) => Ok(CppType { indirection: new_indirection, ..result }),
                  Err(msg) => Err(msg),
                }
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

  fn parse_function(&mut self,
                    entity: Entity,
                    include_file: &Option<String>)
                    -> Result<CppMethod, String> {
    //    log::debug(format!("Parsing function: {:?}", get_full_name(entity).unwrap()));
    //    let allow_debug_print = get_full_name(entity).unwrap().find("QString::").is_some();
    //    if allow_debug_print {
    //    }
    let (scope, class_entity) = match entity.get_semantic_parent() {
      Some(p) => {
        match p.get_kind() {
          EntityKind::ClassDecl |
          EntityKind::ClassTemplate |
          EntityKind::StructDecl => {
            match get_full_name(p) {
              Ok(class_name) => (CppMethodScope::Class(class_name), Some(p)),
              Err(msg) => {
                panic!("function parent is a class but it doesn't have a name: {}",
                       msg)
              }
            }
          }
          _ => (CppMethodScope::Global, None),
        }
      }
      None => (CppMethodScope::Global, None),
    };
    if let CppMethodScope::Class(..) = scope {
      if let Some(accessibility) = entity.get_accessibility() {
        if accessibility == Accessibility::Private {
          return Err("Private method".to_string());
        }
      } else {
        panic!("class method without accessibility");
      }
    }
    let return_type = entity.get_type()
                            .unwrap_or_else(|| panic!("failed to get function type"))
                            .get_result_type()
                            .unwrap_or_else(|| panic!("failed to get function return type"));
    let return_type_parsed = match self.parse_type(return_type.get_canonical_type(),
                                                   class_entity,
                                                   Some(entity)) {
      Ok(x) => x,
      Err(msg) => {
        //        dump_entity(&entity, 0);
        //        println!("first child's type: {:?}", entity.get_children()[0].get_type());
        return Err(format!("Can't parse return type: {:?}: {}", return_type, msg));
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
                 .find(|c| c.get_kind() == EntityKind::NonTypeTemplateParameter)
                 .is_some() {
          return Err(format!("Non-type template parameter is not supported"));
        }
        Some(get_template_arguments(entity))
      }
      _ => None,
    };

    for (argument_number, argument_entity) in argument_entities.into_iter()
                                                               .enumerate() {
      let name = argument_entity.get_name().unwrap_or(format!("arg{}", argument_number + 1));
      let type1 = self.parse_type(argument_entity.get_type().unwrap().get_canonical_type(),
                                  class_entity,
                                  Some(entity));

      match type1 {
        Ok(argument_type) => {
          arguments.push(CppFunctionArgument {
            name: name,
            argument_type: argument_type,
            default_value: if argument_entity.get_range()
                                             .unwrap()
                                             .tokenize()
                                             .iter()
                                             .find(|t| t.get_spelling() == "=")
                                             .is_some() {
              Some("?".to_string())
            } else {
              None
            },
          });
        }
        Err(msg) => {
          return Err(format!("Can't parse argument type: {}: {:?}: {}",
                             name,
                             argument_entity.get_type().unwrap(),
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
    Ok(CppMethod {
      name: name,
      scope: scope,
      is_virtual: entity.is_virtual_method(),
      is_pure_virtual: entity.is_pure_virtual_method(),
      is_const: entity.is_const_method(),
      is_static: entity.is_static_method(),
      is_protected: entity.get_accessibility().unwrap_or(Accessibility::Public) ==
                    Accessibility::Protected,
      is_signal: false, // TODO: somehow get this information
      arguments: arguments,
      allows_variable_arguments: entity.is_variadic(),
      return_type: Some(return_type_parsed),
      is_constructor: entity.get_kind() == EntityKind::Constructor,
      is_destructor: entity.get_kind() == EntityKind::Destructor,
      operator: None, // TODO: operator
      is_variable: false, // TODO: move variables into CppTypeInfo
      original_index: -1,
      origin: match include_file {
        &Some(ref include_file) => CppTypeOrigin::CLang { include_file: include_file.clone() },
        &None => CppTypeOrigin::Unknown,
      },
      template_arguments: template_arguments,
    })
  }

  fn parse_enum(&mut self,
                entity: Entity,
                include_file: &String)
                -> Result<CLangCppTypeData, String> {
    let mut values = Vec::new();
    for child in entity.get_children() {
      if child.get_kind() == EntityKind::EnumConstantDecl {
        values.push(EnumValue {
          name: child.get_name().unwrap(),
          value: child.get_enum_constant_value().unwrap().0,
        });
      }
    }
    Ok(CLangCppTypeData {
      name: get_full_name(entity).unwrap(),
      header: include_file.clone(),
      kind: CLangCppTypeKind::Enum { values: values },
    })
  }

  fn parse_class(&mut self,
                 entity: Entity,
                 include_file: &String)
                 -> Result<CLangCppTypeData, String> {
    let mut fields = Vec::new();
    let mut bases = Vec::new();
    let template_arguments = get_template_arguments(entity);
    for child in entity.get_children() {
      if child.get_kind() == EntityKind::FieldDecl {
        let is_protected = match child.get_accessibility().unwrap() {
          Accessibility::Private => continue,
          Accessibility::Protected => true,
          Accessibility::Public => false,
        };
        match self.parse_type(child.get_type().unwrap().get_canonical_type(),
                              Some(entity),
                              None) {
          Ok(field_type) => {
            fields.push(CLangClassField {
              name: child.get_name().unwrap(),
              field_type: field_type,
              is_protected: is_protected,
            });
          }
          Err(msg) => {
            log::warning(format!("Can't parse field type: {}::{}: {}",
                                 get_full_name(entity).unwrap(),
                                 child.get_name().unwrap(),
                                 msg))
          }
        };
      }
      if child.get_kind() == EntityKind::BaseSpecifier {
        let base_type = match self.parse_type(child.get_type().unwrap().get_canonical_type(),
                                              None,
                                              None) {
          Ok(r) => r,
          Err(msg) => return Err(format!("Can't parse base class type: {}", msg)),
        };
        bases.push(base_type);
      }
      if child.get_kind() == EntityKind::NonTypeTemplateParameter {
        return Err(format!("Non-type template parameter is not supported"));
      }
    }
    Ok(CLangCppTypeData {
      name: get_full_name(entity).unwrap(),
      header: include_file.clone(),
      kind: CLangCppTypeKind::Class {
        bases: bases,
        fields: fields,
        template_arguments: if entity.get_kind() == EntityKind::ClassTemplate {
          if template_arguments.is_empty() {
            panic!("missing template arguments");
          }
          Some(template_arguments)
        } else {
          if !template_arguments.is_empty() {
            panic!("unexpected template arguments");
          }
          None
        },
      },
    })
  }


  fn process_entity(&mut self, entity: Entity, context: &EntityContext) {
    let mut child_context = context.clone();
    child_context.level = child_context.level + 1;
    self.entity_kinds.insert(entity.get_kind());
    if let Some(accessibility) = entity.get_accessibility() {
      if accessibility == Accessibility::Private {
        return; // skipping private stuff
      }
    }

    let include_file = if let Some(location) = entity.get_location() {
      let file_path = location.get_presumed_location().0;
      if file_path.is_empty() {
        log::noisy(format!("empty file path: {:?}", entity.get_kind()));
        None
      } else {
        let file_path_buf = PathBuf::from(file_path.clone());
        if !file_path_buf.starts_with(&self.library_include_dir) {
          log::noisy(format!("skipping entities from {}", file_path));
          return;
        }
        let file_name = file_path_buf.strip_prefix(&self.library_include_dir)
                                     .unwrap()
                                     .to_str()
                                     .unwrap()
                                     .to_string();
        self.files.insert(file_name.clone());
        Some(file_name)
      }
    } else {
      None
    };
    if entity.get_kind() == EntityKind::Namespace {
      if entity.get_name().unwrap() == "QtPrivate" {
        return;
      }
    }
    match entity.get_kind() {
      EntityKind::FunctionDecl |
      EntityKind::Method |
      EntityKind::Constructor |
      EntityKind::Destructor |
      EntityKind::ConversionFunction |
      EntityKind::FunctionTemplate => {
        if entity.get_canonical_entity() == entity {
          self.stats.total_methods = self.stats.total_methods + 1;
          match self.parse_function(entity, &include_file) {
            Ok(r) => {
              if r.full_name() == "QSizeF::isEmpty" {
                println!("test {:?}", r);
                dump_entity(&entity, 0);
              }
              self.stats.success_methods = self.stats.success_methods + 1;
              self.data.methods.push(r);
            }
            Err(msg) => {
              self.stats.failed_methods = self.stats.failed_methods + 1;
              log::warning(format!("Failed to parse method: {}\nentity: {:?}\nerror: {}\n",
                                   get_full_name(entity).unwrap(),
                                   entity,
                                   msg));
            }
          }
        }
      }
      EntityKind::EnumDecl => {
        if entity.get_name().is_some() && entity.is_definition() {
          self.stats.total_types = self.stats.total_types + 1;
          if let Some(include_file) = include_file {
            match self.parse_enum(entity, &include_file) {
              Ok(r) => {
                self.stats.success_types = self.stats.success_types + 1;
                if self.data.types.iter().find(|x| x.name == r.name).is_some() {
                  panic!("repeating class declaration: {:?}", entity);
                }
                self.data.types.push(r);
              }
              Err(msg) => {
                self.stats.failed_types = self.stats.failed_types + 1;
                log::warning(format!("Failed to parse enum: {}\nentity: {:?}\nerror: {}\n",
                                     get_full_name(entity).unwrap(),
                                     entity,
                                     msg));
              }
            }
          } else {
            self.stats.failed_types = self.stats.failed_types + 1;
            log::warning(format!("Origin of type is unknown: {}\nentity: {:?}\n",
                                 get_full_name(entity).unwrap(),
                                 entity));
          }
        }
      }
      EntityKind::ClassDecl | EntityKind::ClassTemplate | EntityKind::StructDecl => {
        let ok = entity.get_name().is_some() && // not an anonymous struct
          entity.is_definition() && // not a forward declaration
          entity.get_template().is_none(); // not a template specialization
        if ok {
          self.stats.total_types = self.stats.total_types + 1;
          if let Some(include_file) = include_file {
            match self.parse_class(entity, &include_file) {
              Ok(r) => {
                self.stats.success_types = self.stats.success_types + 1;
                if self.data.types.iter().find(|x| x.name == r.name).is_some() {
                  panic!("repeating class declaration: {:?}", entity);
                }
                self.data.types.push(r);
              }
              Err(msg) => {
                self.stats.failed_types = self.stats.failed_types + 1;
                log::warning(format!("Failed to parse class: {}\nentity: {:?}\nerror: {}\n",
                                     get_full_name(entity).unwrap(),
                                     entity,
                                     msg));
              }
            }
          } else {
            self.stats.failed_types = self.stats.failed_types + 1;
            log::warning(format!("Origin of class is unknown: {}\nentity: {:?}\n",
                                 get_full_name(entity).unwrap(),
                                 entity));
          }
        }
      }
      _ => {}
    }
    for c in entity.get_children() {
      self.process_entity(c, &child_context);
    }
  }

  pub fn run(&mut self) {
    log::info(format!("clang version: {}", get_version()));
    log::info("Initializing clang...");
    let clang = Clang::new().unwrap_or_else(|err| panic!("clang init failed: {:?}", err));
    let index = Index::new(&clang, false, false);
    let tu = index.parser("/home/ri/tmp/1.cpp")
                  .arguments(&["-fPIC",
                               "-I",
                               "/home/ri/bin/Qt/5.5/gcc_64/include",
                               "-I",
                               "/home/ri/bin/Qt/5.5/gcc_64/include/QtCore/",
                               "-Xclang",
                               "-detailed-preprocessing-record"])
                  .parse()
                  .unwrap_or_else(|err| panic!("clang parse failed: {:?}", err));
    let translation_unit = tu.get_entity();
    assert!(translation_unit.get_kind() == EntityKind::TranslationUnit);
    if !tu.get_diagnostics().is_empty() {
      log::warning("Diagnostics:");
      for diag in tu.get_diagnostics() {
        log::warning(format!("{}", diag));
      }
    }
    log::info("Found entities:");
    self.process_entity(translation_unit, &EntityContext::new());
    self.check_integrity();
    log::info(format!("Entity kinds: {:?}", self.entity_kinds));
    log::info(format!("Files: {:?}", self.files));
    log::info(format!("{}/{} METHODS UNHARMED",
                      self.stats.success_methods,
                      self.stats.total_methods));
    log::info(format!("{}/{} METHODS DESTROYED",
                      self.stats.failed_methods,
                      self.stats.total_methods));

    log::info(format!("{}/{} TYPES UNHARMED",
                      self.stats.success_types,
                      self.stats.total_types));
    log::info(format!("{}/{} TYPES DESTROYED",
                      self.stats.failed_types,
                      self.stats.total_types));


  }

  fn check_type_integrity(&mut self, type1: &CppType) -> Result<(), String> {
    match type1.base {
      CppTypeBase::Void | CppTypeBase::BuiltInNumeric(..) => {}
      CppTypeBase::Unspecified { .. } => unreachable!(),
      CppTypeBase::Enum { ref name } => {
        if self.data.types.iter().find(|x| &x.name == name).is_none() {
          return Err(format!("unknown type: {}", name));
        }
      }
      CppTypeBase::Class { ref name, ref template_arguments } => {
        if self.data.types.iter().find(|x| &x.name == name).is_none() {
          return Err(format!("unknown type: {}", name));
        }
        if let &Some(ref args) = template_arguments {
          for arg in args {
            if let Err(msg) = self.check_type_integrity(&arg) {
              return Err(msg);
            }
          }
        }
      }
      CppTypeBase::TemplateParameter { ref index1, ref index2 } => {
        // TODO: check template parameters
      }
    }
    Ok(())
  }

  fn check_integrity(&mut self) {
    self.data.methods = self.data
                            .methods
                            .clone()
                            .into_iter()
                            .filter(|method| {
                              if let Err(msg) = self.check_type_integrity(&method.return_type
                                                                                 .clone()
                                                                                 .unwrap()) {
                                log::warning(format!("Method is removed: {}: {}",
                                                     method.short_text(),
                                                     msg));
                                return false;
                              }
                              for arg in &method.arguments {
                                if let Err(msg) = self.check_type_integrity(&arg.argument_type) {
                                  log::warning(format!("Method is removed: {}: {}",
                                                       method.short_text(),
                                                       msg));
                                  return false;
                                }
                              }
                              true
                            })
                            .collect();
    for t in self.data.types.clone() {
      if let CLangCppTypeKind::Class { bases, .. } = t.kind {
        for base in bases {
          if let Err(msg) = self.check_type_integrity(&base) {
            log::warning(format!("Class {}: base class type {:?}: {}", t.name, base, msg));
          }
        }
      }
    }
  }
}
