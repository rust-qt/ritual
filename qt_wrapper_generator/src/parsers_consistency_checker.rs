use cpp_data::CppData;
use clang_cpp_data::{CLangCppData, CLangCppTypeKind};
use log;
use enums::{CppTypeOrigin, CppTypeKind};
use std::collections::HashMap;

pub fn check(result1: &CLangCppData, result2: &CppData) {
  log::info("Checking parsers consistency...");
  let mut missing_enum_values1: HashMap<String, Vec<String>> = HashMap::new();
  let mut missing_enum_values2: HashMap<String, Vec<String>> = HashMap::new();
  let mut missing_types1 = Vec::new();
  let mut missing_types2 = Vec::new();
  for (_, ref type_info2) in &result2.types.0 {
    if let CppTypeOrigin::Qt { ref include_file } = type_info2.origin {
      let include_file2 = include_file;
      match type_info2.kind {
        // typedefs are not supposed to be in result1
        CppTypeKind::TypeDef { .. } | CppTypeKind::Flags { .. } | CppTypeKind::Unknown { .. } => {}
        _ => {
          if let Some(type_info1) = result1.types.iter().find(|x| x.name == type_info2.name) {
            if &type_info1.header != include_file2 {
              log::warning(format!("Header mismatch for {}: {} vs {}",
                                   type_info2.name,
                                   type_info1.header,
                                   include_file2));
            }
            match type_info2.kind {
              CppTypeKind::Enum { ref values } => {
                let values2 = values.iter().map(|x| x.name.clone()).collect::<Vec<_>>();
                if let CLangCppTypeKind::Enum { ref values } = type_info1.kind {
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
      log::warning(format!("  {}: {:?}", enu, values));
    }
  }


  if !missing_types2.is_empty() {
    log::warning(format!("Result 2 lacks types: {:?}", missing_types2));
  }
  if !missing_enum_values2.is_empty() {
    log::warning(format!("Result 2 misses enum values:"));
    for (enu, values) in missing_enum_values2 {
      log::warning(format!("  {}: {:?}", enu, values));
    }
  }
}
