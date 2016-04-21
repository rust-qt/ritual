extern crate clang;
use self::clang::*;

use log;
use std::collections::HashSet;
use std::path::PathBuf;
use std::ffi::OsStr;

pub struct CppParser {
  entity_kinds: HashSet<EntityKind>,
  files: HashSet<String>,
}

fn inspect_method(entity: Entity) {
  println!("{:?}", entity.get_display_name());
//  println!("children:");
//  for c in entity.get_children() {
//    println!("child {:?}", c);
//  }
  println!("type: {:?}", entity.get_type());
  println!("return type: {:?}", entity.get_type().unwrap().get_result_type());
  println!("args:");
  for c in entity.get_arguments().unwrap() {
    //println!("arg: {:?}", c);
    println!("arg: name={} type={:?}", c.get_name().unwrap_or("[no name]".to_string()), c.get_type());
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

impl CppParser {
  pub fn new() -> Self {
    CppParser {
      entity_kinds: HashSet::new(),
      files: HashSet::new(),
    }
  }
  fn process_entity(&mut self, entity: Entity, level: i32) {
    self.entity_kinds.insert(entity.get_kind());
    if let Some(loc) = entity.get_location() {
      let file_path = loc.get_presumed_location().0;
      if file_path.is_empty() {
        // println!("empty file path: {:?}", entity.get_kind());
        return;
      }
      let file_name = PathBuf::from(file_path.clone())
                        .file_name()
                        .unwrap_or(OsStr::new(&file_path))
                        .to_str()
                        .unwrap()
                        .to_string();
      if !file_name.starts_with("q") {
        // println!("skipped: {}", file_name);
        return;
      }
      self.files.insert(file_name);
    } else {
      if entity.get_kind() == EntityKind::TranslationUnit {
      } else {
        log::warning(format!("skipped: {:?} (no source file detected)", entity));
        return;
      }
    }
    if entity.get_kind() == EntityKind::Method {
      //      for _ in 0..level {
      //        print!("> ");
      //      }
      println!("{}", get_full_name(entity));
      inspect_method(entity);
      //      if let Some(p) = entity.get_lexical_parent() {
      //        println!("parent {:?}", p.get_name());
      //      }
    }
    for c in entity.get_children() {
      self.process_entity(c, level + 1);
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
                               "/home/ri/bin/Qt/5.5/gcc_64/include/QtCore/"])
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
    self.process_entity(translation_unit, 0);
    //  for entity in translation_unit.get_children() {
    //    println!("{:?}", entity);
    //  }
    println!("Entity kinds: {:?}", self.entity_kinds);
    println!("Files: {:?}", self.files);
  }
}
