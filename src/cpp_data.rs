
use cpp_method::{CppMethod, CppMethodScope, CppMethodKind};
use cpp_operators::CppOperator;
use std::collections::HashMap;
use log;
use cpp_type::{CppType, CppTypeBase, CppTypeIndirection};

pub use serializable::{EnumValue, CppClassField, CppTypeKind, CppOriginLocation, CppVisibility,
                       CppTypeData, CppData};

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

  fn add_inherited_methods_from(&mut self, base_name: &String) {
    let template_arguments = match self.types.iter().find(|x| &x.name == base_name) {
      None => None,
      Some(type_data) => match type_data.kind {
        CppTypeKind::Class { ref template_arguments, .. } => {
          match *template_arguments {
            None => None,
            Some(ref strings) => {
              Some(strings.iter()
                  .enumerate()
                  .map(|(num, _)| {
                    CppType {
                      is_const: false,
                      indirection: CppTypeIndirection::None,
                      base: CppTypeBase::TemplateParameter {
                        nested_level: 0,
                        index: num as i32,
                      },
                    }
                  })
                  .collect())
            }
          }
        }
        _ => None
      }
    };

    let mut new_methods = Vec::new();
    let mut derived_types = Vec::new();
    {
      let base_methods: Vec<_> = self.methods.iter().filter(|method| {
        if method.kind.is_constructor() || method.kind.is_destructor() ||
            method.kind == CppMethodKind::Operator(CppOperator::Assignment) {
          return false;
        }
        if let CppMethodScope::Class(ref name) = method.scope {
          name == base_name
        } else {
          false
        }
      }).collect();
      for type1 in &self.types {
        if type1.inherits(base_name) {
          let derived_name = &type1.name;
          derived_types.push(derived_name.clone());
          for base_class_method in base_methods.clone() {
            let mut ok = true;
            for method in &self.methods {
              if let CppMethodScope::Class(ref name) = method.scope {
                if name == derived_name && method.name == base_class_method.name {
                  log::info("Method is not added because it's overriden in derived class");
                  log::info(format!("Base method: {}", base_class_method.short_text()));
                  log::info(format!("Derived method: {}\n", method.short_text()));
                  ok = false;
                  break;
                }
              }
            }
            if ok {
              let mut new_method = base_class_method.clone();
              new_method.scope = CppMethodScope::Class(derived_name.clone());
              new_method.include_file = type1.include_file.clone();
              new_method.origin_location = None;
              if new_method.arguments.len() > 0 && new_method.arguments[0].name == "this" {
                new_method.arguments[0].argument_type.base = CppTypeBase::Class {
                  name: derived_name.clone(),
                  template_arguments: template_arguments.clone(),
                };
              }
              log::info(format!("Method added: {}", new_method.short_text()));
              new_methods.push(new_method.clone());
            }
          }
        }
      }
    }
    self.methods.append(&mut new_methods);
    for name in derived_types {
      self.add_inherited_methods_from(&name);
    }
  }

  pub fn add_inherited_methods(&mut self) {
    log::info("Adding inherited methods");
    let all_type_names: Vec<_> = self.types.iter().map(|t| t.name.clone()).collect();
    for name in all_type_names {
      self.add_inherited_methods_from(&name);
    }
    log::info("Finished adding inherited methods");
  }

  pub fn generate_methods_with_omitted_args(&mut self) {
    let mut new_methods = Vec::new();
    for method in &self.methods {
      if method.arguments.len() > 0 && method.arguments.last().unwrap().has_default_value {
        let mut method_copy = method.clone();
        while method_copy.arguments.len() > 0 &&
              method_copy.arguments.last().unwrap().has_default_value {
          method_copy.arguments.pop().unwrap();
          new_methods.push(method_copy.clone());
        }
      }
    }
    self.methods.append(&mut new_methods);
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

  pub fn is_template_class(&self, name: &String) -> bool {
    if let Some(type_info) = self.types.iter().find(|t| &t.name == name) {
      if let CppTypeKind::Class { ref template_arguments, ref bases, .. } = type_info.kind {
        if template_arguments.is_some() {
          return true;
        }
        for base in bases {
          if let CppTypeBase::Class { ref name, ref template_arguments } = base.base {
            if template_arguments.is_some() {
              return true;
            }
            if self.is_template_class(name) {
              return true;
            }
          }
        }
      }
    } else {
      log::warning(format!("Unknown type assumed to be non-template: {}", name));
    }
    false
  }
}
