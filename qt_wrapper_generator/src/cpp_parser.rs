extern crate clang;
use self::clang::*;

extern crate regex;
use self::regex::Regex;

use log;
use std::collections::HashSet;
use std::path::PathBuf;
// use std::ffi::OsStr;

use clang_cpp_data::CLangCppData;
use cpp_type_map::CppTypeInfo;
use cpp_method::CppMethod;
use enums::{CppMethodScope, CppTypeOrigin, CppTypeIndirection};
use cpp_type::CppType;

#[derive(Default)]
pub struct CppParserStats {
  pub total_methods: i32,
  pub failed_methods: i32,
  pub success_methods: i32,
}




pub struct CppParser {
  library_include_dir: PathBuf,
  entity_kinds: HashSet<EntityKind>,
  files: HashSet<String>,
  data: CLangCppData,
  stats: CppParserStats
}

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

fn dump_entity(entity: &Entity, level: i32) {
  for _ in 0..level {
    print!(". ");
  }
  println!("{:?}", entity);
  for child in entity.get_children() {
    dump_entity(&child, level + 1);
  }
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
      stats: Default::default()
    }
  }

  fn parse_type(&mut self, type1: Type) -> Result<CppType, String> {
    match type1.get_kind() {
      TypeKind::Void |
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
      TypeKind::LongDouble |
      TypeKind::Enum |
      TypeKind::Record |
      TypeKind::Typedef => {
        Ok(CppType {
          base: type1.get_display_name(),
          is_const: type1.is_const_qualified(),
          indirection: CppTypeIndirection::None,
          template_arguments: None, // TODO: get template arguments
        })
      }
      TypeKind::Pointer |
      TypeKind::LValueReference |
      TypeKind::RValueReference => {
        match type1.get_pointee_type() {
          Some(pointee) => {
            match self.parse_type(pointee.get_canonical_type()) {
              Ok(result) => {
                let new_indirection = match type1.get_kind() {
                  TypeKind::Pointer => {
                    match result.indirection {
                      CppTypeIndirection::None => Ok(CppTypeIndirection::Ptr),
                      CppTypeIndirection::Ptr => Ok(CppTypeIndirection::PtrPtr),
                      CppTypeIndirection::Ref => Ok(CppTypeIndirection::PtrRef),
                      _ => {
                        Err(format!("Unsupported level of indirection: pointer to {:?}",
                                    result.indirection))
                      }
                    }
                  }
                  TypeKind::LValueReference => {
                    if result.indirection == CppTypeIndirection::None {
                      Ok(CppTypeIndirection::Ref)
                    } else {
                      Err(format!("Unsupported level of indirection: reference to {:?}",
                                  result.indirection))
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
      _ => {
        if type1.get_kind() == TypeKind::Unexposed {
          if let Some(declaration) = type1.get_declaration() {
            if declaration.get_kind() == EntityKind::ClassDecl {
              return Ok(CppType {
                base: declaration.get_display_name().unwrap(),
                is_const: type1.is_const_qualified(),
                indirection: CppTypeIndirection::None,
                template_arguments: None, /* TODO: get template arguments
                                           * TODO: (well, that's probably not gonna happen) */
              });
            }
          }
          let name = type1.get_display_name();
          let re = Regex::new(r"^(const ){0,1}(type-parameter-0-(\d+))$").unwrap();
          if let Some(matches) = re.captures(name.as_ref()) {
            // TODO: capture and use type parameter index
            return Ok(CppType {
              base: matches.at(2).unwrap().to_string(),
              is_const: type1.is_const_qualified(),
              indirection: CppTypeIndirection::None,
              template_arguments: None,
            });
          }
        }
        println!("unsupported kind of type: {:?}", type1);
        println!("canonical: {:?}", type1.get_canonical_type());
        println!("declaration: {:?}", type1.get_declaration());
        println!("template arguments: {:?}",
                 type1.get_template_argument_types());
        Err(format!("Unsupported kind of type: {:?}", type1.get_kind()))
      }
    }
  }

  fn parse_function(&mut self,
                    entity: Entity,
                    include_file: &Option<String>)
                    -> Result<CppMethod, String> {
    log::noisy(format!("Parsing function: {:?}", get_full_name(entity)));
    let scope = if let Some(p) = entity.get_semantic_parent() {
      match p.get_kind() {
        EntityKind::ClassDecl |
        EntityKind::ClassTemplate |
        EntityKind::StructDecl |
        EntityKind::ClassTemplatePartialSpecialization => {
          match get_full_name(p) {
            Ok(class_name) => CppMethodScope::Class(class_name),
            Err(msg) => {
              panic!("function parent is a class but it doesn't have a name: {}",
                     msg)
            }
          }
        }
        _ => CppMethodScope::Global,
      }
    } else {
      CppMethodScope::Global
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
    let return_type_parsed = match self.parse_type(return_type.get_canonical_type()) {
      Ok(x) => x,
      Err(msg) => {
        return Err(format!("Can't parse return type: {:?}: {}", return_type, msg));
      }
    };

    for c in entity.get_arguments().unwrap() {
      //c.get_name().unwrap_or("[no name]".to_string()),
      //c.get_type()
    }


    Ok(CppMethod {
      name: get_full_name(entity).unwrap_or_else(|_| panic!("failed to get function name")),
      scope: scope,
      is_virtual: entity.is_virtual_method(),
      is_pure_virtual: entity.is_pure_virtual_method(),
      is_const: entity.is_const_method(),
      is_static: entity.is_static_method(),
      is_protected: entity.get_accessibility().unwrap_or(Accessibility::Public) ==
                    Accessibility::Protected,
      is_signal: false, // TODO: somehow get this information
      arguments: vec![], // TODO: arguments
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
        println!("empty file path: {:?}", entity.get_kind());
        None
      } else {
        let file_path_buf = PathBuf::from(file_path.clone());
        // println!("file path {}", file_path);
        if !file_path_buf.starts_with(&self.library_include_dir) {
          log::noisy(format!("skipping entities from {}", file_path));
          return;
        }
        //        let file_name = PathBuf::from(file_path.clone())
        //                          .file_name()
        //                          .unwrap_or(OsStr::new(&file_path))
        //                          .to_str()
        //                          .unwrap()
        //                          .to_string();
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
    // else if entity.get_kind() != EntityKind::TranslationUnit {
    // log::warning(format!("skipped: {:?} (no source file detected)", entity));
    // return;
    // }
    //    if entity.get_kind() == EntityKind::InclusionDirective {
    //      println!("include: {:?}", entity);
    //      child_context.includes.push(entity.get_display_name().unwrap_or(String::new()));
    //    }
    //    if entity.get_kind() == EntityKind::ClassDecl {
    //      println!("class: {} (from {:?})",
    //               entity.get_display_name().unwrap_or(String::new()),
    //               context.includes);
    //    }
    match entity.get_kind() {
      EntityKind::FunctionDecl |
      EntityKind::Method |
      EntityKind::Constructor |
      EntityKind::Destructor => {
        self.stats.total_methods = self.stats.total_methods + 1;
        match self.parse_function(entity, &include_file) {
          Ok(r) => {
            self.stats.success_methods = self.stats.success_methods + 1;
            self.data.methods.push(r);
          }
          Err(msg) => {
            self.stats.failed_methods = self.stats.failed_methods + 1;
            log::warning(format!("Failed to parse method: {}\nentity: {:?}\nerror: {}",
                                 get_full_name(entity).unwrap(),
                                 entity,
                                 msg));
            dump_entity(&entity, 0);
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
    println!("Entity kinds: {:?}", self.entity_kinds);
    println!("Files: {:?}", self.files);
    println!("{}/{} METHODS UNHARMED", self.stats.success_methods, self.stats.total_methods);
    println!("{}/{} METHODS DESTROYED", self.stats.failed_methods, self.stats.total_methods);
  }
}
