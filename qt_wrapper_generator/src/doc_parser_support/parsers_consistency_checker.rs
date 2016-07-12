use doc_parser_support::cpp_data::DocCppData;
use doc_parser_support::cpp_type_map::DocCppTypeOrigin;
use cpp_data::{CppData, CppTypeKind, CppVisibility};
use cpp_parser::CppParserStats;
use log;
use cpp_method::CppMethodScope;
use doc_parser_support::enums::DocCppTypeKind;
use std::collections::HashMap;
extern crate core;
use self::core::ops::AddAssign;

pub fn check(result1: &CppData, result1_stats: &CppParserStats, result2: &DocCppData) {
  log::info("Checking parsers consistency...");
  let mut missing_enum_values1: HashMap<String, Vec<String>> = HashMap::new();
  let mut missing_enum_values2: HashMap<String, Vec<String>> = HashMap::new();
  let mut missing_types1 = Vec::new();
  let mut missing_types2 = Vec::new();
  for (_, ref type_info2) in &result2.types.0 {
    if let DocCppTypeOrigin::IncludeFile { ref include_file, .. } = type_info2.origin {
      let include_file2 = include_file;
      match type_info2.kind {
        // typedefs are not supposed to be in result1
        DocCppTypeKind::TypeDef { .. } |
        DocCppTypeKind::Flags { .. } |
        DocCppTypeKind::Unknown { .. } => {}
        _ => {
          if let Some(type_info1) = result1.types.iter().find(|x| x.name == type_info2.name) {
            if &type_info1.include_file != include_file2 {
              log::warning(format!("Header mismatch for {}: {} vs {}",
                                   type_info2.name,
                                   type_info1.include_file,
                                   include_file2));
            }
            match type_info2.kind {
              DocCppTypeKind::Enum { ref values } => {
                let values2 = values.iter().map(|x| x.name.clone()).collect::<Vec<_>>();
                if let CppTypeKind::Enum { ref values } = type_info1.kind {
                  let values1 = values.iter().map(|x| x.name.clone()).collect::<Vec<_>>();
                  for val1 in &values1 {
                    if values2.iter().find(|&x| x == val1).is_none() {
                      if missing_enum_values2.contains_key(&type_info2.name) {
                        missing_enum_values2.get_mut(&type_info2.name).unwrap().push(val1.clone());
                      } else {
                        missing_enum_values2.insert(type_info2.name.clone(), vec![val1.clone()]);
                      }
                    }
                  }
                  for val2 in &values2 {
                    if values1.iter().find(|&x| x == val2).is_none() {
                      if missing_enum_values1.contains_key(&type_info2.name) {
                        missing_enum_values1.get_mut(&type_info2.name).unwrap().push(val2.clone());
                      } else {
                        missing_enum_values1.insert(type_info2.name.clone(), vec![val2.clone()]);
                      }
                    }
                  }

                } else {
                  log::warning(format!("Result 1 type mismatch: {} is {:?} (result2: {:?})",
                                       type_info2.name,
                                       type_info1.kind,
                                       type_info2.kind));
                }


              }
              _ => {}
            }
          } else {
            missing_types1.push(type_info2.name.clone());
          }
        }
      }
    }
  }
  for type_info1 in &result1.types {
    if !result2.types.0.contains_key(&type_info1.name) {
      missing_types2.push(type_info1.name.clone());
    }
  }
  if !missing_types1.is_empty() {
    log::warning(format!("Result 1 lacks types: {:?}", missing_types1));
  }
  if !missing_enum_values1.is_empty() {
    log::warning(format!("Result 1 misses enum values:"));
    for (enu, values) in missing_enum_values1 {
      log::debug(format!("    {}: {:?}", enu, values));
    }
  }


  if !missing_types2.is_empty() {
    log::warning(format!("Result 2 lacks types: {:?}", missing_types2));
  }
  if !missing_enum_values2.is_empty() {
    log::warning(format!("Result 2 misses enum values:"));
    for (enu, values) in missing_enum_values2 {
      log::debug(format!("    {}: {:?}", enu, values));
    }
  }

  let mut method_counts1: HashMap<String, i32> = HashMap::new();
  let mut method_counts2: HashMap<String, i32> = HashMap::new();
  for method in &result1.methods {
    if method.visibility == CppVisibility::Private {
      continue;
    }
    let name = method.full_name();
    if !method_counts1.contains_key(&name) {
      method_counts1.insert(name.clone(), 0);
    }
    method_counts1.get_mut(&name).unwrap().add_assign(1);
  }
  for header in &result2.headers {
    for method in &header.methods {
      if method.visibility == CppVisibility::Private {
        continue;
      }
      let name = method.full_name();
      if !method_counts2.contains_key(&name) {
        method_counts2.insert(name.clone(), 0);
      }
      method_counts2.get_mut(&name).unwrap().add_assign(1);
      if method.is_signal {
        method_counts1.remove(&name);
        method_counts2.remove(&name);
      }
    }
  }
  for method in &result1.methods {
    if method.visibility == CppVisibility::Private {
      continue;
    }
    if let CppMethodScope::Class(ref class_name) = method.scope {
      for class_type in result1.types.iter().filter(|x| x.inherits(class_name)) {
        let base_method = format!("{}::{}", class_type.name, method.name);
        if method_counts2.contains_key(&base_method) {
          if !method_counts1.contains_key(&base_method) {
            method_counts1.insert(base_method.clone(), 0);
            method_counts1.get_mut(&base_method).unwrap().add_assign(1);
          }
        }
      }
    }
  }

  let mut missing_methods1 = Vec::new();
  for (method, count2) in &method_counts2 {
    let count1 = method_counts1.get(method).unwrap_or(&0).clone();
    if count1 < *count2 {
      missing_methods1.push(method.clone());
    }
  }
  if !missing_methods1.is_empty() {
    missing_methods1.sort();
    log::warning(format!("Missing methods in result 1 (total {}):",
                         missing_methods1.len()));
    for method in missing_methods1 {
      let count1 = method_counts1.get(&method).unwrap_or(&0).clone();
      let count2 = method_counts2.get(&method).unwrap_or(&0).clone();
      log::debug(format!("    {} (count: {} vs {})", method, count1, count2));
      if let Some(message) = result1_stats.method_messages.get(&method) {
        log::debug(message.as_ref());
      }
    }
  }

  let mut missing_methods2 = Vec::new();
  for (method, count1) in &method_counts1 {
    let count2 = method_counts2.get(method).unwrap_or(&0).clone();
    if count2 < *count1 {
      missing_methods2.push(method.clone());
    }
  }
  if !missing_methods2.is_empty() {
    missing_methods2.sort();
    log::warning(format!("Missing methods in result 2 (total {}):",
                         missing_methods2.len()));
    for method in missing_methods2 {
      let count1 = method_counts1.get(&method).unwrap_or(&0).clone();
      let count2 = method_counts2.get(&method).unwrap_or(&0).clone();
      log::debug(format!("    {} (count: {} vs {})", method, count1, count2));
    }
  }
}
