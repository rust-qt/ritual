extern crate select;
use self::select::document::Document;
extern crate csv;

use std::path::PathBuf;
use std::collections::HashMap;
use utils::PathBufPushTweak;
use std::fs;
use std::fs::File;
use std::io::Read;
use log;


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
  method_docs: HashMap<String, Vec<QtDocForMethod>>,
}

#[derive(Debug)]
struct QtDocForMethod {
  anchor: String,
  declarations: Vec<String>,
  text: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum QtDocResultForMethodKind {
  ExactMatch,
  Mismatch { declaration: String },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct QtDocResultForMethod {
  pub anchor: String,
  pub text: String,
  pub kind: QtDocResultForMethodKind,
}


fn arguments_from_declaration(declaration: &str) -> Option<Vec<&str>> {
  match declaration.find('(') {
    None => None,
    Some(start_index) => {
      match declaration.rfind(')') {
        None => None,
        Some(end_index) => Some(declaration[start_index + 1..end_index].split(',').collect()),
      }
    }
  }


}

fn are_argument_types_equal(declaration1: &str, declaration2: &str) -> bool {
  let args1 = match arguments_from_declaration(declaration1) {
    Some(r) => r,
    None => return false,
  };
  let args2 = match arguments_from_declaration(declaration2) {
    Some(r) => r,
    None => return false,
  };
  if args1.len() != args2.len() {
    return false;
  }
  fn arg_prepare(arg: &str) -> &str {
    let arg1 = arg.trim();
    match arg1.find('=') {
      Some(index) => arg1[0..index].trim(),
      None => arg1,
    }
  }

  fn arg_to_type(arg: &str) -> &str {
    match arg.rfind(|c: char| !c.is_alphanumeric() && c != '_') {
      Some(index) => arg[0..index + 1].trim(),
      None => arg,
    }
  }
  for i in 0..args1.len() {
    let arg1 = arg_prepare(args1[i]);
    let arg2 = arg_prepare(args2[i]);
    let arg1_maybe_type = arg_to_type(arg1.as_ref());
    let arg2_maybe_type = arg_to_type(arg2.as_ref());
    let a1_orig = arg1.replace(" ", "");
    let a1_type = arg1_maybe_type.replace(" ", "");
    let a2_orig = arg2.replace(" ", "");
    let a2_type = arg2_maybe_type.replace(" ", "");
    if a1_orig != a2_orig && a1_orig != a2_type && a1_type != a2_orig && a1_type != a2_type {
      return false;
    }
  }
  true
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
      method_docs: HashMap::new(),
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
      let doc = Document::from(html_content.as_ref());
      result.method_docs.insert(item.file_name().into_string().unwrap(),
                                QtDocData::all_method_docs(&doc));
      result.files.insert(item.file_name().into_string().unwrap(), doc);

    }
    Ok(result)
  }

  pub fn doc_for_method(&self,
                        name: &str,
                        parser_declaration: &str,
                        method_short_text: &str)
                        -> Result<QtDocResultForMethod, String> {
    let mut name_parts: Vec<_> = name.split("::").collect();
    let mut anchor_override = None;
    if name_parts.len() >= 2 &&
       name_parts[name_parts.len() - 1] == name_parts[name_parts.len() - 2] {
      anchor_override = Some(name_parts.last().unwrap().to_string());
      // constructors are not in the index
      name_parts.pop().unwrap();
    }
    if name_parts.len() == 3 {
      // nested types don't have full names in the index
      name_parts.remove(0);
    }
    let corrected_name = name_parts.join("::");
    match self.index.iter().find(|item| &item.name == &corrected_name) {
      Some(item) => {
        match self.method_docs.get(&item.file_name) {
          Some(method_docs) => {
            let anchor = match anchor_override {
              Some(ref x) => x,
              None => &item.anchor,
            };
            let anchor_prefix = format!("{}-", anchor);
            let candidates: Vec<_> = method_docs.iter()
              .filter(|x| &x.anchor == anchor || x.anchor.starts_with(&anchor_prefix))
              .collect();
            if candidates.is_empty() {
              return Err(format!("No matching anchors found for {}", name));
            }
            let scope_prefix = match name.find("::") {
              Some(index) => {
                let prefix = &name[0..index];
                Some((format!("{} ::", prefix), format!("{}::", prefix)))

              }
              None => None,
            };
            for declaration in &[parser_declaration, method_short_text] {
              let mut declaration_no_scope = declaration.to_string();
              if let Some((ref prefix1, ref prefix2)) = scope_prefix {
                declaration_no_scope = declaration_no_scope.replace(prefix1, "")
                  .replace(prefix2, "");
              }
              let mut query_imprint = declaration_no_scope.replace("Q_REQUIRED_RESULT", "")
                .replace("Q_DECL_NOTHROW", "")
                .replace("Q_DECL_CONST_FUNCTION", "")
                .replace("Q_DECL_CONSTEXPR", "")
                .replace("QT_FASTCALL", "")
                .replace("inline ", "")
                .replace("virtual ", "")
                .replace(" ", "");
              if let Some(index) = query_imprint.find("Q_DECL_NOEXCEPT_EXPR") {
                query_imprint = query_imprint[0..index].to_string();
              }
              for item in &candidates {
                for item_declaration in &item.declarations {
                  let mut item_declaration_imprint = item_declaration.replace("virtual ", "")
                    .replace(" ", "");
                  if let Some((ref prefix1, ref prefix2)) = scope_prefix {
                    item_declaration_imprint = item_declaration_imprint.replace(prefix1, "")
                      .replace(prefix2, "");
                  }
                  if &item_declaration_imprint == &query_imprint {
                    if item.text.find(|c| c != '\n').is_none() {
                      return Err("found empty documentation".to_string());
                    }
                    return Ok(QtDocResultForMethod {
                      text: item.text.clone(),
                      anchor: item.anchor.clone(),
                      kind: QtDocResultForMethodKind::ExactMatch,
                    });
                  }
                }
              }
              for item in &candidates {
                for item_declaration in &item.declarations {
                  let mut item_declaration_imprint = item_declaration.clone();
                  if let Some((ref prefix1, ref prefix2)) = scope_prefix {
                    item_declaration_imprint = item_declaration_imprint.replace(prefix1, "")
                      .replace(prefix2, "");
                  }
                  if are_argument_types_equal(&declaration_no_scope, &item_declaration_imprint) {
                    if item.text.find(|c| c != '\n').is_none() {
                      return Err("found empty documentation".to_string());
                    }
                    return Ok(QtDocResultForMethod {
                      text: item.text.clone(),
                      anchor: item.anchor.clone(),
                      kind: QtDocResultForMethodKind::ExactMatch,
                    });
                  }
                }
              }
            }
            if candidates.len() == 1 {
              log::warning(format!("\
                  Declaration mismatch ignored because there is only one method.\n\
                  Parser declaration: {}\n\
                  Short text: {}\n\
                  Doc declaration: {:?}\n",
                                   parser_declaration,
                                   method_short_text,
                                   candidates[0].declarations));

              if candidates[0].text.is_empty() {
                return Err("found empty documentation".to_string());
              }
              return Ok(QtDocResultForMethod {
                text: candidates[0].text.clone(),
                anchor: candidates[0].anchor.clone(),
                kind: QtDocResultForMethodKind::Mismatch {
                  declaration: candidates[0].declarations[0].clone(),
                },
              });
            }
            log::warning(format!("Declaration mismatch!\n\
                  Parser declaration: {}\n\
                  Short text: {}",
                                 parser_declaration,
                                 method_short_text));
            log::warning("Candidates:");
            for item in &candidates {
              log::warning(format!("  {:?}", item.declarations));
            }
            log::warning("");
            Err("Declaration mismatch".to_string())
          }
          None => Err(format!("No such file: {}", &item.file_name)),
        }
      }
      None => Err(format!("No documentation entry for {}", corrected_name)),
    }
  }


  fn all_method_docs(doc: &Document) -> Vec<QtDocForMethod> {
    let mut results = Vec::new();
    use self::select::predicate::{And, Attr, Name, Class};
    let h3s = doc.find(And(Name("h3"), Class("fn")));
    for h3 in h3s.iter() {
      let anchor = h3.find(And(Name("a"), Attr("name", ())));
      let anchor_node = match anchor.iter().next() {
        Some(r) => r,
        None => {
          log::warning("Failed to get anchor_node");
          continue;
        }
      };
      let anchor_text = anchor_node.attr("name").unwrap().to_string();
      let mut main_declaration = h3.text()
        .replace("[static]", "static")
        .replace("[protected]", "protected")
        .replace("[virtual]", "virtual")
        .replace("[signal]", "")
        .replace("[slot]", "");
      if main_declaration.find("[pure virtual]").is_some() {
        main_declaration = format!("virtual {} = 0",
                                   main_declaration.replace("[pure virtual]", ""));
      }
      let mut declarations = vec![main_declaration];
      let mut result = String::new();
      let mut node = match h3.next() {
        Some(r) => r,
        None => {
          log::warning("Failed to find element next to h3_node");
          continue;
        }
      };
      loop {
        if node.name() == Some("h3") {
          break; // end of method
        }
        if node.as_comment().is_none() {
          result.push_str(node.html().as_ref());
          for td1 in node.find(And(Name("td"), Class("memItemLeft"))).iter() {
            let declaration = format!("{} {}", td1.text(), td1.next().unwrap().text());
            declarations.push(declaration);
          }

        }
        match node.next() {
          Some(r) => node = r,
          None => break,
        }
      }
      results.push(QtDocForMethod {
        declarations: declarations,
        text: result,
        anchor: anchor_text,
      });
    }
    results
  }
}

#[test]
fn qt_doc_parser_test() {
  assert!(are_argument_types_equal(&"Q_CORE_EXPORT int qstricmp(const char *, const char *)"
                                     .to_string(),
                                   &"int qstricmp(const char * str1, const char * str2)"
                                     .to_string()));

  assert!(are_argument_types_equal(&"static void exit ( int retcode = 0 )".to_string(),
                                   &"static void exit(int returnCode = 0)".to_string()));

  assert!(are_argument_types_equal(&"static QMetaObject :: Connection connect ( const QObject * \
                                    sender , const char * signal , const QObject * receiver , \
                                    const char * member , Qt :: ConnectionType = Qt :: \
                                    AutoConnection )"
                                     .to_string(),
                                   &"static QMetaObject::Connection connect(const QObject * \
                                    sender, const char * signal, const QObject * receiver, \
                                    const char * method, Qt::ConnectionType type = \
                                    Qt::AutoConnection)"
                                     .to_string()));
}
