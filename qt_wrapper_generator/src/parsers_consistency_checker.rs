use cpp_data::CppData;
use clang_cpp_data::CLangCppData;
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
