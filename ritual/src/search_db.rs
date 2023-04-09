use std::collections::BTreeMap;

use crate::{cpp_data::CppItem, cpp_type::CppType, database2};

fn type_matches(cpp_type: &CppType, name: &str) -> bool {
    match cpp_type {
        CppType::Void
        | CppType::BuiltInNumeric(_)
        | CppType::SpecificNumeric(_)
        | CppType::PointerSizedInteger { .. }
        | CppType::TemplateParameter(_)
        | CppType::FunctionPointer(_) => false,
        CppType::Enum { path } => path.to_templateless_string() == name,
        CppType::Class(path) => {
            path.to_templateless_string() == name
                || path.items().iter().any(|item| {
                    item.template_arguments
                        .iter()
                        .flatten()
                        .any(|t| type_matches(t, name))
                })
        }
        CppType::PointerLike { target, .. } => type_matches(target, name),
    }
}

fn item_matches(item: &CppItem, name: &str) -> bool {
    if let Some(path) = item.path() {
        if path.to_templateless_string() == name {
            return true;
        }
    }
    match item {
        CppItem::Namespace(_) | CppItem::Type(_) | CppItem::Function(_) => false,
        CppItem::EnumValue(item) => item.path.parent().unwrap().to_templateless_string() == name,
        // CppItem::Function(item) => item
        //     .all_involved_types()
        //     .iter()
        //     .any(|t| type_matches(t, name)),
        CppItem::ClassField(item) => {
            item.path.parent().unwrap().to_templateless_string() == name
                || type_matches(&item.field_type, name)
        }
        CppItem::ClassBase(item) => item.derived_class_type.to_templateless_string() == name,
    }
}

pub fn run(dbs: &[database2::Database], name: &str) {
    println!();
    println!("best matches:");
    println!();
    for db in dbs {
        for item in db.items() {
            if item_matches(&item.item, name) {
                println!("{}", item.item.short_text());
            }
        }
    }

    println!();
    println!("uses in functions:");
    println!();

    let mut categorized = BTreeMap::<_, Vec<_>>::new();
    for db in dbs {
        for item in db.items() {
            if let CppItem::Function(item) = &item.item {
                for arg in &item.arguments {
                    if type_matches(&arg.argument_type, name) {
                        categorized
                            .entry(("accepts", arg.argument_type.to_cpp_pseudo_code()))
                            .or_default()
                            .push(item.short_text());
                    }
                }
                if type_matches(&item.return_type, name) {
                    categorized
                        .entry(("returns", item.return_type.to_cpp_pseudo_code()))
                        .or_default()
                        .push(item.short_text());
                }
            }
        }
    }
    for ((cat, type_), fns) in categorized {
        println!("{} {}:", cat, type_);
        for f in fns {
            println!("    {f}");
        }
    }
}
