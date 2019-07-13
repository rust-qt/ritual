//! HTML parsing and some workarounds
//! for reading Qt documentation.

use crate::doc_decoder::DocData;
use itertools::Itertools;
use log::{debug, error, trace};
use regex::Regex;
use ritual::cpp_data::{CppItem, CppPath, CppTypeDoc, CppVisibility};
use ritual::cpp_function::CppFunctionExternalDoc;
use ritual::database::CppDatabaseItem;
use ritual::processor::ProcessorData;
use ritual_common::errors::{bail, err_msg, format_err, Result, ResultExt};
use select::document::Document;
use select::node::Node;
use std::collections::{hash_map, HashMap, HashSet};
use std::path::Path;

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

// TODO: support public field docs

#[derive(Debug)]
struct DocForType {
    type_doc: CppTypeDoc,
    enum_variants_doc: Vec<DocForEnumVariant>,
}

impl DocParser {
    /// Creates new parser with `data`.
    pub fn new(data: DocData) -> Self {
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
                document,
                item_docs,
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
    #[allow(clippy::cognitive_complexity)]
    fn doc_for_method(
        &mut self,
        name: &str,
        declaration1: &str,
        declaration2: &str,
    ) -> Result<CppFunctionExternalDoc> {
        let mut name_parts = name.split("::").collect_vec();
        let anchor_override = if name_parts.len() >= 2
            && name_parts[name_parts.len() - 1] == name_parts[name_parts.len() - 2]
        {
            // constructors are not in the index
            let last_part = name_parts
                .pop()
                .ok_or_else(|| err_msg("name_parts can't be empty"))?;
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
                item.name == corrected_name && (item.anchor.is_some() || anchor_override.is_some())
            })
            .ok_or_else(|| format_err!("No documentation entry for {}", corrected_name))?;
        let anchor = match anchor_override {
            Some(x) => x,
            None => match &index_item.anchor {
                Some(anchor) => anchor.clone(),
                None => bail!("anchor is expected here!"),
            },
        };
        let anchor_prefix = format!("{}-", anchor);
        let base_url = self.base_url.clone();
        let file_data = self.file_data(index_item.document_id)?;
        let file_url = format!("{}{}", base_url, file_data.file_name);
        let candidates = file_data
            .item_docs
            .iter()
            .filter(|x| x.anchor == anchor || x.anchor.starts_with(&anchor_prefix))
            .collect_vec();
        if candidates.is_empty() {
            trace!(
                "failed searching for anchor: {}; prefix: {}",
                anchor,
                anchor_prefix
            );
            trace!(
                "available anchors: {:?}",
                file_data.item_docs.iter().map(|x| &x.anchor).collect_vec()
            );
            bail!("No matching anchors found for {}", name);
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
            if let Some((prefix1, prefix2)) = &scope_prefix {
                declaration_no_scope = declaration_no_scope
                    .replace(prefix1, "")
                    .replace(prefix2, "");
            }
            let mut query_imprint = declaration_no_scope.clone();
            for text in &[
                "Q_REQUIRED_RESULT",
                "Q_DECL_NOTHROW",
                "Q_DECL_CONST_FUNCTION",
                "Q_DECL_CONSTEXPR",
                "QT_FASTCALL",
                "Q_DECL_COLD_FUNCTION",
                "inline ",
                "virtual ",
                " ",
            ] {
                query_imprint = query_imprint.replace(text, "");
            }
            for text in &["Q_DECL_NOEXCEPT_EXPR", "Q_ATTRIBUTE_FORMAT_PRINTF"] {
                if let Some(index) = query_imprint.find(text) {
                    query_imprint = query_imprint[..index].to_string();
                }
            }
            trace!("query_imprint = {:?}", query_imprint);
            for item in &candidates {
                for item_declaration in &item.declarations {
                    let mut item_declaration_imprint =
                        item_declaration.replace("virtual ", "").replace(" ", "");
                    if let Some((prefix1, prefix2)) = &scope_prefix {
                        item_declaration_imprint = item_declaration_imprint
                            .replace(prefix1, "")
                            .replace(prefix2, "");
                    }
                    trace!("item_declaration_imprint = {:?}", item_declaration_imprint);
                    if item_declaration_imprint == query_imprint {
                        if item.html.find(|c| c != '\n').is_none() {
                            bail!("found empty documentation");
                        }
                        return Ok(CppFunctionExternalDoc {
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
                    if let Some((prefix1, prefix2)) = &scope_prefix {
                        item_declaration_imprint = item_declaration_imprint
                            .replace(prefix1, "")
                            .replace(prefix2, "");
                    }
                    if are_argument_types_equal(&declaration_no_scope, &item_declaration_imprint) {
                        if item.html.find(|c| c != '\n').is_none() {
                            bail!("found empty documentation");
                        }
                        return Ok(CppFunctionExternalDoc {
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
            trace!(
                "Declaration mismatch ignored because there is only one \
                 method.\nDeclaration 1: {}\nDeclaration 2: {}\nDoc declaration: {:?}\n",
                declaration1,
                declaration2,
                candidates[0].declarations
            );

            if candidates[0].html.is_empty() {
                bail!("found empty documentation");
            }
            return Ok(CppFunctionExternalDoc {
                html: candidates[0].html.clone(),
                anchor: candidates[0].anchor.clone(),
                url: format!("{}#{}", file_url, candidates[0].anchor),
                mismatched_declaration: Some(candidates[0].declarations[0].clone()),
                cross_references: candidates[0].cross_references.clone(),
            });
        }
        trace!("Declaration mismatch! Declaration 1: {}", declaration1);
        trace!("Declaration 2: {}", declaration2);
        trace!("Candidates:");
        for item in &candidates {
            trace!("* {:?}", item.declarations);
        }
        bail!("Declaration mismatch");
    }

    /// Returns documentation for C++ type `name`.
    fn doc_for_type(&mut self, path: &CppPath) -> Result<DocForType> {
        let name = path.doc_id();
        let index_item = self
            .doc_data
            .find_index_item(|item| item.name == name)
            .ok_or_else(|| format_err!("No documentation entry for {}", name))?;
        if let Some(anchor) = &index_item.anchor {
            let (result, file_name) = {
                let file_data = self.file_data(index_item.document_id)?;
                let result = file_data
                    .item_docs
                    .iter()
                    .find(|x| &x.anchor == anchor)
                    .ok_or_else(|| format_err!("no such anchor: {}", anchor))?;
                (result.clone(), file_data.file_name.clone())
            };
            return Ok(DocForType {
                type_doc: CppTypeDoc {
                    html: result.html,
                    url: format!("{}{}#{}", self.base_url, file_name, anchor),
                    cross_references: result.cross_references,
                },
                enum_variants_doc: result.enum_variants,
            });
        }
        let mut result = String::new();
        let mut url = self.base_url.clone();
        {
            let file_data = self
                .file_data(index_item.document_id)
                .with_context(|_| "failed to get document")?;
            url.push_str(&file_data.file_name);
            let doc = &file_data.document;

            use select::predicate::{And, Class, Name};

            let mut div_r = doc.find(And(Name("div"), Class("descr")));
            let div = div_r.next().ok_or_else(|| err_msg("no div.descr"))?;
            let mut h2_r = div.find(Name("h2"));
            let h2 = h2_r.next().ok_or_else(|| err_msg("no div.descr h2"))?;
            let mut node = h2
                .next()
                .ok_or_else(|| err_msg("no next() for div.descr h2"))?;
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
        Ok(DocForType {
            type_doc: CppTypeDoc {
                html,
                url,
                cross_references: cross_references.into_iter().collect(),
            },
            enum_variants_doc: Vec::new(),
        })
    }

    /// Marks an enum variant `full_name` as used in the `DocData` index,
    /// so that it won't be listed in unused documentation entries.
    pub fn mark_enum_variant_used(&mut self, full_name: &str) {
        if self
            .doc_data
            .find_index_item(|item| item.name == full_name)
            .is_none()
        {
            trace!("mark_enum_variant_used failed for {}", full_name);
        }
    }

    /// Lists unused documentation entries to the debug log.
    pub fn report_unused_anchors(&self) {
        trace!("Unused entries in Qt documentation:");
        for item in self.doc_data.index() {
            if !item.accessed {
                if let Ok(file_name) = self.doc_data.file_name(item.document_id) {
                    if file_name.ends_with("-obsolete.html") || file_name.ends_with("-compat.html")
                    {
                        continue;
                    }
                }
                trace!("* {}", item.name);
            }
        }
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
    fn arg_prepare(arg: &str) -> &str {
        let arg1 = arg.trim();
        match arg1.find('=') {
            Some(index) => arg1[0..index].trim(),
            None => arg1,
        }
    }

    fn arg_to_type(arg: &str) -> &str {
        match arg.rfind(|c: char| !c.is_alphanumeric() && c != '_') {
            Some(index) => arg[0..=index].trim(),
            None => arg,
        }
    }

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

    for i in 0..args1.len() {
        let arg1 = arg_prepare(args1[i]);
        let arg2 = arg_prepare(args2[i]);
        let arg1_maybe_type = arg_to_type(arg1);
        let arg2_maybe_type = arg_to_type(arg2);
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
    let bad_subfolder_regex = Regex::new(r"^\.\./qt[^/]+/").with_context(|_| "invalid regex")?;

    let link_regex = Regex::new("(href|src)=\"([^\"]*)\"").with_context(|_| "invalid regex")?;
    let mut cross_references = HashSet::new();
    let html = link_regex
        .replace_all(html.trim(), |captures: &::regex::Captures<'_>| {
            let mut link = bad_subfolder_regex.replace(&captures[2], "").to_string();
            if !link.contains(':') {
                link = format!("{}{}", base_url, link);
                cross_references.insert(link.clone());
            }
            format!("{}=\"{}\"", &captures[1], link)
        })
        .to_string();
    Ok((html, cross_references))
}

fn reformat_qt_declaration(declaration: &str) -> String {
    let mut declaration = declaration
        .replace("[static]", "static")
        .replace("[protected]", "protected")
        .replace("[virtual]", "virtual")
        .replace("[signal]", "")
        .replace("[slot]", "");
    if declaration.find("[pure virtual]").is_some() {
        declaration = format!("virtual {} = 0", declaration.replace("[pure virtual]", ""));
    }
    declaration
}

fn parse_item_doc(h3: Node<'_>, base_url: &str) -> Result<ItemDoc> {
    use select::predicate::{And, Attr, Class, Name};

    let mut anchor = h3.find(And(Name("a"), Attr("name", ())));
    let anchor_node = if let Some(r) = anchor.next() {
        r
    } else {
        bail!("Failed to get anchor_node");
    };
    let anchor_text = anchor_node
        .attr("name")
        .ok_or_else(|| err_msg("anchor_node doesn't have name attribute"))?
        .to_string();
    let main_declaration = reformat_qt_declaration(&h3.text());
    let mut declarations = vec![main_declaration];
    let mut result = String::new();
    let mut node = if let Some(r) = h3.next() {
        r
    } else {
        bail!("Failed to find element next to h3_node");
    };
    let mut enum_variants = Vec::new();
    let mut all_cross_references = HashSet::new();
    let mut pending_wide_enum_variant: Option<(String, String)> = None;
    let value_list_condition = And(Name("table"), Class("valuelist"));
    loop {
        if node.name() == Some("h3") {
            if let Some(pending_wide_enum_variant) = pending_wide_enum_variant.take() {
                if !pending_wide_enum_variant.1.is_empty() {
                    let (html, cross_references) =
                        process_html(&pending_wide_enum_variant.1, base_url).unwrap();
                    all_cross_references.extend(cross_references.into_iter());
                    enum_variants.push(DocForEnumVariant {
                        name: pending_wide_enum_variant.0,
                        html,
                    });
                }
            }
            break; // end of method
        }
        if node.as_comment().is_none() {
            let value_list_node = if node.is(value_list_condition) {
                Some(node)
            } else {
                let mut value_list_r = node.find(value_list_condition);
                if let Some(value_list) = value_list_r.next() {
                    Some(value_list)
                } else {
                    None
                }
            };

            if let Some(value_list_node) = value_list_node {
                if let Some(pending_wide_enum_variant) = pending_wide_enum_variant.take() {
                    if !pending_wide_enum_variant.1.is_empty() {
                        let (html, cross_references) =
                            process_html(&pending_wide_enum_variant.1, base_url).unwrap();
                        all_cross_references.extend(cross_references.into_iter());
                        enum_variants.push(DocForEnumVariant {
                            name: pending_wide_enum_variant.0,
                            html,
                        });
                    }
                }
                let tr_count = value_list_node.find(Name("tr")).count();
                for tr in value_list_node.find(Name("tr")) {
                    let td_r = tr.find(Name("td"));
                    let tds = td_r.collect_vec();
                    if tds.len() == 3 || (tds.len() == 2 && tr_count == 2) {
                        let name_orig = tds[0].text();
                        let mut name = name_orig.trim();
                        if let Some(i) = name.rfind("::") {
                            name = &name[i + 2..];
                        }
                        if tds.len() == 3 {
                            let (html, cross_references) =
                                process_html(&tds[2].inner_html(), base_url).unwrap();
                            all_cross_references.extend(cross_references.into_iter());
                            enum_variants.push(DocForEnumVariant {
                                name: name.to_string(),
                                html,
                            });
                        } else {
                            // tds.len() == 2 && tr_count == 1
                            pending_wide_enum_variant = Some((name.to_string(), String::new()));
                        }
                    }
                }
            } else {
                if let Some(pending_wide_enum_variant) = &mut pending_wide_enum_variant {
                    pending_wide_enum_variant.1.push_str(&node.html());
                } else {
                    result.push_str(&node.html());
                }
                for td1 in node.find(And(Name("td"), Class("memItemLeft"))) {
                    let td2 = td1.next().ok_or_else(|| err_msg("td1.next() failed"))?;
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
    Ok(ItemDoc {
        declarations,
        html,
        anchor: anchor_text,
        enum_variants,
        cross_references: all_cross_references.into_iter().collect(),
    })
}

/// Parses document to a list of `ItemDoc`s.
fn all_item_docs(doc: &Document, base_url: &str) -> Result<Vec<ItemDoc>> {
    use select::predicate::{And, Class, Name, Or};

    let mut results = Vec::new();
    let h3s = doc.find(And(Name("h3"), Or(Class("fn"), Class("flags"))));
    for h3 in h3s {
        trace!("parsing item doc: {}", h3.html());
        match parse_item_doc(h3, base_url) {
            Ok(v) => results.push(v),
            Err(err) => debug!("parse_item_doc failed: {}", err),
        }
    }
    Ok(results)
}

/// Adds documentation from `data` to `cpp_methods`.
fn find_methods_docs(items: &mut [CppDatabaseItem], data: &mut DocParser) -> Result<()> {
    for item in items {
        if !item.source.is_parser() {
            continue;
        }
        if let CppItem::Function(cpp_method) = &mut item.item {
            if let Some(info) = &cpp_method.member {
                if info.visibility == CppVisibility::Private {
                    continue;
                }
            }
            if let Some(declaration_code) = &cpp_method.declaration_code {
                match data.doc_for_method(
                    &cpp_method.path.doc_id(),
                    declaration_code,
                    &cpp_method.pseudo_declaration(),
                ) {
                    Ok(doc) => cpp_method.doc.external_doc = Some(doc),
                    Err(msg) => {
                        if cpp_method.member.is_some()
                            && (cpp_method.path.last().name == "tr"
                                || cpp_method.path.last().name == "trUtf8"
                                || cpp_method.path.last().name == "metaObject")
                        {
                            // no error message
                            // TODO: add docs from `QObject::*` for these methods
                        } else {
                            let templateless_path = cpp_method.path.to_templateless_string();
                            // undocumented but probably useful
                            let suppressed = [
                                // checks if Qt build is shared (?)
                                "qSharedBuild",
                            ];
                            if !suppressed.contains(&templateless_path.as_str()) {
                                trace!(
                                    "Failed to get Qt documentation for method: {}: {}",
                                    &cpp_method.short_text(),
                                    msg
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn parse_docs(
    data: &mut ProcessorData<'_>,
    qt_crate_name: &str,
    docs_path: &Path,
) -> Result<()> {
    // TODO: only run on new database items?
    let doc_data = match DocData::new(&qt_crate_name, &docs_path) {
        Ok(doc_data) => doc_data,
        Err(err) => {
            error!("Failed to get Qt documentation: {}", err);
            return Ok(());
        }
    };
    let mut parser = DocParser::new(doc_data);
    find_methods_docs(data.current_database.cpp_items_mut(), &mut parser)?;
    let mut type_doc_cache = HashMap::new();
    for item in data.current_database.cpp_items_mut() {
        if !item.source.is_parser() {
            continue;
        }
        let type_name = match &item.item {
            CppItem::Type(data) => data.path.clone(),
            CppItem::EnumValue(data) => data
                .path
                .parent()
                .expect("enum value must have parent path"),
            _ => continue,
        };
        if !type_doc_cache.contains_key(&type_name) {
            let doc = parser.doc_for_type(&type_name);
            if let Err(err) = &doc {
                trace!(
                    "Failed to get Qt documentation for type: {}: {}",
                    type_name.to_cpp_pseudo_code(),
                    err
                );
            }
            type_doc_cache.insert(type_name.clone(), doc);
        }
        let doc = type_doc_cache
            .get(&type_name)
            .expect("type_doc_cache is guaranteed to have an entry here because we added it above");
        if let Ok(doc) = doc {
            match &mut item.item {
                CppItem::Type(data) => {
                    data.doc = Some(doc.type_doc.clone());
                }
                CppItem::EnumValue(data) => {
                    if let Some(r) = doc
                        .enum_variants_doc
                        .iter()
                        .find(|x| x.name == data.path.last().name)
                    {
                        trace!(
                            "Got Qt documentation for enum variant: {}: {}",
                            data.path.to_cpp_pseudo_code(),
                            r.html,
                        );
                        data.doc = Some(r.html.clone());
                        parser.mark_enum_variant_used(&data.unscoped_path().doc_id());
                    } else {
                        debug!(
                            "Failed to get Qt documentation for enum variant: {}",
                            data.path.to_cpp_pseudo_code()
                        );
                        //trace!("type data: {:?}", doc);
                    }
                }
                _ => unreachable!(),
            };
        }
    }
    parser.report_unused_anchors();
    Ok(())
}
