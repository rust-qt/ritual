use enums::CppTypeKind;

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

#[derive(Debug)]
pub struct CppTypeMap(HashMap<String, CppTypeInfo>);

impl CppTypeMap {
  fn get_info(&self, name: &String) -> Option<&CppTypeInfo> {
    if let Some(ref r) = self.value(name) {
      if let KindOfType::TypeDef { ref meaning } = r.kind {
        if let Some(ref meaning) = meaning {
          self.get_info(meaning)
        } else {
          None
        }
      } else {
        Some(r)
      }
    } else {
      None
    }
  }
}
