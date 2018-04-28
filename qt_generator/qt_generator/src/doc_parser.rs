//! HTML parsing and some workarounds
//! for reading Qt documentation.

use doc_decoder::DocData;
use std::collections::{hash_map, HashMap, HashSet};
use cpp_to_rust_generator::common::log;
use cpp_to_rust_generator::common::errors::{unexpected, ChainErr, Result};
use cpp_to_rust_generator::cpp_method::CppMethodDoc;
use cpp_to_rust_generator::cpp_data::CppTypeDoc;
use html_parser::node::Node;
use html_parser::document::Document;
use regex::Regex;

/// Documentation data for an enum variant.
#[derive(Debug, Clone)]
pub struct DocForEnumVariant {
  /// C++ name of the enum variant.
  pub name: String,
  /// HTML description.
  pub html: String,
}

/// An item of parsed document.
#[derive(Debug, Clone)]
pub struct ItemDoc {
  /// HTML link anchor of this item.
  pub anchor: String,
  /// C++ declarations in this item.
  pub declarations: Vec<String>,
  /// Documentations of enum variants in this item.
  pub enum_variants: Vec<DocForEnumVariant>,
  /// Main HTML description of this item.
  pub html: String,
  /// Absolute URLs of links found in this item.
  pub cross_references: Vec<String>,
}

/// Documentation data found in one document.
struct FileData {
  /// Content of the HTML document.
  document: Document,
  /// Virtual file name of the document.
  file_name: String,
  /// Parsed documentation items.
  item_docs: Vec<ItemDoc>,
}

/// Documentation parser.
pub struct DocParser {
  doc_data: DocData,
  file_data: HashMap<i32, FileData>,
  base_url: String,
}

impl DocParser {
  /// Creates new parser with `data`.
  pub fn new(data: DocData) -> DocParser {
    DocParser {
      doc_data: data,
      file_data: HashMap::new(),
      base_url: "http://doc.qt.io/qt-5/".to_string(),
    }
  }

  /// Parses document `doc_id` if it wasn't requested before.
  /// Returns result of parsing the document.
  fn file_data(&mut self, doc_id: i32) -> Result<&FileData> {
    if let hash_map::Entry::Vacant(entry) = self.file_data.entry(doc_id) {
      let document = self.doc_data.document(doc_id)?;
      let item_docs = all_item_docs(&document, &self.base_url)?;
      entry.insert(FileData {
        document: document,
        item_docs: item_docs,
        file_name: self.doc_data.file_name(doc_id)?,
      });
    }
    Ok(&self.file_data[&doc_id])
  }

  /// Finds documentation for a method.
  /// `name` is the fully qualified C++ name of the method.
  ///
  /// `declaration1` and `declaration2` are C++ code containing
  /// this method's signature. One of declarations usually comes from
  /// the C++ parser, and the other one is constructed based on
  /// the parsed signature data. Declarations are used to distinguish between
  /// multiple methods with the same name.
  pub fn doc_for_method(
    &mut self,
    name: &str,
    declaration1: &str,
    declaration2: &str,
  ) -> Result<CppMethodDoc> {
    let mut name_parts: Vec<_> = name.split("::").collect();
    let anchor_override = if name_parts.len() >= 2
      && name_parts[name_parts.len() - 1] == name_parts[name_parts.len() - 2]
    {
      // constructors are not in the index
      let last_part = name_parts.pop().chain_err(|| "name_parts can't be empty")?;
      Some(last_part.to_string())
    } else {
      None
    };
    if name_parts.len() == 3 {
      // nested types don't have full names in the index
      name_parts.remove(0);
    }
    let corrected_name = name_parts.join("::");
    let index_item = self
      .doc_data
      .find_index_item(|item| {
        &item.name == &corrected_name && (item.anchor.is_some() || anchor_override.is_some())
      })
      .chain_err(|| format!("No documentation entry for {}", corrected_name))?;
    let anchor = match anchor_override {
      Some(x) => x,
      None => index_item
        .anchor
        .clone()
        .chain_err(|| unexpected("anchor is expected here!"))?,
    };
    let anchor_prefix = format!("{}-", anchor);
    let base_url = self.base_url.clone();
    let file_data = self.file_data(index_item.document_id)?;
    let file_url = format!("{}{}", base_url, file_data.file_name);
    let candidates: Vec<_> = file_data
      .item_docs
      .iter()
      .filter(|x| &x.anchor == &anchor || x.anchor.starts_with(&anchor_prefix))
      .collect();
    if candidates.is_empty() {
      return Err(format!("No matching anchors found for {}", name).into());
    }
    let scope_prefix = match name.find("::") {
      Some(index) => {
        let prefix = &name[0..index];
        Some((format!("{} ::", prefix), format!("{}::", prefix)))
      }
      None => None,
    };
    for declaration in &[declaration1, declaration2] {
      let mut declaration_no_scope = declaration.to_string();
      if let Some((ref prefix1, ref prefix2)) = scope_prefix {
        declaration_no_scope = declaration_no_scope
          .replace(prefix1, "")
          .replace(prefix2, "");
      }
      let mut query_imprint = declaration_no_scope
        .replace("Q_REQUIRED_RESULT", "")
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
          let mut item_declaration_imprint =
            item_declaration.replace("virtual ", "").replace(" ", "");
          if let Some((ref prefix1, ref prefix2)) = scope_prefix {
            item_declaration_imprint = item_declaration_imprint
              .replace(prefix1, "")
              .replace(prefix2, "");
          }
          if &item_declaration_imprint == &query_imprint {
            if item.html.find(|c| c != '\n').is_none() {
              return Err("found empty documentation".into());
            }
            return Ok(CppMethodDoc {
              html: item.html.clone(),
              anchor: item.anchor.clone(),
              mismatched_declaration: None,
              url: format!("{}#{}", file_url, item.anchor),
              cross_references: item.cross_references.clone(),
            });
          }
        }
      }
      for item in &candidates {
        for item_declaration in &item.declarations {
          let mut item_declaration_imprint = item_declaration.clone();
          if let Some((ref prefix1, ref prefix2)) = scope_prefix {
            item_declaration_imprint = item_declaration_imprint
              .replace(prefix1, "")
              .replace(prefix2, "");
          }
          if are_argument_types_equal(&declaration_no_scope, &item_declaration_imprint) {
            if item.html.find(|c| c != '\n').is_none() {
              return Err("found empty documentation".into());
            }
            return Ok(CppMethodDoc {
              html: item.html.clone(),
              anchor: item.anchor.clone(),
              mismatched_declaration: None,
              url: format!("{}#{}", file_url, item.anchor),
              cross_references: item.cross_references.clone(),
            });
          }
        }
      }
    }
    if candidates.len() == 1 {
      log::llog(log::DebugQtDocDeclarations, || {
        format!(
          "\
           Declaration mismatch ignored because there is only one \
           method.\nDeclaration 1: {}\nDeclaration 2: {}\nDoc declaration: {:?}\n",
          declaration1, declaration2, candidates[0].declarations
        )
      });

      if candidates[0].html.is_empty() {
        return Err("found empty documentation".into());
      }
      return Ok(CppMethodDoc {
        html: candidates[0].html.clone(),
        anchor: candidates[0].anchor.clone(),
        url: format!("{}#{}", file_url, candidates[0].anchor),
        mismatched_declaration: Some(candidates[0].declarations[0].clone()),
        cross_references: candidates[0].cross_references.clone(),
      });
    }
    log::llog(log::DebugQtDocDeclarations, || {
      format!(
        "Declaration mismatch!\nDeclaration 1: {}\nDeclaration 2: {}",
        declaration1, declaration2
      )
    });
    log::llog(log::DebugQtDocDeclarations, || "Candidates:");
    for item in &candidates {
      log::llog(log::DebugQtDocDeclarations, || {
        format!("  {:?}", item.declarations)
      });
    }
    log::llog(log::DebugQtDocDeclarations, || "");
    Err("Declaration mismatch".into())
  }

  /// Returns documentation for C++ type `name`.
  pub fn doc_for_type(&mut self, name: &str) -> Result<(CppTypeDoc, Vec<DocForEnumVariant>)> {
    let index_item = self
      .doc_data
      .find_index_item(|item| &item.name == &name)
      .chain_err(|| format!("No documentation entry for {}", name))?;
    if let Some(ref anchor) = index_item.anchor {
      let (result, file_name) = {
        let file_data = self.file_data(index_item.document_id)?;
        let result = file_data
          .item_docs
          .iter()
          .find(|x| &x.anchor == anchor)
          .chain_err(|| format!("no such anchor: {}", anchor))?;
        (result.clone(), file_data.file_name.clone())
      };
      return Ok((
        CppTypeDoc {
          html: result.html,
          url: format!("{}{}#{}", self.base_url, file_name, anchor),
          cross_references: result.cross_references,
        },
        result.enum_variants,
      ));
    }
    let mut result = String::new();
    let mut url = self.base_url.clone();
    {
      let file_data = self
        .file_data(index_item.document_id)
        .chain_err(|| "failed to get document")?;
      url.push_str(&file_data.file_name);
      let doc = &file_data.document;
      use html_parser::predicate::{And, Class, Name};
      let div_r = doc.find(And(Name("div"), Class("descr")));
      let div = div_r.iter().next().chain_err(|| "no div.descr")?;
      let h2_r = div.find(Name("h2"));
      let h2 = h2_r.iter().next().chain_err(|| "no div.descr h2")?;
      let mut node = h2.next().chain_err(|| "no next() for div.descr h2")?;
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
    }
    let (html, cross_references) = process_html(&result, &self.base_url)?;
    Ok((
      CppTypeDoc {
        html: html,
        url: url,
        cross_references: cross_references.into_iter().collect(),
      },
      Vec::new(),
    ))
  }

  /// Marks an enum variant `full_name` as used in the `DocData` index,
  /// so that it won't be listed in unused documentation entries.
  pub fn mark_enum_variant_used(&mut self, full_name: &str) {
    if self
      .doc_data
      .find_index_item(|item| &item.name == &full_name)
      .is_none()
    {
      log::llog(log::DebugQtDoc, || {
        format!("mark_enum_variant_used failed for {}", full_name)
      });
    }
  }

  /// Lists unused documentation entries to the debug log.
  pub fn report_unused_anchors(&self) {
    let mut logger = log::default_logger();
    if !logger.is_on(log::DebugQtDoc) {
      return;
    }
    logger.log(log::DebugQtDoc, "Unused entries in Qt documentation:");
    for item in self.doc_data.index() {
      if !item.accessed {
        if let Ok(file_name) = self.doc_data.file_name(item.document_id) {
          if file_name.ends_with("-obsolete.html") || file_name.ends_with("-compat.html") {
            continue;
          }
        }
        logger.log(log::DebugQtDoc, item.name.as_str());
      }
    }
    logger.log(log::DebugQtDoc, "");
  }
}

/// Extracts portions of the declaration corresponding to the function's arguments
fn arguments_from_declaration(declaration: &str) -> Option<Vec<&str>> {
  match declaration.find('(') {
    None => None,
    Some(start_index) => match declaration.rfind(')') {
      None => None,
      Some(end_index) => Some(declaration[start_index + 1..end_index].split(',').collect()),
    },
  }
}

/// Returns true if argument types in two declarations are equal.
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

#[test]
fn qt_doc_parser_test() {
  assert!(are_argument_types_equal(
    &"Q_CORE_EXPORT int qstricmp(const char *, const char *)".to_string(),
    &"int qstricmp(const char * str1, const char * str2)".to_string(),
  ));

  assert!(are_argument_types_equal(
    &"static void exit ( int retcode = 0 )".to_string(),
    &"static void exit(int returnCode = 0)".to_string(),
  ));

  assert!(are_argument_types_equal(
    &"static QMetaObject :: Connection connect ( const QObject * \
      sender , const char * signal , const QObject * receiver , \
      const char * member , Qt :: ConnectionType = Qt :: \
      AutoConnection )"
      .to_string(),
    &"static QMetaObject::Connection connect(const QObject * \
      sender, const char * signal, const QObject * receiver, \
      const char * method, Qt::ConnectionType type = \
      Qt::AutoConnection)"
      .to_string(),
  ));
}

/// Returns a copy of `html` with all relative link URLs replaced with absolute URLs.
/// Also returns the set of absolute URLs.
fn process_html(html: &str, base_url: &str) -> Result<(String, HashSet<String>)> {
  let bad_subfolder_regex = Regex::new(r"^\.\./qt[^/]+/").chain_err(|| "invalid regex")?;

  let link_regex = Regex::new("(href|src)=\"([^\"]*)\"").chain_err(|| "invalid regex")?;
  let mut cross_references = HashSet::new();
  let html = link_regex.replace_all(html.trim(), |captures: &::regex::Captures| {
    let mut link = bad_subfolder_regex.replace(&captures[2], "");
    if !link.contains(":") {
      link = format!("{}{}", base_url, link);
      cross_references.insert(link.clone());
    }
    format!("{}=\"{}\"", &captures[1], link)
  });
  Ok((html, cross_references))
}

/// Parses document to a list of `ItemDoc`s.
fn all_item_docs(doc: &Document, base_url: &str) -> Result<Vec<ItemDoc>> {
  let mut results = Vec::new();
  use html_parser::predicate::{And, Attr, Class, Name, Or};
  let h3s = doc.find(And(Name("h3"), Or(Class("fn"), Class("flags"))));
  for h3 in h3s.iter() {
    let anchor = h3.find(And(Name("a"), Attr("name", ())));
    let anchor_node = if let Some(r) = anchor.iter().next() {
      r
    } else {
      log::llog(log::DebugGeneral, || "Failed to get anchor_node");
      continue;
    };
    let anchor_text = anchor_node
      .attr("name")
      .chain_err(|| "anchor_node doesn't have name attribute")?
      .to_string();
    let mut main_declaration = h3.text()
      .replace("[static]", "static")
      .replace("[protected]", "protected")
      .replace("[virtual]", "virtual")
      .replace("[signal]", "")
      .replace("[slot]", "");
    if main_declaration.find("[pure virtual]").is_some() {
      main_declaration = format!(
        "virtual {} = 0",
        main_declaration.replace("[pure virtual]", "")
      );
    }
    let mut declarations = vec![main_declaration];
    let mut result = String::new();
    let mut node = if let Some(r) = h3.next() {
      r
    } else {
      log::llog(log::DebugGeneral, || {
        "Failed to find element next to h3_node"
      });
      continue;
    };
    let mut enum_variants = Vec::new();
    let mut all_cross_references = HashSet::new();
    loop {
      if node.name() == Some("h3") {
        break; // end of method
      }
      if node.as_comment().is_none() {
        let value_list_condition = And(Name("table"), Class("valuelist"));
        let mut parse_enum_variants = |value_list: Node| {
          for tr in value_list.find(Name("tr")).iter() {
            let td_r = tr.find(Name("td"));
            let tds: Vec<_> = td_r.iter().collect();
            if tds.len() == 3 {
              let name_orig = tds[0].text();
              let mut name = name_orig.trim();
              if let Some(i) = name.rfind("::") {
                name = &name[i + 2..];
              }
              let (html, cross_references) = process_html(&tds[2].inner_html(), base_url).unwrap();
              all_cross_references.extend(cross_references.into_iter());
              enum_variants.push(DocForEnumVariant {
                name: name.to_string(),
                html: html,
              });
            }
          }
        };
        let value_list_r = node.find(value_list_condition.clone());
        if node.is(value_list_condition) {
          parse_enum_variants(node);
        } else if let Some(value_list) = value_list_r.iter().next() {
          parse_enum_variants(value_list);
        } else {
          result.push_str(node.html().as_ref());
          for td1 in node.find(And(Name("td"), Class("memItemLeft"))).iter() {
            let td2 = td1.next().chain_err(|| "td1.next() failed")?;
            let declaration = format!("{} {}", td1.text(), td2.text());
            declarations.push(declaration);
          }
        }
      }
      match node.next() {
        Some(r) => node = r,
        None => break,
      }
    }
    let (html, cross_references) = process_html(&result, base_url)?;
    all_cross_references.extend(cross_references.into_iter());
    results.push(ItemDoc {
      declarations: declarations,
      html: html,
      anchor: anchor_text,
      enum_variants: enum_variants,
      cross_references: all_cross_references.into_iter().collect(),
    });
  }
  Ok(results)
}
