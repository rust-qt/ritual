use cpp_method::CppMethodDoc;
use rust_code_generator::rust_type_to_code;
use rust_info::{RustMethodSelfArgKind, RustMethodArgumentsVariant};
use string_utils::JoinWithString;

#[derive(Debug, Clone)]
pub struct DocItem {
  pub doc: Option<CppMethodDoc>,
  pub rust_fns: Vec<String>,
  pub cpp_fn: String,
}

pub fn rust_method_variant(args: &RustMethodArgumentsVariant,
                           method_name: &str,
                           self_arg_kind: RustMethodSelfArgKind,
                           crate_name: &str)
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

pub fn wrap_inline_cpp_code(code: &str) -> String {
  format!("<span style='color: green;'>```{}```</span>", code)
}

pub fn wrap_cpp_doc_block(html: &str) -> String {
  format!("<div style='border: 1px solid #5CFF95; \
                       background: #D6FFE4; padding: 16px;'>{}</div>",
          html)
}

pub fn type_doc(cpp_type_name: &str, cpp_doc: &Option<String>) -> String {
  let mut doc = format!("C++ type: {}", wrap_inline_cpp_code(cpp_type_name));
  if let Some(ref cpp_doc) = *cpp_doc {
    // TODO: use doc_formatter
    doc += &format!("\n\nC++ documentation: {}", wrap_cpp_doc_block(cpp_doc));
  }
  doc
}

pub fn method_doc(doc_items: Vec<DocItem>, cpp_method_name: &str) -> String {
  let overloaded = doc_items.len() > 1 || (doc_items.len() == 1 && doc_items[0].rust_fns.len() > 1);
  let mut doc = Vec::new();
  if overloaded {
    doc.push(format!("C++ method: {}\n\n", wrap_inline_cpp_code(cpp_method_name)));
    doc.push("This is an overloaded function. Available variants:\n\n".to_string());
  }

  let mut shown_docs = Vec::new();
  for doc_item in &doc_items {
    let ok = if let Some(ref x) = doc_item.doc {
      x.mismatched_declaration.is_none()
    } else {
      true
    };
    if ok {
      shown_docs.push(doc_item.clone())
    }
  }
  for doc_item in doc_items {
    if let Some(ref item_doc) = doc_item.doc {
      if item_doc.mismatched_declaration.is_some() {
        let anchor = &item_doc.anchor;
        if shown_docs.iter()
          .any(|x| if let Some(ref xx) = x.doc {
            &xx.anchor == anchor
          } else {
            false
          }) {
          shown_docs.push(DocItem { doc: None, ..doc_item.clone() });
        } else {
          shown_docs.push(doc_item.clone());
        }
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
                    String::new()
                  },
                  x)
        })
                         .join("")));
    }
    doc.push(format!("C++ method: {}", wrap_inline_cpp_code(&doc_item.cpp_fn)));
    doc.push("\n\n".to_string());
    // TODO: use inheritance_chain to generate documentation
    //    if let Some(ref inherited_from) = doc_item.inherited_from {
    //      doc.push(format!("Inherited from {}. Original C++ method: {}\n\n",
    //                       wrap_inline_cpp_code(&CppTypeBase::Class(inherited_from.class_type
    //                           .clone())
    //                         .to_cpp_pseudo_code()),
    //                       wrap_inline_cpp_code(&inherited_from.short_text)));
    //    }
    if let Some(result) = doc_item.doc {
      let prefix = if let Some(ref declaration) = result.mismatched_declaration {
        format!("Warning: no exact match found in C++ documentation.\
                         Below is the C++ documentation for <code>{}</code>:",
                declaration)
      } else {
        "C++ documentation:".to_string()
      };
      doc.push(format!("{} {}", prefix, wrap_cpp_doc_block(&result.text)));
    }
  }
  doc.join("")
}
