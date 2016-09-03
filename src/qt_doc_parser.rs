extern crate select;
use self::select::document::Document;
extern crate csv;

use std::path::PathBuf;
use std::collections::HashMap;
use utils::PathBufPushTweak;
use std::fs;
use std::fs::File;
use std::io::Read;

#[derive(Debug)]
struct QtDocIndexItem {
  name: String,
  file_name: String,
  anchor: String,
}

impl QtDocIndexItem {
  fn from_line(line: (String, String, String)) -> QtDocIndexItem {
    QtDocIndexItem {
      name: line.0,
      file_name: line.1,
      anchor: line.2,
    }
  }
}

#[derive(Debug)]
pub struct QtDocData {
  index: Vec<QtDocIndexItem>,
  files: HashMap<String, Document>,
}

impl QtDocData {
  pub fn new(data_folder: &PathBuf) -> Result<QtDocData, String> {
    let index_path = data_folder.with_added("index.csv");
    if !index_path.exists() {
      return Err(format!("Index file not found: {}", index_path.display()));
    }
    let mut index_reader = match csv::Reader::from_file(index_path) {
      Ok(r) => r,
      Err(err) => return Err(format!("CSV reader error: {}", err)),
    };
    let mut result = QtDocData {
      index: index_reader.decode().map(|x| QtDocIndexItem::from_line(x.unwrap())).collect(),
      files: HashMap::new(),
    };
    let dir_path = data_folder.with_added("html");
    let dir_iterator = match fs::read_dir(&dir_path) {
      Ok(r) => r,
      Err(err) => return Err(format!("Failed to read directory {}: {}", dir_path.display(), err)),
    };
    for item in dir_iterator {
      let item = match item {
        Ok(r) => r,
        Err(err) => {
          return Err(format!("Failed to iterate over directory {}: {}",
                             dir_path.display(),
                             err))
        }
      };
      let file_path = item.path();
      if file_path.is_dir() {
        continue;
      }
      let mut html_file = match File::open(&file_path) {
        Ok(r) => r,
        Err(err) => return Err(format!("Failed to open file {}: {}", file_path.display(), err)),
      };
      let mut html_content = String::new();
      match html_file.read_to_string(&mut html_content) {
        Ok(_size) => {}
        Err(err) => return Err(format!("Failed to read file {}: {}", file_path.display(), err)),
      }
      result.files.insert(item.file_name().into_string().unwrap(),
                          Document::from(html_content.as_ref()));

    }
    Ok(result)
  }

  pub fn for_method(&self, name: &String) -> Result<String, String> {
    match self.index.iter().find(|item| &item.name == name) {
      Some(item) => {
        match self.files.get(&item.file_name) {
          Some(doc) => {
            use self::select::predicate::{And, Attr, Name};
            let selection = doc.find(And(Name("a"), Attr("name", item.anchor.as_ref())));
            match selection.parent()
              .iter()
              .next() {
              Some(h3) => {
                if h3.name() != Some("h3") {
                  Err(format!("Element name mismatch: {:?}", h3.name()))
                } else {
                  let mut result = String::new();
                  let mut node = match h3.next() {
                    Some(r) => r,
                    None => return Err(format!("Failed to find element next to h3")),
                  };
                  loop {
                    if node.name() == Some("h3") {
                      break; // end of method
                    }
                    if node.as_comment().is_none() {
                      result.push_str(node.html().as_ref());
                    }
                    match node.next() {
                      Some(r) => node = r,
                      None => break,
                    }
                  }
                  Ok(result)
                }
              }
              None => Err(format!("Failed to find element")),
            }

          }
          None => Err(format!("No such file: {}", &item.file_name)),
        }
      }
      None => Err(format!("No documentation entry for {}", name)),
    }
  }
}

// #[test]
// fn qt_doc_parser_test() {
//  let data = QtDocData::new(&PathBuf::from("/home/ri/rust/rust_qt/qt-doc")).unwrap();
//  println!("TEST: {:?}", data.for_method(&"QMetaType::isRegistered".to_string()));
//
// /  let doc = data.files.get(&"qmetatype.html".to_string()).unwrap();
// /  use self::select::predicate::{And, Attr, Name};
// /  println!("{:?}",
// /           doc.find(And(Name("a"), Attr("name", "isRegistered")))
// /              .parent()
// /              .iter()
// /              .next()
// /              .unwrap()
// /              .html());
//
//  assert!(false);
// }
