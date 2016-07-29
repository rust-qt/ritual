
use cpp_method::{CppMethod, CppMethodScope, CppMethodKind};
use cpp_type::{CppType, CppTypeBase};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct EnumValue {
  pub name: String,
  pub value: i64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct CppClassField {
  pub name: String,
  pub field_type: CppType,
  pub visibility: CppVisibility,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub enum CppTypeKind {
  Enum {
    values: Vec<EnumValue>,
  },
  Class {
    size: Option<i32>,
    bases: Vec<CppType>,
    fields: Vec<CppClassField>,
    template_arguments: Option<Vec<String>>,
  },
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct CppOriginLocation {
  pub include_file_path: String,
  pub line: u32,
  pub column: u32,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub enum CppVisibility {
  Public,
  Protected,
  Private,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct CppTypeData {
  pub name: String,
  pub include_file: String,
  pub origin_location: CppOriginLocation,
  pub kind: CppTypeKind,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[derive(Serialize, Deserialize)]
pub struct CppData {
  pub types: Vec<CppTypeData>,
  pub methods: Vec<CppMethod>,
  pub template_instantiations: HashMap<String, Vec<Vec<CppType>>>,
}

impl CppTypeData {
  pub fn is_class(&self) -> bool {
    match self.kind {
      CppTypeKind::Class { .. } => true,
      _ => false,
    }
  }

  #[allow(dead_code)]
  pub fn inherits(&self, class_name: &String) -> bool {
    if let CppTypeKind::Class { ref bases, .. } = self.kind {
      for base in bases {
        if let CppTypeBase::Class { ref name, .. } = base.base {
          if name == class_name {
            return true;
          }
        }
      }
    }
    false
  }
}

impl CppData {
  pub fn ensure_explicit_destructors(&mut self) {
    for type1 in &self.types {
      if let CppTypeKind::Class { .. } = type1.kind {
        let class_name = &type1.name;
        let mut found_destructor = false;
        for method in &self.methods {
          if method.kind == CppMethodKind::Destructor {
            if let CppMethodScope::Class(ref name) = method.scope {
              if name == class_name {
                found_destructor = true;
                break;
              }
            }
          }
        }
        if !found_destructor {
          self.methods.push(CppMethod {
            name: format!("~{}", class_name),
            scope: CppMethodScope::Class(class_name.clone()),
            is_virtual: false, // TODO: destructors may be virtual
            is_pure_virtual: false,
            is_const: false,
            is_static: false,
            visibility: CppVisibility::Public,
            is_signal: false,
            return_type: None,
            kind: CppMethodKind::Destructor,
            arguments: vec![],
            allows_variable_arguments: false,
            include_file: type1.include_file.clone(),
            origin_location: None,
            template_arguments: None,
          });
        }
      }
    }
  }

  pub fn split_by_headers(&self) -> HashMap<String, CppData> {
    let mut result = HashMap::new();
    for method in &self.methods {
      if !result.contains_key(&method.include_file) {
        result.insert(method.include_file.clone(), CppData::default());
      }
      result.get_mut(&method.include_file).unwrap().methods.push(method.clone());
    }
    for tp in &self.types {
      if !result.contains_key(&tp.include_file) {
        result.insert(tp.include_file.clone(), CppData::default());
      }
      result.get_mut(&tp.include_file).unwrap().types.push(tp.clone());
      if let CppTypeKind::Class { .. } = tp.kind {
        if let Some(ins) = self.template_instantiations.get(&tp.name) {
          result.get_mut(&tp.include_file)
                .unwrap()
                .template_instantiations
                .insert(tp.name.clone(), ins.clone());
        }
      }
    }
    result
  }
}
