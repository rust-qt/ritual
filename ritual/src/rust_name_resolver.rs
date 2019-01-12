#![allow(warnings)]

use crate::config::Config;
use crate::cpp_checker::cpp_checker_step;
use crate::cpp_data::CppPath;
use crate::cpp_type::CppType;
use crate::database::CppDatabaseItem;
use crate::database::CppFfiItem;
use crate::database::CppItemData;
use crate::database::Database;
use crate::processor::ProcessingStep;
use crate::processor::ProcessorData;
use crate::rust_info::RustDatabase;
use crate::rust_info::RustDatabaseItem;
use crate::rust_info::RustItemKind;
use crate::rust_info::RustModule;
use crate::rust_info::RustPathScope;
use crate::rust_type::RustPath;
use log::trace;
use ritual_common::errors::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::once;

/// Adds "_" to a string if it is a reserved word in Rust
fn sanitize_rust_identifier(name: &str) -> String {
    match name {
        "abstract" | "alignof" | "as" | "become" | "box" | "break" | "const" | "continue"
        | "crate" | "do" | "else" | "enum" | "extern" | "false" | "final" | "fn" | "for" | "if"
        | "impl" | "in" | "let" | "loop" | "macro" | "match" | "mod" | "move" | "mut"
        | "offsetof" | "override" | "priv" | "proc" | "pub" | "pure" | "ref" | "return"
        | "Self" | "self" | "sizeof" | "static" | "struct" | "super" | "trait" | "true"
        | "type" | "typeof" | "unsafe" | "unsized" | "use" | "virtual" | "where" | "while"
        | "yield" => format!("{}_", name),
        _ => name.to_string(),
    }
}

struct State<'a> {
    dep_databases: &'a [Database],
    rust_database: &'a mut RustDatabase,
    config: &'a Config,
    cpp_path_to_index: HashMap<CppPath, usize>,
}

impl State<'_> {
    fn get_strategy(&self, parent_path: &CppPath) -> Result<RustPathScope> {
        let index = match self.cpp_path_to_index.get(parent_path) {
            Some(index) => index,
            None => unexpected!("unknown parent path: {}", parent_path),
        };

        let rust_item = self
            .rust_database
            .items
            .iter()
            .find(|item| item.cpp_item_index == *index)
            .ok_or_else(|| err_msg(format!("rust item not found for path: {:?}", parent_path)))?;

        let rust_path = rust_item.path().ok_or_else(|| {
            err_msg(format!(
                "rust item doesn't have rust path (cpp_path = {:?})",
                parent_path
            ))
        })?;

        Ok(RustPathScope {
            path: rust_path.clone(),
            prefix: None,
        })
    }

    fn default_strategy(&self) -> RustPathScope {
        RustPathScope {
            path: RustPath {
                parts: vec![self.config.crate_properties().name().into()],
            },
            prefix: None,
        }
    }

    fn generate_rust_items(
        &mut self,
        cpp_item: &mut CppDatabaseItem,
        cpp_item_index: usize,
        modified: &mut bool,
    ) -> Result<()> {
        match &cpp_item.cpp_data {
            CppItemData::Namespace(path) => {
                let strategy = if let Some(parent) = path.parent() {
                    self.get_strategy(&parent)?
                } else {
                    self.default_strategy()
                };
                assert!(path.last().template_arguments.is_none());
                let sanitized_name = sanitize_rust_identifier(&path.last().name);
                let rust_path = strategy.apply(&sanitized_name);
                if let Some(rust_item) = self.rust_database.find(&rust_path) {
                    bail!(
                        "namespace name {:?} already exists! Rust item: {:?}",
                        rust_path,
                        rust_item
                    );
                }
                let rust_item = RustDatabaseItem {
                    kind: RustItemKind::Module(RustModule {
                        path: rust_path,
                        doc: Some(format!("C++ namespace: `{}`", path.to_cpp_pseudo_code())),
                    }),
                    cpp_item_index,
                };
                self.rust_database.items.push(rust_item);
                *modified = true;
                cpp_item.is_rust_processed = true;
            }
            _ => bail!("unimplemented"),
            /*
            CppItemData::Type(t) => unimplemented!(),
            CppItemData::EnumValue(value) => unimplemented!(),
            _ => unimplemented!(),*/
        }
        Ok(())
    }
}

fn run(data: &mut ProcessorData) -> Result<()> {
    let mut state = State {
        dep_databases: data.dep_databases,
        rust_database: &mut data.current_database.rust_database,
        config: data.config,
        cpp_path_to_index: data
            .current_database
            .cpp_items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| match item.cpp_data {
                CppItemData::Namespace(ref path) => Some((path.clone(), index)),
                CppItemData::Type(ref data) => Some((data.path.clone(), index)),
                _ => None,
            })
            .collect(),
    };
    let cpp_items = &mut data.current_database.cpp_items;

    loop {
        let mut something_changed = false;

        for (index, mut cpp_item) in cpp_items.iter_mut().enumerate() {
            if cpp_item.is_rust_processed {
                continue;
            }

            let _ = state.generate_rust_items(&mut cpp_item, index, &mut something_changed);
        }

        if !something_changed {
            break;
        }
    }

    for (index, mut cpp_item) in cpp_items.iter_mut().enumerate() {
        if cpp_item.is_rust_processed {
            continue;
        }

        let err = state
            .generate_rust_items(&mut cpp_item, index, &mut true)
            .err()
            .expect("previous iteration had no success, so fail is expected");
        trace!("skipping item: {}: {}", &cpp_item.cpp_data, err);
    }
    Ok(())
}

pub fn rust_name_resolver_step() -> ProcessingStep {
    // TODO: set dependencies
    ProcessingStep::new("rust_name_resolver", vec![cpp_checker_step().name], run)
}

pub fn clear_rust_info(data: &mut ProcessorData) -> Result<()> {
    data.current_database.rust_database.items.clear();
    for item in &mut data.current_database.cpp_items {
        item.is_rust_processed = false;
        if let Some(ffi_items) = &mut item.ffi_items {
            for item in ffi_items {
                item.is_rust_processed = false;
            }
        }
    }
    Ok(())
}

pub fn clear_rust_info_step() -> ProcessingStep {
    ProcessingStep::new_custom("clear_rust_info", clear_rust_info)
}
