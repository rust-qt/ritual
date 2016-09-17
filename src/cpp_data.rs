
use cpp_method::{CppMethod, CppMethodKind, CppMethodClassMembership, CppFunctionArgument,
                 CppMethodInheritedFrom};
use cpp_operator::CppOperator;
use std::collections::HashSet;
use log;
use cpp_type::{CppType, CppTypeBase, CppTypeIndirection, CppTypeClassBase};
use std::iter;

pub use serializable::{EnumValue, CppClassField, CppTypeKind, CppOriginLocation, CppVisibility,
                       CppTypeData, CppData, CppTemplateInstantiation, CppTemplateInstantiations,
                       CppClassUsingDirective};
// TODO: remove template arguments from methods, e.g.
// QList<T> findChildren<T>() -> QList<QObject*> findChildren()
fn apply_instantiations_to_method(method: &CppMethod,
                                  nested_level: i32,
                                  template_instantiations: &Vec<CppTemplateInstantiation>)
                                  -> Result<Vec<CppMethod>, String> {
  let mut new_methods = Vec::new();
  for ins in template_instantiations {
    log::noisy(format!("instantiation: {:?}", ins.template_arguments));
    let mut new_method = method.clone();
    new_method.arguments.clear();
    for arg in &method.arguments {
      new_method.arguments.push(CppFunctionArgument {
        name: arg.name.clone(),
        has_default_value: arg.has_default_value,
        argument_type: try!(arg.argument_type
          .instantiate(nested_level, &ins.template_arguments)),
      });
    }
    new_method.return_type = try!(method.return_type
      .instantiate(nested_level, &ins.template_arguments));
    if let Some(ref mut info) = new_method.class_membership {
      info.class_type = try!(info.class_type
        .instantiate_class(nested_level, &ins.template_arguments));
    }
    let mut conversion_type = None;
    if let Some(ref mut operator) = new_method.operator {
      if let &mut CppOperator::Conversion(ref mut cpp_type) = operator {
        let r = try!(cpp_type.instantiate(nested_level, &ins.template_arguments));
        *cpp_type = r.clone();
        conversion_type = Some(r);
      }
    }
    if new_method.all_involved_types()
      .iter()
      .find(|t| t.base.is_or_contains_template_parameter())
      .is_some() {
      return Err(format!("found remaining template parameters: {}",
                         new_method.short_text()));
    } else {
      if let Some(conversion_type) = conversion_type {
        new_method.name = format!("operator {}", try!(conversion_type.to_cpp_code(None)));
      }
      log::noisy(format!("success: {}", new_method.short_text()));
      new_methods.push(new_method);
    }
  }
  Ok(new_methods)
}

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
  pub fn default_class_type(&self) -> CppTypeClassBase {
    if !self.is_class() {
      panic!("not a class");
    }
    CppTypeClassBase {
      name: self.name.clone(),
      template_arguments: self.default_template_parameters(),
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
        if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = base.base {
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
  pub fn ensure_explicit_destructors(&mut self, dependencies: &Vec<&CppData>) {
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
          let is_virtual = self.has_virtual_destructor(class_name, dependencies);
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
            arguments_before_omitting: None,
            allows_variadic_arguments: false,
            include_file: type1.include_file.clone(),
            origin_location: None,
            template_arguments: None,
            declaration_code: None,
            inherited_from: None, // TODO: do we need inherited_from for destructors?
          });
        }
      }
    }
  }

  /// Helper function that performs a portion of add_inherited_methods implementation.
  fn inherited_methods_from(&self,
                            base_name: &String,
                            all_base_methods: &Vec<&CppMethod>)
                            -> Vec<CppMethod> {
    // TODO: speed up this method
    let mut new_methods = Vec::new();
    {
      for type1 in &self.types {
        if let CppTypeKind::Class { ref bases, ref using_directives, .. } = type1.kind {
          for base in bases {
            if let CppTypeBase::Class(CppTypeClassBase { ref name, ref template_arguments }) =
                   base.base {
              if name == base_name {
                log::noisy(format!("Adding inherited methods from {} to {}",
                                   base_name,
                                   type1.name));
                let derived_name = &type1.name;
                let base_template_arguments = template_arguments;
                let base_methods = all_base_methods.clone().into_iter().filter(|method| {
                  &method.class_membership
                    .as_ref()
                    .unwrap()
                    .class_type
                    .template_arguments == base_template_arguments
                });
                // derived_types.push(derived_name.clone());
                let mut current_new_methods = Vec::new();
                for base_class_method in base_methods {
                  let mut using_directive_enables = false;
                  let mut using_directive_disables = false;
                  for dir in using_directives {
                    if &dir.method_name == &base_class_method.name {
                      if &dir.class_name == base_name {
                        log::noisy(format!("UsingDirective enables inheritance of {}",
                                           base_class_method.short_text()));
                        using_directive_enables = true;
                      } else {
                        log::noisy(format!("UsingDirective disables inheritance of {}",
                                           base_class_method.short_text()));
                        using_directive_disables = true;
                      }
                    }
                  }
                  if using_directive_disables {
                    continue;
                  }

                  let mut ok = true;
                  for method in &self.methods {
                    if method.class_name() == Some(derived_name) &&
                       method.name == base_class_method.name {
                      // without using directive, any method with the same name
                      // disables inheritance of base class method;
                      // with using directive, only method with the same arguments
                      // disables inheritance of base class method.
                      if !using_directive_enables ||
                         method.argument_types_equal(base_class_method) {
                        log::noisy("Method is not added because it's overriden in derived class");
                        log::noisy(format!("Base method: {}", base_class_method.short_text()));
                        log::noisy(format!("Derived method: {}\n", method.short_text()));
                        ok = false;
                      }
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
                    new_method.declaration_code = None;
                    if new_method.inherited_from.is_none() {
                      new_method.inherited_from = Some(CppMethodInheritedFrom {
                        doc_id: base_class_method.doc_id(),
                        short_text: base_class_method.short_text(),
                        declaration_code: base_class_method.declaration_code.clone(),
                        class_type: base_class_method.class_membership
                          .as_ref()
                          .unwrap()
                          .class_type
                          .clone(),
                      });
                    }
                    log::noisy(format!("Method added: {}", new_method.short_text()));
                    log::noisy(format!("Base method: {} ({:?})\n",
                                       base_class_method.short_text(),
                                       base_class_method.origin_location));
                    current_new_methods.push(new_method.clone());
                  }
                }
                new_methods.append(&mut self.inherited_methods_from(derived_name,
                                                                    &current_new_methods.iter().collect()));
                new_methods.append(&mut current_new_methods);
              }
            }
          }
        }
      }
    }
    new_methods
  }

  /// Adds methods of derived classes inherited from base classes.
  /// A method will not be added if there is a method with the same
  /// name in the derived class. Constructors, destructors and assignment
  /// operators are also not added. This reflects C++'s method inheritance rules.
  pub fn add_inherited_methods(&mut self, dependencies: &Vec<&CppData>) {
    log::info("Adding inherited methods");
    let mut all_new_methods = Vec::new();
    for (is_self, cpp_data) in dependencies.clone()
      .into_iter()
      .map(|x| (false, x))
      .chain(iter::once((true, self as &_))) {
      for type1 in &cpp_data.types {
        if type1.is_class() {
          let mut interesting_cpp_datas = vec![cpp_data];
          if !is_self {
            interesting_cpp_datas.push(self);
          }
          for cpp_data2 in interesting_cpp_datas {
            let base_methods = cpp_data2.methods
              .iter()
              .filter(|method| {
                if let Some(ref info) = method.class_membership {
                  &info.class_type.name == &type1.name && !info.kind.is_constructor() &&
                  !info.kind.is_destructor() &&
                  method.operator != Some(CppOperator::Assignment)
                } else {
                  false
                }
              });
            all_new_methods.append(&mut self.inherited_methods_from(&type1.name,
                                                                    &base_methods.collect()));
          }
        }
      }
    }
    self.methods.append(&mut all_new_methods);
    log::info("Finished adding inherited methods");
  }

  /// Generates duplicate methods with fewer arguments for
  /// C++ methods with default argument values.
  pub fn generate_methods_with_omitted_args(&mut self) {
    let mut new_methods = Vec::new();
    for method in &self.methods {
      if method.arguments.len() > 0 && method.arguments.last().unwrap().has_default_value {
        let mut method_copy = method.clone();
        method_copy.arguments_before_omitting = Some(method.arguments.clone());
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
    for instantiations in &self.template_instantiations {
      if !result.contains(&instantiations.include_file) {
        result.insert(instantiations.include_file.clone());
      }
    }
    result
  }

  /// Checks if specified class is a template class.
  #[allow(dead_code)]
  pub fn is_template_class(&self, name: &String) -> bool {
    if let Some(type_info) = self.types.iter().find(|t| &t.name == name) {
      if let CppTypeKind::Class { ref template_arguments, ref bases, .. } = type_info.kind {
        if template_arguments.is_some() {
          return true;
        }
        for base in bases {
          if let CppTypeBase::Class(CppTypeClassBase { ref name, ref template_arguments }) =
                 base.base {
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
  pub fn has_virtual_destructor(&self, class_name: &String, dependencies: &Vec<&CppData>) -> bool {
    for method in &self.methods {
      if method.is_destructor() && method.class_name() == Some(class_name) {
        return method.class_membership.as_ref().unwrap().is_virtual;
      }
    }
    if let Some(type_info) = self.types.iter().find(|t| &t.name == class_name) {
      if let CppTypeKind::Class { ref bases, .. } = type_info.kind {
        for base in bases {
          if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = base.base {
            if self.has_virtual_destructor(name, dependencies) {
              return true;
            }
          }
        }
      }
    }
    for dep in dependencies {
      if dep.has_virtual_destructor(class_name, &Vec::new()) {
        return true;
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
          if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = base.base {
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

  // TODO: dependency data is needed here!
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
          if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = base.base {
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



  fn instantiate_templates(&mut self, dependencies: &Vec<&CppData>) {
    log::info("Instantiating templates.");
    let mut new_methods = Vec::new();

    for cpp_data in dependencies.clone().into_iter().chain(iter::once(self as &_)) {
      for method in &cpp_data.methods {
        for type1 in method.all_involved_types() {
          if let CppTypeBase::Class(CppTypeClassBase { ref name, ref template_arguments }) =
                 type1.base {
            if let &Some(ref template_arguments) = template_arguments {
              assert!(!template_arguments.is_empty());
              if template_arguments.iter().find(|x| !x.base.is_template_parameter()).is_none() {
                if let Some(template_instantiations) = self.template_instantiations
                  .iter()
                  .find(|x| &x.class_name == name) {
                  let nested_level = if let CppTypeBase::TemplateParameter { nested_level, .. } =
                                            template_arguments[0].base {
                    nested_level
                  } else {
                    panic!("only template parameters can be here");
                  };
                  log::noisy(format!(""));
                  log::noisy(format!("method: {}", method.short_text()));
                  log::noisy(format!("found template instantiations: {:?}",
                                     template_instantiations));
                  match apply_instantiations_to_method(method,
                                                       nested_level,
                                                       &template_instantiations.instantiations) {
                    Ok(mut methods) => {
                      new_methods.append(&mut methods);
                      break;
                    }
                    Err(msg) => log::noisy(format!("failed: {}", msg)),
                  }
                  break;
                }
              }
            }
          }
        }
      }
    }
    self.methods.append(&mut new_methods);
  }

  pub fn remove_existing_instantiations(&mut self, dependencies: &Vec<&CppData>) {
    self.template_instantiations = self.template_instantiations
      .iter()
      .filter_map(|data| {
        let good_items: Vec<_> = {
          let class_name = &data.class_name;
          data.instantiations
            .clone()
            .into_iter()
            .filter(|ins| {
              dependencies.iter()
                .find(|dep| {
                  match dep.template_instantiations
                    .iter()
                    .find(|x| &x.class_name == class_name) {
                    None => false,
                    Some(vec) => {
                      vec.instantiations
                        .iter()
                        .find(|x| x.template_arguments == ins.template_arguments)
                        .is_some()
                    }
                  }
                })
                .is_none()
            })
            .collect()
        };
        if good_items.is_empty() {
          None
        } else {
          Some(CppTemplateInstantiations {
            class_name: data.class_name.clone(),
            include_file: data.include_file.clone(),
            instantiations: good_items,
          })
        }
      })
      .collect();
  }


  /// Performs data conversion to make it more suitable
  /// for further wrapper generation.
  pub fn post_process(&mut self, dependencies: &Vec<&CppData>) {
    self.remove_existing_instantiations(dependencies);
    self.ensure_explicit_destructors(dependencies);
    self.generate_methods_with_omitted_args();
    self.instantiate_templates(dependencies);
    self.add_inherited_methods(dependencies);
  }
}
