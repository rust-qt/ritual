use qt_doc_parser::{QtDocResultForMethod, QtDocResultForMethodKind};
use rust_info::{RustMethodSelfArgKind, RustMethodArgumentsVariant};
use rust_code_generator::rust_type_to_code;
use utils::JoinWithString;
use cpp_method::CppMethodInheritedFrom;

#[derive(Debug, Clone)]
pub struct DocItem {
  pub doc: Option<QtDocResultForMethod>,
  pub rust_fns: Vec<String>,
  pub cpp_fn: String,
  pub inherited_from: Option<CppMethodInheritedFrom>,
}

pub fn rust_method_variant(args: &RustMethodArgumentsVariant,
                           method_name: &String,
                           self_arg_kind: RustMethodSelfArgKind,
                           crate_name: &String)
                           -> String {
  let self_arg_doc_text = match self_arg_kind {
    RustMethodSelfArgKind::Static => "",
    RustMethodSelfArgKind::ConstRef => "&self, ",
    RustMethodSelfArgKind::MutRef => "&mut self, ",
    RustMethodSelfArgKind::Value => "self, ",
  };
  let return_type_text = rust_type_to_code(&args.return_type
                                             .rust_api_type,
                                           crate_name);
  let arg_texts = args.arguments
    .iter()
    .map(|x| {
      rust_type_to_code(&x.argument_type
                          .rust_api_type,
                        crate_name)
    })
    .join(", ");
  let arg_final_text = if args.arguments.len() == 1 {
    arg_texts
  } else {
    format!("({})", arg_texts)
  };
  format!("fn {name}({self_arg}{arg_text}) -> {return_type}",
          name = method_name,
          self_arg = self_arg_doc_text,
          arg_text = arg_final_text,
          return_type = return_type_text)
}

pub fn method_doc(doc_items: Vec<DocItem>, cpp_method_name: &String) -> String {
  let overloaded = doc_items.len() > 1 || (doc_items.len() == 1 && doc_items[0].rust_fns.len() > 1);
  let mut doc = Vec::new();
  if overloaded {
    doc.push(format!("C++ method: <span style='color: green;'>```{}```</span>\n\n", cpp_method_name));
    doc.push(format!("This is an overloaded function. Available variants:\n\n"));
  }

  let mut shown_docs = Vec::new();
  for doc_item in &doc_items {
    if doc_item.doc.is_none() ||
       doc_item.doc.as_ref().unwrap().kind == QtDocResultForMethodKind::ExactMatch {
      shown_docs.push(doc_item.clone());
    }
  }
  for doc_item in doc_items {
    if doc_item.doc.is_some() &&
       doc_item.doc.as_ref().unwrap().kind != QtDocResultForMethodKind::ExactMatch {
      let anchor = &doc_item.doc.as_ref().unwrap().anchor;
      if shown_docs.iter()
        .find(|x| x.doc.is_some() && &x.doc.as_ref().unwrap().anchor == anchor)
        .is_some() {
        shown_docs.push(DocItem { doc: None, ..doc_item.clone() });
      } else {
        shown_docs.push(doc_item.clone());
      }
    }
  }
  let shown_docs_count = shown_docs.len();
  for (doc_index, doc_item) in shown_docs.into_iter().enumerate() {
    if shown_docs_count > 1 {
      doc.push(format!("\n\n## Variant {}\n\n", doc_index + 1));
    }
    let rust_count = doc_item.rust_fns.len();
    if overloaded {
      doc.push(format!("Rust arguments: {}{}\n",
                       if rust_count > 1 { "<br>" } else { "" },
                       doc_item.rust_fns
                         .iter()
                         .enumerate()
                         .map(|(i, x)| {
          format!("{}```{}```<br>",
                  if rust_count > 1 {
                    format!("{}) ", i + 1)
                  } else {
                    format!("")
                  },
                  x)
        })
                         .join("")));
    }
    doc.push(format!("C++ method: <span style='color: green;'>```{}```</span>",
                     doc_item.cpp_fn));
    doc.push(format!("\n\n"));
    if let Some(ref inherited_from) = doc_item.inherited_from {
      doc.push(format!("Inherited from {}. Original C++ method: \
                        <span style='color: green;'>```{}```</span>\n\n",
                       inherited_from.class_type.to_cpp_code().unwrap(), // TODO: use permissive
                       inherited_from.short_text));
    }
    if let Some(result) = doc_item.doc {
      let prefix = match result.kind {
        QtDocResultForMethodKind::ExactMatch => format!("C++ documentation:"),
        QtDocResultForMethodKind::Mismatch { ref declaration } => {
          format!("Warning: no exact match found in C++ documentation.\
                         Below is the C++ documentation for <code>{}</code>:",
                  declaration)
        }
      };

      doc.push(format!("{} <div style='border: 1px solid #5CFF95; \
                                    background: #D6FFE4; padding: 16px;'>{}</div>",
                       prefix,
                       result.text));

    }
  }
  doc.join("")
}
