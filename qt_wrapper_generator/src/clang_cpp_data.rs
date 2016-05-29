
// use cpp_type_map::CppTypeInfo;
use cpp_method::CppMethod;
use cpp_type::{CppType, CppTypeBase};
use cpp_type_map::EnumValue;
use std::collections::HashMap;
use enums::{CppMethodScope, CppTypeOrigin, CppVisibility};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CLangClassField {
  pub name: String,
  pub field_type: CppType,
  pub visibility: CppVisibility,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CLangCppTypeKind {
  Enum {
    values: Vec<EnumValue>,
  },
  Class {
    size: Option<i32>,
    bases: Vec<CppType>,
    fields: Vec<CLangClassField>,
    template_arguments: Option<Vec<String>>,
  },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CLangCppTypeData {
  pub name: String,
  pub header: String,
  pub kind: CLangCppTypeKind,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct CLangCppData {
  pub types: Vec<CLangCppTypeData>,
  pub methods: Vec<CppMethod>,
  pub template_instantiations: HashMap<String, Vec<Vec<CppType>>>,
}

impl CLangCppTypeData {
  pub fn inherits(&self, class_name: &String) -> bool {
    if let CLangCppTypeKind::Class { ref bases, .. } = self.kind {
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

impl CLangCppData {
  pub fn ensure_explicit_destructors(&mut self) {
    for type1 in &self.types {
      if let CLangCppTypeKind::Class { .. } = type1.kind {
        let class_name = &type1.name;
        let mut found_destructor = false;
        for method in &self.methods {
          if method.is_destructor {
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
            is_constructor: false,
            is_destructor: true,
            operator: None,
            conversion_operator: None,
            is_variable: false,
            arguments: vec![],
            allows_variable_arguments: false,
            original_index: 1000,
            origin: CppTypeOrigin::IncludeFile {
              include_file: type1.header.clone(),
              location: None,
            },
            template_arguments: None,
          });
        }
      }
    }
  }

  pub fn split_by_headers(&self) -> HashMap<String, CLangCppData> {
    let mut result = HashMap::new();
    for method in &self.methods {
      if let CppTypeOrigin::IncludeFile { ref include_file, .. } = method.origin {
        if !result.contains_key(include_file) {
          result.insert(include_file.clone(), CLangCppData::default());
        }
        result.get_mut(include_file).unwrap().methods.push(method.clone());
      }
    }
    for tp in &self.types {
      if !result.contains_key(&tp.header) {
        result.insert(tp.header.clone(), CLangCppData::default());
      }
      result.get_mut(&tp.header).unwrap().types.push(tp.clone());
      if let CLangCppTypeKind::Class { .. } = tp.kind {
        if let Some(ins) = self.template_instantiations.get(&tp.name) {
          result.get_mut(&tp.header)
                .unwrap()
                .template_instantiations
                .insert(tp.name.clone(), ins.clone());
        }
      }
    }
    result
  }
}
