extern crate clang;
use self::clang::*;

use log;
use std::collections::HashSet;
use std::path::PathBuf;
use std::ffi::OsStr;

use clang_cpp_data::CLangCppData;
use cpp_type_map::CppTypeInfo;
use cpp_method::CppMethod;
use enums::{CppMethodScope, CppTypeOrigin};

pub struct CppParser {
  library_include_dir: PathBuf,
  entity_kinds: HashSet<EntityKind>,
  files: HashSet<String>,
  data: CLangCppData,
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


fn get_full_name(entity: Entity) -> String {
  let mut current_entity = entity;
  let mut s = entity.get_name().unwrap();
  loop {
    if let Some(p) = current_entity.get_semantic_parent() {
      if p.get_kind() == EntityKind::ClassDecl || p.get_kind() == EntityKind::ClassTemplate ||
         p.get_kind() == EntityKind::StructDecl || p.get_kind() == EntityKind::Namespace ||
         p.get_kind() == EntityKind::EnumDecl || p.get_kind() == EntityKind::Method ||
         p.get_kind() == EntityKind::ClassTemplatePartialSpecialization {
        s = format!("{}::{}", p.get_name().unwrap_or("[anon]".to_string()), s);
        current_entity = p;
      } else {
        if p.get_kind() != EntityKind::TranslationUnit {
          println!("test1 {:?}", p.get_kind());
        }
        break;
      }
    }
  }
  return s;
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
    }
  }

//  fn parse_type(&mut self, type1: Type) -> Result<CppType, String> {
//    match type1.kind() {
//    }
//    Err("not implemented".to_string())
//  }

  fn parse_function(&mut self, entity: Entity) {
    log::debug(format!("Parsing function: {}", get_full_name(entity)));
    let scope = if let Some(p) = entity.get_semantic_parent() {
      match p.get_kind() {
        EntityKind::ClassDecl |
        EntityKind::ClassTemplate |
        EntityKind::StructDecl |
        EntityKind::ClassTemplatePartialSpecialization => {
          match p.get_name() {
            Some(class_name) => CppMethodScope::Class(class_name.clone()),
            None => panic!("function parent is a class but it doesn't have a name"),
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
          return;
        }
      } else {
        panic!("class method without accessibility");
      }
    }
    let return_type = entity.get_type()
                            .unwrap_or_else(|| panic!("failed to get function type"))
                            .get_result_type()
                            .unwrap_or_else(|| panic!("failed to get function return type"));
    println!("return type: {:?}", return_type);


    self.data.methods.push(CppMethod {
      name: get_full_name(entity),
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
      return_type: None, // TODO: return type
      is_constructor: entity.get_kind() == EntityKind::Constructor,
      is_destructor: entity.get_kind() == EntityKind::Destructor,
      operator: None, // TODO: operator
      is_variable: false, // TODO: move variables into CppTypeInfo
      original_index: -1,
      // TODO: move include file detection to separate function
      origin: CppTypeOrigin::CLang { include_file: String::new() },
    });
  }

  fn process_entity(&mut self, entity: Entity, context: &EntityContext) {
    let mut child_context = context.clone();
    child_context.level = child_context.level + 1;
    self.entity_kinds.insert(entity.get_kind());
    if let Some(location) = entity.get_location() {
      let file_path = location.get_presumed_location().0;
      if file_path.is_empty() {
        println!("empty file path: {:?}", entity.get_kind());
      } else {
        // println!("file path {}", file_path);
        if !PathBuf::from(file_path.clone()).starts_with(&self.library_include_dir) {
          // log::debug(format!("skipping entities from {}", file_path));
          return;
        }
        let file_name = PathBuf::from(file_path.clone())
                          .file_name()
                          .unwrap_or(OsStr::new(&file_path))
                          .to_str()
                          .unwrap()
                          .to_string();
        self.files.insert(file_name);
      }
    } //else if entity.get_kind() != EntityKind::TranslationUnit {
      // log::warning(format!("skipped: {:?} (no source file detected)", entity));
      // return;
    //}
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
        self.parse_function(entity);
      }
      _ => {}
    }
    for c in entity.get_children() {
      self.process_entity(c, &child_context);
    }
  }

  pub fn run(&mut self) {
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
    //  for entity in translation_unit.get_children() {
    //    println!("{:?}", entity);
    //  }
    println!("Entity kinds: {:?}", self.entity_kinds);
    println!("Files: {:?}", self.files);
  }
}
