use crate::common::errors::{bail, Result};
use crate::cpp_checker::cpp_checker_step;
use crate::cpp_data::CppPath;
use crate::cpp_type::CppType;
use crate::database::CppItemData;
use crate::database::DatabaseItem;
use crate::database::RustItem;
use crate::processor::ProcessingStep;
use crate::processor::ProcessorData;
use crate::rust_type::RustPath;
use log::trace;
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::once;

struct State {
    db_items: HashMap<CppPath, DatabaseItem>,
    rust_names: HashSet<RustPath>,
}

impl State {
    fn new(data: &ProcessorData) -> State {
        let all_items = once(&data.current_database as &_)
            .chain(data.dep_databases.iter())
            .flat_map(|d| d.items.iter());

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
            .filter_map(|db_item| db_item.rust_items.as_ref()) // -> Iterator<Item=&Vec<RustItem>>
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

    fn give_name(&mut self, cpp_item: &CppItemData, rust_item: &mut RustItem) -> Result<()> {
        unimplemented!("{:?}", (&self.rust_names, cpp_item, rust_item))
    }
}

fn run(data: &mut ProcessorData) -> Result<()> {
    let mut state = State::new(data);

    for item in &mut data.current_database.items {
        if let Err(err) = state.check_cpp_item_resolvable(&item.cpp_data) {
            trace!("skipping item: {}: {}", &item.cpp_data, err);
            continue;
        }

        if let Some(ref mut rust_items) = item.rust_items {
            for mut rust_item in rust_items {
                if rust_item.has_rust_path_resolved() {
                    continue;
                }

                state.give_name(&item.cpp_data, &mut rust_item)?;
            }
        }
    }
    Ok(())
}

pub fn rust_name_resolver_step() -> ProcessingStep {
    // TODO: set dependencies
    ProcessingStep::new("rust_name_resolver", vec![cpp_checker_step().name], run)
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
