use rust_code_generator::rust_type_to_code;
use rust_info::{RustMethodSelfArgKind, RustMethodArgumentsVariant, RustTypeDeclaration,
                RustTypeDeclarationKind, RustMethodScope, RustEnumValue, RustMethod,
                RustMethodArguments, RustMethodDocItem};
use cpp_type::{CppType, CppTypeBase, CppTypeClassBase, CppTypeIndirection};
use string_utils::JoinWithString;
use log;

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

pub fn type_doc(type1: &RustTypeDeclaration) -> String {
  match type1.kind {
    RustTypeDeclarationKind::CppTypeWrapper { ref cpp_type_name,
                                              ref cpp_template_arguments,
                                              ref cpp_doc,
                                              .. } => {
      let cpp_type_code = CppType {
          base: CppTypeBase::Class(CppTypeClassBase {
            name: cpp_type_name.clone(),
            template_arguments: cpp_template_arguments.clone(),
          }),
          indirection: CppTypeIndirection::None,
          is_const: false,
          is_const2: false,
        }
        .to_cpp_pseudo_code();
      let mut doc = format!("C++ type: {}", wrap_inline_cpp_code(&cpp_type_code));
      if let Some(ref cpp_doc) = *cpp_doc {
        // TODO: use doc_formatter
        doc += &format!("\n\n<a href=\"{}\">C++ documentation:</a> {}",
                        cpp_doc.url,
                        wrap_cpp_doc_block(&cpp_doc.html));
      }
      doc
    }
    RustTypeDeclarationKind::MethodParametersTrait { ref method_scope, ref method_name, .. } => {
      let method_name_with_scope = match *method_scope {
        RustMethodScope::Impl { ref type_name } => {
          format!("{}::{}",
                  type_name.last_name().unwrap(),
                  method_name.last_name().unwrap())
        }
        RustMethodScope::TraitImpl { .. } => {
          panic!("TraitImpl is totally not expected here");
        }
        RustMethodScope::Free => method_name.last_name().unwrap().clone(),
      };
      let method_link = match *method_scope {
        RustMethodScope::Impl { ref type_name } => {
          format!("../struct.{}.html#method.{}",
                  type_name.last_name().unwrap(),
                  method_name.last_name().unwrap())
        }
        RustMethodScope::TraitImpl { .. } => {
          panic!("TraitImpl is totally not expected here");
        }
        RustMethodScope::Free => format!("../fn.{}.html", method_name.last_name().unwrap()),
      };
      format!("This trait represents a set of arguments accepted by [{name}]({link}) \
                      method.",
              name = method_name_with_scope,
              link = method_link)
    }
  }
}



pub fn enum_value_doc(value: &RustEnumValue) -> String {
  if value.is_dummy {
    return "This variant is added in Rust because \
            enums with one variant and C representation are not supported."
      .to_string();
  }
  if value.cpp_docs.is_empty() {
    log::warning("enum_value_doc: cpp_docs is empty");
    return String::new();
  }
  if value.cpp_docs.len() > 1 {
    let mut doc = "This variant corresponds to multiple C++ enum variants with the same value:\n\n"
      .to_string();
    for cpp_doc in &value.cpp_docs {
      doc.push_str(&format!("- {}{}\n",
                            wrap_inline_cpp_code(&format!("{} = {}",
                                                          cpp_doc.variant_name,
                                                          value.value)),
                            if let Some(ref text) = cpp_doc.doc {
                              format!(": {}", text)
                            } else {
                              String::new()
                            }));
    }
    doc
  } else {
    let cpp_doc = &value.cpp_docs[0];
    let doc_part =
      format!("C++ enum variant: {}",
              wrap_inline_cpp_code(&format!("{} = {}", cpp_doc.variant_name, value.value)));
    match cpp_doc.doc {
      Some(ref text) if !text.is_empty() => format!("{} ({})", text, doc_part),
      _ => doc_part,
    }
  }
}

pub fn method_doc(method: &RustMethod) -> String {

  let cpp_method_name = match method.arguments {
    RustMethodArguments::SingleVariant(ref v) => v.cpp_method.cpp_method.full_name(),
    RustMethodArguments::MultipleVariants { ref cpp_method_name, .. } => cpp_method_name.clone(),
  };

  let overloaded = method.docs.len() > 1 ||
                   (method.docs.len() == 1 && method.docs[0].rust_fns.len() > 1);
  let mut doc = Vec::new();
  if overloaded {
    doc.push(format!("C++ method: {}\n\n", wrap_inline_cpp_code(&cpp_method_name)));
    doc.push("This is an overloaded function. Available variants:\n\n".to_string());
  }

  let mut shown_docs = Vec::new();
  for doc_item in &method.docs {
    let ok = if let Some(ref x) = doc_item.doc {
      x.mismatched_declaration.is_none()
    } else {
      true
    };
    if ok {
      shown_docs.push(doc_item.clone())
    }
  }
  for doc_item in &method.docs {
    if let Some(ref item_doc) = doc_item.doc {
      if item_doc.mismatched_declaration.is_some() {
        let anchor = &item_doc.anchor;
        if shown_docs.iter()
          .any(|x| if let Some(ref xx) = x.doc {
            &xx.anchor == anchor
          } else {
            false
          }) {
          shown_docs.push(RustMethodDocItem { doc: None, ..doc_item.clone() });
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
                         Below is the <a href=\"{}\">C++ documentation</a> for <code>{}</code>:",
                result.url,
                declaration)
      } else {
        format!("<a href=\"{}\">C++ documentation:</a>", result.url)
      };
      doc.push(format!("{} {}", prefix, wrap_cpp_doc_block(&result.html)));
    }
  }
  doc.join("")
}
