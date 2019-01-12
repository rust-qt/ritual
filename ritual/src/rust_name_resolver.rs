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

/*
struct State {
    db_items: HashMap<CppPath, CppDatabaseItem>,
    rust_names: HashSet<RustPath>,
}

impl State {
    fn new(data: &ProcessorData) -> State {
        let all_items = once(&data.current_database as &_)
            .chain(data.dep_databases.iter())
            .flat_map(|d| d.cpp_items.iter());

        let db_items = all_items
            .filter_map(|db_item| {
                let cpp_path = match db_item.cpp_data {
                    CppItemData::Namespace(ref path) => path,
                    CppItemData::Type(ref t) => &t.path,
                    _ => return None,
                };
                Some((cpp_path.clone(), db_item.clone()))
            })
            .collect();

        let local_items = data.current_database.items();

        let rust_names = local_items
            .iter()
            .filter_map(|db_item| db_item.ffi_items.as_ref()) // -> Iterator<Item=&Vec<RustItem>>
            .flatten() // -> Iterator<Item=&RustItem>
            .filter_map(|rust_item| rust_item.rust_path()) // -> Iterator<Item=&RustPath>
            .cloned() // -> Iterator<Item=RustPath>
            .collect();

        State {
            db_items,
            rust_names,
        }
    }

    fn check_type(&self, cpp_type: &CppType) -> Result<()> {
        match cpp_type {
            CppType::Class(ref path) => {
                if !self
                    .db_items
                    .get(&path)
                    .iter()
                    .filter_map(|item| item.cpp_data.as_type_ref())
                    .any(|t| t.kind.is_class())
                {
                    bail!("class not found: {}", path.to_cpp_pseudo_code());
                }

                // TODO: maybe delete?
                if let Some(ref template_arguments) = path.last().template_arguments {
                    if template_arguments
                        .iter()
                        .any(|arg| arg.is_or_contains_template_parameter())
                    {
                        bail!("template parameters are not supported");
                    }
                }
            }
            CppType::Enum { path } => {
                if !self
                    .db_items
                    .get(&path)
                    .iter()
                    .filter_map(|item| item.cpp_data.as_type_ref())
                    .any(|t| t.kind.is_enum())
                {
                    bail!("enum not found: {}", path);
                }
            }
            CppType::PointerLike { ref target, .. } => {
                self.check_type(target)?;
            }
            CppType::FunctionPointer(t) => {
                self.check_type(&t.return_type)?;

                for arg in &t.arguments {
                    self.check_type(arg)?;
                }
            }
            CppType::TemplateParameter { .. } => {
                bail!("template parameters are not supported");
            }
            _ => {}
        }
        Ok(())
    }

    fn check_cpp_item_resolvable(&self, item: &CppItemData) -> Result<()> {
        for cpp_type in &item.all_involved_types() {
            self.check_type(cpp_type)?;
        }
        Ok(())
    }

    fn give_name(&mut self, cpp_item: &CppItemData, rust_item: &mut CppFfiItem) -> Result<()> {
        unimplemented!("{:?}", (&self.rust_names, cpp_item, rust_item))
    }
}*/

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
    /*let mut state = State::new(data);

    for item in &mut data.current_database.cpp_items {
        if let Err(err) = state.check_cpp_item_resolvable(&item.cpp_data) {
            trace!("skipping item: {}: {}", &item.cpp_data, err);
            continue;
        }

        if let Some(ref mut rust_items) = item.ffi_items {
            for mut rust_item in rust_items {
                if rust_item.has_rust_path_resolved() {
                    continue;
                }

                state.give_name(&item.cpp_data, &mut rust_item)?;
            }
        }
    }*/
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

// TODO: update this
/*
#[test]
fn it_should_check_functions() {
    use crate::cpp_data::CppPath;
    use crate::cpp_data::CppTypeData;
    use crate::cpp_data::CppTypeDataKind;
    use crate::cpp_function::CppFunction;
    use crate::cpp_function::CppFunctionArgument;
    use crate::database::DatabaseItemSource;

    let func = CppFunction {
        path: CppPath::from_str_unchecked("foo"),
        member: None,
        operator: None,
        return_type: CppType::Void,
        arguments: vec![],
        allows_variadic_arguments: false,
        declaration_code: None,
        doc: None,
    };
    let func_item = DatabaseItem {
        cpp_data: CppItemData::Function(func.clone()),
        source: DatabaseItemSource::ImplicitDestructor,
        rust_items: None,
    };

    let func2_item = DatabaseItem {
        cpp_data: CppItemData::Function(CppFunction {
            arguments: vec![CppFunctionArgument {
                name: "a".to_string(),
                argument_type: CppType::Class(CppPath::from_str_unchecked("C1")),
                has_default_value: false,
            }],
            ..func
        }),
        source: DatabaseItemSource::ImplicitDestructor,
        rust_items: None,
    };
    let all_items = &[func_item.clone(), func2_item.clone()];
    assert!(check_cpp_item_resolvable(all_items, &func_item.cpp_data).is_ok());
    assert!(check_cpp_item_resolvable(all_items, &func2_item.cpp_data).is_err());

    let class_item = DatabaseItem {
        cpp_data: CppItemData::Type(CppTypeData {
            path: CppPath::from_str_unchecked("C1"),
            kind: CppTypeDataKind::Class,
            doc: None,
            is_movable: false,
        }),
        source: DatabaseItemSource::ImplicitDestructor,
        rust_items: None,
    };
    let all_items = &[func_item.clone(), func2_item.clone(), class_item];
    assert!(check_cpp_item_resolvable(all_items, &func_item.cpp_data).is_ok());
    assert!(check_cpp_item_resolvable(all_items, &func2_item.cpp_data).is_ok());
}
*/
