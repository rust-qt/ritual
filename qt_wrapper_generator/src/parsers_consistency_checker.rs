use cpp_data::CppData;
use clang_cpp_data::{CLangCppData, CLangCppTypeKind};
use log;
use enums::{CppTypeOrigin, CppTypeKind};

pub fn check(result1: &CLangCppData, result2: &CppData) {
  log::info("Checking parsers consistency...");
  for (_, ref type_info2) in &result2.types.0 {
    if let CppTypeOrigin::Qt { ref include_file } = type_info2.origin {
      match type_info2.kind {
        // typedefs are not supposed to be in result1
        CppTypeKind::TypeDef { .. } | CppTypeKind::Flags { .. } => {}
        _ => {
          if let Some(type_info1) = result1.types.iter().find(|x| x.name == type_info2.name) {
            match type_info2.kind {
              CppTypeKind::Enum { ref values } => {
                let values2 = values.iter().map(|x| x.name.clone()).collect::<Vec<_>>();
                if let CLangCppTypeKind::Enum { ref values } = type_info1.kind {
                  let values1 = values.iter().map(|x| x.name.clone()).collect::<Vec<_>>();
                  for val1 in &values1 {
                    if values2.iter().find(|&x| x == val1).is_none() {
                      log::warning(format!("Result 2 misses enum value: {}; {}", type_info2.name, val1));
                    }
                  }
                  for val2 in &values2 {
                    if values1.iter().find(|&x| x == val2).is_none() {
                      log::warning(format!("Result 1 misses enum value: {}; {}", type_info2.name, val2));
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
            log::warning(format!("Result 1 lacks type: {}", type_info2.name));
          }
        }
      }
    }
  }
  for type_info1 in &result1.types {
    if !result2.types.0.contains_key(&type_info1.name) {
      log::warning(format!("Result 2 lacks type: {}", type_info1.name));
    }
  }
}
