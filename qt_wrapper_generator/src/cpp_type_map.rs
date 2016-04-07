use enums::{CppTypeKind, CppTypeOrigin};

use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumValue {
  pub name: String,
  pub value: String,
  pub description: String,
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppTypeInfo {
  pub name: String,
  pub origin: CppTypeOrigin,
  pub kind: CppTypeKind,
}

#[derive(Debug, Clone)]
pub struct CppTypeMap(pub HashMap<String, CppTypeInfo>);

impl CppTypeMap {
  pub fn get_info(&self, name: &String) -> Result<&CppTypeInfo, String> {
    if let Some(ref r) = self.0.get(name) {
      if let CppTypeKind::TypeDef { ref meaning } = r.kind {
        if meaning.is_template() {
          Err("Template typedefs are not supported".to_string())
        } else {
          self.get_info(&meaning.base)
        }
      } else {
        Ok(r)
      }
    } else {
      Err("No type info".to_string())
    }
  }

  pub fn get_types_from_include_file(&self, include: &String) -> Vec<String> {
    let mut r = vec![];
    for (_, type_info) in &self.0 {
      if let CppTypeOrigin::Qt{ ref include_file } = type_info.origin {
        if include_file == include {
          r.push(type_info.name.clone());
        }
      }
    }
    r
  }
}
