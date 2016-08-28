
use cpp_method::{CppMethod, CppMethodKind, CppMethodClassMembership};
use cpp_operator::CppOperator;
use std::collections::HashSet;
use log;
use cpp_type::{CppType, CppTypeBase, CppTypeIndirection};

pub use serializable::{EnumValue, CppClassField, CppTypeKind, CppOriginLocation, CppVisibility,
                       CppTypeData, CppData, CppTemplateInstantiation};

impl CppTypeData {
  /// Checks if the type is a class type.
  pub fn is_class(&self) -> bool {
    match self.kind {
      CppTypeKind::Class { .. } => true,
      _ => false,
    }
  }

  /// Creates CppTypeBase object representing type
  /// of an object of this type. See
  /// default_template_parameters() documentation
  /// for details about handling template parameters.
  pub fn default_class_type(&self) -> CppTypeBase {
    match self.kind {
      CppTypeKind::Class { .. } => {
        CppTypeBase::Class {
          name: self.name.clone(),
          template_arguments: self.default_template_parameters(),
        }
      }
      _ => panic!("not a class"),
    }
  }

  /// Creates template parameters expected for this type.
  /// For example, QHash<QString, int> will have 2 default
  /// template parameters with indexes 0 and 1. This function
  /// is helpful for determining type of "this" pointer.
  /// Result of this function may differ from actual template
  /// parameters, for example:
  /// - if a class is inside another template class,
  /// nested level should be 1 instead of 0;
  /// - if QList<V> type is used inside QHash<K, V> type,
  /// QList's template parameter will have index = 1
  /// instead of 0.
  pub fn default_template_parameters(&self) -> Option<Vec<CppType>> {
    match self.kind {
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
      _ => None,
    }
  }

  /// Checks if the type was directly derived from specified type.
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
  /// Adds destructors for every class that does not have explicitly
  /// defined destructor, allowing to create wrappings for
  /// destructors implicitly available in C++.
  pub fn ensure_explicit_destructors(&mut self) {
    for type1 in &self.types {
      if let CppTypeKind::Class { .. } = type1.kind {
        let class_name = &type1.name;
        let mut found_destructor = false;
        for method in &self.methods {
          if method.is_destructor() && method.class_name() == Some(class_name) {
            found_destructor = true;
            break;
          }
        }
        if !found_destructor {
          let is_virtual = self.has_virtual_destructor(class_name);
          self.methods.push(CppMethod {
            name: format!("~{}", class_name),
            class_membership: Some(CppMethodClassMembership {
              class_type: type1.default_class_type(),
              is_virtual: is_virtual,
              is_pure_virtual: false,
              is_const: false,
              is_static: false,
              visibility: CppVisibility::Public,
              is_signal: false,
              kind: CppMethodKind::Destructor,
            }),
            operator: None,
            return_type: CppType::void(),
            arguments: vec![],
            allows_variadic_arguments: false,
            include_file: type1.include_file.clone(),
            origin_location: None,
            template_arguments: None,
          });
        }
      }
    }
  }

  /// Helper function that performs a portion of add_inherited_methods implementation.
  fn add_inherited_methods_from(&mut self, base_name: &String) {
    let mut new_methods = Vec::new();
    let mut derived_types = Vec::new();
    {
      let base_methods: Vec<_> = self.methods
        .iter()
        .filter(|method| {
          if let Some(ref info) = method.class_membership {
            info.class_type.maybe_name().unwrap() == base_name && !info.kind.is_constructor() &&
            !info.kind.is_destructor() &&
            method.operator != Some(CppOperator::Assignment)
          } else {
            false
          }
        })
        .collect();
      for type1 in &self.types {
        if type1.inherits(base_name) {
          let derived_name = &type1.name;
          derived_types.push(derived_name.clone());
          for base_class_method in base_methods.clone() {
            let mut ok = true;
            for method in &self.methods {
              if method.class_name() == Some(derived_name) &&
                 method.name == base_class_method.name {
                // log::info("Method is not added because it's overriden in derived class");
                // log::info(format!("Base method: {}", base_class_method.short_text()));
                // log::info(format!("Derived method: {}\n", method.short_text()));
                ok = false;
                break;
              }
            }
            if ok {
              let mut new_method = base_class_method.clone();
              if let Some(ref mut info) = new_method.class_membership {
                info.class_type = type1.default_class_type();
              } else {
                panic!("class_membership must be present");
              }
              new_method.include_file = type1.include_file.clone();
              new_method.origin_location = None;
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

  /// Adds methods of derived classes inherited from base classes.
  /// A method will not be added if there is a method with the same
  /// name in the derived class. Constructors, destructors and assignment
  /// operators are also not added. This reflects C++'s method inheritance rules.
  pub fn add_inherited_methods(&mut self) {
    log::info("Adding inherited methods");
    let all_type_names: Vec<_> = self.types.iter().map(|t| t.name.clone()).collect();
    for name in all_type_names {
      self.add_inherited_methods_from(&name);
    }
    log::info("Finished adding inherited methods");
  }

  /// Generates duplicate methods with fewer arguments for
  /// C++ methods with default argument values.
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

  pub fn all_include_files(&self) -> HashSet<String> {
    let mut result = HashSet::new();
    for method in &self.methods {
      if !result.contains(&method.include_file) {
        result.insert(method.include_file.clone());
      }
    }
    for tp in &self.types {
      if !result.contains(&tp.include_file) {
        result.insert(tp.include_file.clone());
      }
    }
    result
  }

  /// Checks if specified class is a template class.
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

  /// Checks if specified class has virtual destructor (own or inherited).
  pub fn has_virtual_destructor(&self, class_name: &String) -> bool {
    for method in &self.methods {
      if method.is_destructor() && method.class_name() == Some(class_name) {
        return method.class_membership.as_ref().unwrap().is_virtual;
      }
    }
    if let Some(type_info) = self.types.iter().find(|t| &t.name == class_name) {
      if let CppTypeKind::Class { ref bases, .. } = type_info.kind {
        for base in bases {
          if let CppTypeBase::Class { ref name, .. } = base.base {
            if self.has_virtual_destructor(name) {
              return true;
            }
          }
        }
      }
    }
    return false;
  }


  #[allow(dead_code)]
  pub fn get_all_methods(&self, class_name: &String) -> Vec<&CppMethod> {
    let own_methods: Vec<_> = self.methods
      .iter()
      .filter(|m| m.class_name() == Some(class_name))
      .collect();
    let mut inherited_methods = Vec::new();
    if let Some(type_info) = self.types.iter().find(|t| &t.name == class_name) {
      if let CppTypeKind::Class { ref bases, .. } = type_info.kind {
        for base in bases {
          if let CppTypeBase::Class { ref name, .. } = base.base {
            for method in self.get_all_methods(name) {
              if own_methods.iter()
                .find(|m| m.name == method.name && m.argument_types_equal(&method))
                .is_none() {
                inherited_methods.push(method);
              }
            }
          }
        }
      } else {
        panic!("get_all_methods: not a class");
      }
    } else {
      log::warning(format!("get_all_methods: no type info for {:?}", class_name));
    }
    for method in own_methods {
      inherited_methods.push(method);
    }
    inherited_methods
  }

  pub fn get_pure_virtual_methods(&self, class_name: &String) -> Vec<&CppMethod> {

    let own_methods: Vec<_> = self.methods
      .iter()
      .filter(|m| m.class_name() == Some(class_name))
      .collect();
    let own_pure_virtual_methods: Vec<_> = own_methods.iter()
      .filter(|m| {
        m.class_membership
          .as_ref()
          .unwrap()
          .is_pure_virtual
      })
      .collect();
    let mut inherited_methods = Vec::new();
    if let Some(type_info) = self.types.iter().find(|t| &t.name == class_name) {
      if let CppTypeKind::Class { ref bases, .. } = type_info.kind {
        for base in bases {
          if let CppTypeBase::Class { ref name, .. } = base.base {
            for method in self.get_pure_virtual_methods(name) {
              if own_methods.iter()
                .find(|m| m.name == method.name && m.argument_types_equal(&method))
                .is_none() {
                inherited_methods.push(method);
              }
            }
          }
        }
      } else {
        panic!("get_pure_virtual_methods: not a class");
      }
    } else {
      log::warning(format!("get_pure_virtual_methods: no type info for {:?}",
                           class_name));
    }
    for method in own_pure_virtual_methods {
      inherited_methods.push(method);
    }
    inherited_methods
  }


  /// Performs data conversion to make it more suitable
  /// for further wrapper generation.
  pub fn post_process(&mut self) {
    self.ensure_explicit_destructors();
    self.generate_methods_with_omitted_args();
    self.add_inherited_methods();
  }
}
