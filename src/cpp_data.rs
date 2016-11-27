use cpp_method::{CppMethod, CppMethodKind, CppMethodClassMembership, CppFunctionArgument,
                 CppFieldAccessorType, FakeCppMethod};
use cpp_operator::CppOperator;
use cpp_type::{CppType, CppTypeBase, CppTypeIndirection, CppTypeClassBase};
use errors::{Result, unexpected, ChainErr};
use file_utils::open_file;
use log;

use std::collections::{HashSet, HashMap};
use std::iter::once;
use std::io::{BufRead, BufReader};
use string_utils::JoinWithString;

pub use serializable::{CppEnumValue, CppClassField, CppTypeKind, CppOriginLocation, CppVisibility,
                       CppTypeData, CppTypeDoc, CppData, CppTemplateInstantiation,
                       CppTemplateInstantiations, CppClassUsingDirective, CppBaseSpecifier,
                       TemplateArgumentsDeclaration, CppFunctionPointerType};

extern crate regex;
use self::regex::Regex;

fn apply_instantiations_to_method(method: &CppMethod,
                                  nested_level: i32,
                                  template_instantiations: &[CppTemplateInstantiation])
                                  -> Result<Vec<CppMethod>> {
  let mut new_methods = Vec::new();
  for ins in template_instantiations {
    log::noisy(format!("instantiation: {:?}", ins.template_arguments));
    let mut new_method = method.clone();
    if let Some(ref args) = method.template_arguments {
      if args.nested_level == nested_level {
        if args.count() != ins.template_arguments.len() as i32 {
          return Err("template arguments count mismatch".into());
        }
        new_method.template_arguments = None;
        new_method.template_arguments_values = Some(ins.template_arguments.clone());
      }
    }
    new_method.arguments.clear();
    for arg in &method.arguments {
      new_method.arguments.push(CppFunctionArgument {
        name: arg.name.clone(),
        has_default_value: arg.has_default_value,
        argument_type: try!(arg.argument_type
          .instantiate(nested_level, &ins.template_arguments)),
      });
    }
    if let Some(ref args) = method.arguments_before_omitting {
      let mut new_args = Vec::new();
      for arg in args {
        new_args.push(CppFunctionArgument {
          name: arg.name.clone(),
          has_default_value: arg.has_default_value,
          argument_type: try!(arg.argument_type
            .instantiate(nested_level, &ins.template_arguments)),
        });
      }
      new_method.arguments_before_omitting = Some(new_args);
    }
    new_method.return_type = try!(method.return_type
      .instantiate(nested_level, &ins.template_arguments));
    if let Some(ref mut info) = new_method.class_membership {
      info.class_type = try!(info.class_type
        .instantiate_class(nested_level, &ins.template_arguments));
    }
    let mut conversion_type = None;
    if let Some(ref mut operator) = new_method.operator {
      if let CppOperator::Conversion(ref mut cpp_type) = *operator {
        let r = try!(cpp_type.instantiate(nested_level, &ins.template_arguments));
        *cpp_type = r.clone();
        conversion_type = Some(r);
      }
    }
    if new_method.all_involved_types()
      .iter()
      .any(|t| t.base.is_or_contains_template_parameter()) {
      return Err(format!("extra template parameters left: {}",
                         new_method.short_text())
        .into());
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
  /// default_template_arguments() documentation
  /// for details about handling template parameters.
  pub fn default_class_type(&self) -> Result<CppTypeClassBase> {
    if !self.is_class() {
      return Err("not a class".into());
    }
    Ok(CppTypeClassBase {
      name: self.name.clone(),
      template_arguments: self.default_template_arguments(),
    })
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
  pub fn default_template_arguments(&self) -> Option<Vec<CppType>> {
    match self.kind {
      CppTypeKind::Class { ref template_arguments, .. } => {
        match *template_arguments {
          None => None,
          Some(ref arguments) => {
            Some(arguments.names
              .iter()
              .enumerate()
              .map(|(num, _)| {
                CppType {
                  is_const: false,
                  is_const2: false,
                  indirection: CppTypeIndirection::None,
                  base: CppTypeBase::TemplateParameter {
                    nested_level: arguments.nested_level,
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
  pub fn inherits_directly(&self, class_name: &str) -> bool {
    if let CppTypeKind::Class { ref bases, .. } = self.kind {
      for base in bases {
        if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = base.base_type.base {
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
  pub fn find_type_info<F>(&self, f: F) -> Option<&CppTypeData>
    where F: Fn(&&CppTypeData) -> bool
  {
    once(&self.types).chain(self.dependencies.iter().map(|d| &d.types)).flat_map(|x| x).find(f)
  }

  pub fn is_polymorphic_type(&self, name: &str) -> bool {
    self.methods.iter().any(|m| if let Some(ref info) = m.class_membership {
      info.is_virtual && &info.class_type.name == name
    } else {
      false
    })
  }

  /// Adds destructors for every class that does not have explicitly
  /// defined destructor, allowing to create wrappings for
  /// destructors implicitly available in C++.
  pub fn ensure_explicit_destructors(&mut self) -> Result<()> {
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
              class_type: try!(type1.default_class_type()),
              is_virtual: is_virtual,
              is_pure_virtual: false,
              is_const: false,
              is_static: false,
              visibility: CppVisibility::Public,
              is_signal: false,
              is_slot: false,
              kind: CppMethodKind::Destructor,
              fake: None,
            }),
            operator: None,
            return_type: CppType::void(),
            arguments: vec![],
            arguments_before_omitting: None,
            allows_variadic_arguments: false,
            include_file: type1.include_file.clone(),
            origin_location: None,
            template_arguments: None,
            template_arguments_values: None,
            declaration_code: None,
            doc: None,
            inheritance_chain: Vec::new(),
            is_ffi_whitelisted: false,
            is_unsafe_static_cast: false,
          });
        }
      }
    }
    Ok(())
  }

  /// Helper function that performs a portion of add_inherited_methods implementation.
  fn inherited_methods_from(&self,
                            base_name: &str,
                            all_base_methods: &[&CppMethod])
                            -> Result<Vec<CppMethod>> {
    // TODO: speed up this method (#12)
    let mut new_methods = Vec::new();
    {
      for type1 in &self.types {
        if let CppTypeKind::Class { ref bases, ref using_directives, .. } = type1.kind {
          for base in bases {
            if let CppTypeBase::Class(CppTypeClassBase { ref name, ref template_arguments }) =
                   base.base_type.base {
              if name == base_name {
                log::noisy(format!("Adding inherited methods from {} to {}",
                                   base_name,
                                   type1.name));
                let derived_name = &type1.name;
                let base_template_arguments = template_arguments;
                let base_methods = all_base_methods.into_iter().filter(|method| {
                  if let Some(ref info) = method.class_membership {
                    &info.class_type.template_arguments == base_template_arguments
                  } else {
                    false
                  }
                });
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
                    let mut new_method: CppMethod = (*base_class_method).clone();
                    if let Some(ref mut info) = new_method.class_membership {
                      info.class_type = try!(type1.default_class_type());
                    } else {
                      return Err(unexpected("no class membership").into());
                    }
                    new_method.include_file = type1.include_file.clone();
                    new_method.origin_location = None;
                    new_method.declaration_code = None;
                    new_method.inheritance_chain.push(base.clone());
                    log::noisy(format!("Method added: {}", new_method.short_text()));
                    log::noisy(format!("Base method: {} ({:?})\n",
                                       base_class_method.short_text(),
                                       base_class_method.origin_location));
                    current_new_methods.push(new_method.clone());
                  }
                }
                new_methods.append(&mut try!(self.inherited_methods_from(derived_name,
                                                                         &current_new_methods.iter()
                                                                           .collect::<Vec<_>>())));
                new_methods.append(&mut current_new_methods);
              }
            }
          }
        }
      }
    }
    Ok(new_methods)
  }

  /// Adds methods of derived classes inherited from base classes.
  /// A method will not be added if there is a method with the same
  /// name in the derived class. Constructors, destructors and assignment
  /// operators are also not added. This reflects C++'s method inheritance rules.
  #[cfg_attr(feature="clippy", allow(block_in_if_condition_stmt))]
  pub fn add_inherited_methods(&mut self) -> Result<()> {
    log::info("Adding inherited methods");
    let mut all_new_methods = Vec::new();
    for (is_self, cpp_data) in self.dependencies
      .iter()
      .map(|x| (false, x))
      .chain(once((true, self as &_))) {
      for type1 in &cpp_data.types {
        if type1.is_class() {
          let mut interesting_cpp_datas: Vec<&CppData> = vec![cpp_data];
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
            all_new_methods.append(&mut try!(self.inherited_methods_from(&type1.name,
                                                                    &base_methods.collect::<Vec<_>>())));
          }
        }
      }
    }
    while let Some(method) = all_new_methods.pop() {
      let mut duplicates = Vec::new();
      while let Some(index) = all_new_methods.iter()
        .position(|m| m.class_name() == method.class_name() && m.name == method.name) {
        duplicates.push(all_new_methods.remove(index));
      }
      if duplicates.is_empty() {
        self.methods.push(method);
      } else {
        duplicates.push(method);

        let mut allow_method = false;

        let mut lowest_visibility = CppVisibility::Public;
        for duplicate in &duplicates {
          if let Some(ref info) = duplicate.class_membership {
            if info.visibility == CppVisibility::Private {
              lowest_visibility = CppVisibility::Private;
            } else if info.visibility == CppVisibility::Protected &&
                      lowest_visibility != CppVisibility::Private {
              lowest_visibility = CppVisibility::Protected;
            }
          } else {
            return Err("only class methods can appear here".into());
          }
        }
        if duplicates.iter()
          .find(|m| m.inheritance_chain.last() != duplicates[0].inheritance_chain.last())
          .is_none() {
          // all methods are inherited from one base class
          self.methods.append(&mut duplicates);
        } else {
          let signature_mismatch = duplicates.iter()
            .any(|m| {
              let f = &duplicates[0];
              let info_mismatch = if let Some(ref m_info) = m.class_membership {
                if let Some(ref f_info) = f.class_membership {
                  m_info.is_const != f_info.is_const || m_info.is_static != f_info.is_static
                } else {
                  true
                }
              } else {
                true
              };
              info_mismatch || &m.return_type != &f.return_type || !m.argument_types_equal(f) ||
              m.allows_variadic_arguments == f.allows_variadic_arguments
            });
          if !signature_mismatch && !duplicates.iter().any(|x| x.inheritance_chain.is_empty()) {
            // TODO: support more complicated cases (#23)
            let first_base = &duplicates[0].inheritance_chain[0].base_type;
            if duplicates.iter().all(|x| {
              x.inheritance_chain[0].is_virtual && &x.inheritance_chain[0].base_type == first_base
            }) {
              allow_method = true;
            }
          }
          if allow_method {
            log::noisy("Allowing duplicated inherited method (virtual diamond \
                                inheritance)");
            log::noisy(duplicates[0].short_text());
            for duplicate in &duplicates {
              log::noisy(format!("  {}", duplicate.inheritance_chain_text()));
            }
            if let Some(mut final_method) = duplicates.pop() {
              if let Some(ref mut info) = final_method.class_membership {
                info.visibility = lowest_visibility;
              } else {
                return Err("only class methods can appear here".into());
              }
              self.methods.push(final_method);
            } else {
              return Err(unexpected("duplicates can't be empty").into());
            }
          } else {
            log::noisy("Removed ambiguous inherited methods (presumed inaccessible):");
            if signature_mismatch {
              for duplicate in &duplicates {
                log::noisy(format!("  {}", duplicate.short_text()));
                log::noisy(format!("  {}", duplicate.inheritance_chain_text()));
              }
            } else {
              log::noisy(duplicates[0].short_text());
              for duplicate in &duplicates {
                log::noisy(format!("  {}", duplicate.inheritance_chain_text()));
              }
            }
          }
        }
      }
    }
    log::info("Finished adding inherited methods");
    Ok(())
  }

  /// Generates duplicate methods with fewer arguments for
  /// C++ methods with default argument values.
  pub fn generate_methods_with_omitted_args(&mut self) {
    let mut new_methods = Vec::new();
    for method in &self.methods {
      if let Some(last_arg) = method.arguments.last() {
        if last_arg.has_default_value {
          let mut method_copy = method.clone();
          method_copy.arguments_before_omitting = Some(method.arguments.clone());
          while let Some(arg) = method_copy.arguments.pop() {
            if !arg.has_default_value {
              break;
            }
            new_methods.push(method_copy.clone());
          }
        }
      }
    }
    self.methods.append(&mut new_methods);
  }

  pub fn all_include_files(&self) -> Result<HashSet<String>> {
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
      let type_info = try!(self.find_type_info(|x| &x.name == &instantiations.class_name)
        .chain_err(|| format!("type info not found for {}", &instantiations.class_name)));
      if !result.contains(&type_info.include_file) {
        result.insert(type_info.include_file.clone());
      }
    }
    Ok(result)
  }

  /// Checks if specified class is a template class.
  #[allow(dead_code)]
  pub fn is_template_class(&self, name: &str) -> bool {
    if let Some(type_info) = self.types.iter().find(|t| &t.name == name) {
      if let CppTypeKind::Class { ref template_arguments, ref bases, .. } = type_info.kind {
        if template_arguments.is_some() {
          return true;
        }
        for base in bases {
          if let CppTypeBase::Class(CppTypeClassBase { ref name, ref template_arguments }) =
                 base.base_type.base {
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
  pub fn has_virtual_destructor(&self, class_name: &str) -> bool {
    for method in &self.methods {
      if let Some(ref info) = method.class_membership {
        if info.kind == CppMethodKind::Destructor && &info.class_type.name == class_name {
          return info.is_virtual;
        }
      }
    }
    if let Some(type_info) = self.types.iter().find(|t| &t.name == class_name) {
      if let CppTypeKind::Class { ref bases, .. } = type_info.kind {
        for base in bases {
          if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = base.base_type.base {
            if self.has_virtual_destructor(name) {
              return true;
            }
          }
        }
      }
    }
    for dep in &self.dependencies {
      if dep.has_virtual_destructor(class_name) {
        return true;
      }
    }
    false
  }

  /// Checks if specified class has public destructor.
  pub fn has_public_destructor(&self, class_type: &CppTypeClassBase) -> bool {
    for method in &self.methods {
      if let Some(ref info) = method.class_membership {
        if info.kind == CppMethodKind::Destructor && &info.class_type == class_type {
          return info.visibility == CppVisibility::Public;
        }
      }
    }
    false
  }


  #[allow(dead_code)]
  pub fn get_all_methods(&self, class_name: &str) -> Result<Vec<&CppMethod>> {
    let own_methods: Vec<_> = self.methods
      .iter()
      .filter(|m| m.class_name().map(|x| x.as_ref()) == Some(class_name))
      .collect();
    let mut inherited_methods = Vec::new();
    if let Some(type_info) = self.types.iter().find(|t| &t.name == class_name) {
      if let CppTypeKind::Class { ref bases, .. } = type_info.kind {
        for base in bases {
          if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = base.base_type.base {
            for method in try!(self.get_all_methods(name)) {
              if own_methods.iter()
                .find(|m| m.name == method.name && m.argument_types_equal(method))
                .is_none() {
                inherited_methods.push(method);
              }
            }
          }
        }
      } else {
        return Err("get_all_methods: not a class".into());
      }
    } else {
      log::warning(format!("get_all_methods: no type info for {:?}", class_name));
    }
    for method in own_methods {
      inherited_methods.push(method);
    }
    Ok(inherited_methods)
  }

  pub fn has_pure_virtual_methods(&self, class_name: &str) -> bool {
    self.methods.iter().any(|m| match m.class_membership {
      Some(ref info) => &info.class_type.name == class_name && info.is_pure_virtual,
      None => false,
    })
  }

  fn check_template_type(&self, type1: &CppType) -> Result<()> {
    if let CppTypeBase::Class(CppTypeClassBase { ref name, ref template_arguments }) = type1.base {
      if let Some(ref template_arguments) = *template_arguments {
        let is_valid = |cpp_data: &CppData| {
          cpp_data.template_instantiations.iter().any(|inst| {
            &inst.class_name == name &&
            inst.instantiations
              .iter()
              .any(|x| &x.template_arguments == template_arguments)
          })
        };
        if !self.dependencies.iter().chain(once(self)).any(is_valid) {
          return Err(format!("type not available: {:?}", type1).into());
        }
        for arg in template_arguments {
          try!(self.check_template_type(arg));
        }
      }
    }
    Ok(())
  }

  fn instantiate_templates(&mut self) -> Result<()> {
    log::info("Instantiating templates.");
    let mut new_methods = Vec::new();

    for cpp_data in self.dependencies.iter().chain(once(self as &_)) {
      for method in &cpp_data.methods {
        for type1 in method.all_involved_types() {
          if let CppTypeBase::Class(CppTypeClassBase { ref name, ref template_arguments }) =
                 type1.base {
            if let Some(ref template_arguments) = *template_arguments {
              assert!(!template_arguments.is_empty());
              if template_arguments.iter().all(|x| x.base.is_template_parameter()) {
                if let Some(template_instantiations) = self.template_instantiations
                  .iter()
                  .find(|x| &x.class_name == name) {
                  let nested_level = if let CppTypeBase::TemplateParameter { nested_level, .. } =
                                            template_arguments[0].base {
                    nested_level
                  } else {
                    return Err("only template parameters can be here".into());
                  };
                  log::noisy("");
                  log::noisy(format!("method: {}", method.short_text()));
                  log::noisy(format!("found template instantiations: {:?}",
                                     template_instantiations));
                  match apply_instantiations_to_method(method,
                                                       nested_level,
                                                       &template_instantiations.instantiations) {
                    Ok(methods) => {
                      for method in methods {
                        let mut ok = true;
                        for type1 in method.all_involved_types() {
                          match self.check_template_type(&type1) {
                            Ok(_) => {}
                            Err(msg) => {
                              ok = false;
                              log::noisy(format!("method is not accepted: {}",
                                                 method.short_text()));
                              log::noisy(format!("  {}", msg));
                            }
                          }
                        }
                        if ok {
                          new_methods.push(method);
                        }
                      }
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
    Ok(())
  }

  pub fn add_field_accessors(&mut self) -> Result<()> {
    let mut new_methods = Vec::new();
    for type_info in &self.types {
      if let CppTypeKind::Class { ref fields, .. } = type_info.kind {
        for field in fields {
          let create_method =
            |name, accessor_type, return_type, arguments| -> Result<CppMethod> {
              Ok(CppMethod {
                name: name,
                class_membership: Some(CppMethodClassMembership {
                  class_type: try!(type_info.default_class_type()),
                  kind: CppMethodKind::Regular,
                  is_virtual: false,
                  is_pure_virtual: false,
                  is_const: match accessor_type {
                    CppFieldAccessorType::CopyGetter |
                    CppFieldAccessorType::ConstRefGetter => true,
                    CppFieldAccessorType::MutRefGetter |
                    CppFieldAccessorType::Setter => false,
                  },
                  is_static: false,
                  visibility: CppVisibility::Public,
                  is_signal: false,
                  is_slot: false,
                  fake: Some(FakeCppMethod::FieldAccessor {
                    accessor_type: accessor_type,
                    field_name: field.name.clone(),
                  }),
                }),
                operator: None,
                return_type: return_type,
                arguments: arguments,
                arguments_before_omitting: None,
                allows_variadic_arguments: false,
                include_file: type_info.include_file.clone(),
                origin_location: None,
                template_arguments: None,
                template_arguments_values: None,
                declaration_code: None,
                doc: None,
                inheritance_chain: Vec::new(),
                is_ffi_whitelisted: false,
                is_unsafe_static_cast: false,
              })
            };
          if field.visibility == CppVisibility::Public {
            if field.field_type.indirection == CppTypeIndirection::None &&
               field.field_type.base.is_class() {

              let mut type2_const = field.field_type.clone();
              type2_const.is_const = true;
              type2_const.indirection = CppTypeIndirection::Ref;
              let mut type2_mut = field.field_type.clone();
              type2_mut.is_const = false;
              type2_mut.indirection = CppTypeIndirection::Ref;
              new_methods.push(try!(create_method(field.name.clone(),
                                                  CppFieldAccessorType::ConstRefGetter,
                                                  type2_const,
                                                  Vec::new())));
              new_methods.push(try!(create_method(format!("{}_mut", field.name),
                                                  CppFieldAccessorType::MutRefGetter,
                                                  type2_mut,
                                                  Vec::new())));
            } else {
              new_methods.push(try!(create_method(field.name.clone(),
                                                  CppFieldAccessorType::CopyGetter,
                                                  field.field_type.clone(),
                                                  Vec::new())));
            }
            let arg = CppFunctionArgument {
              argument_type: field.field_type.clone(),
              name: "value".to_string(),
              has_default_value: false,
            };
            new_methods.push(try!(create_method(format!("set_{}", field.name),
                                                CppFieldAccessorType::Setter,
                                                CppType::void(),
                                                vec![arg])));
          }
        }
      }
    }
    self.methods.append(&mut new_methods);
    Ok(())
  }

  fn add_casts_one(&self,
                   target_type: &CppTypeClassBase,
                   base_type: &CppType)
                   -> Result<Vec<CppMethod>> {
    let type_info = try!(self.find_type_info(|x| x.name == target_type.name)
      .chain_err(|| "type info not found"));
    let target_ptr_type = CppType {
      base: CppTypeBase::Class(target_type.clone()),
      indirection: CppTypeIndirection::Ptr,
      is_const: false,
      is_const2: false,
    };
    let base_ptr_type = CppType {
      base: base_type.base.clone(),
      indirection: CppTypeIndirection::Ptr,
      is_const: false,
      is_const2: false,
    };
    let create_method =
      |name: &str, from: &CppType, to: &CppType, is_unsafe_static_cast: bool| {
        CppMethod {
          name: name.to_string(),
          class_membership: None,
          operator: None,
          return_type: to.clone(),
          arguments: vec![CppFunctionArgument {
                            name: "ptr".to_string(),
                            argument_type: from.clone(),
                            has_default_value: false,
                          }],
          arguments_before_omitting: None,
          allows_variadic_arguments: false,
          include_file: type_info.include_file.clone(),
          origin_location: None,
          template_arguments: None,
          template_arguments_values: Some(vec![to.clone()]),
          declaration_code: None,
          doc: None,
          inheritance_chain: Vec::new(),
          is_ffi_whitelisted: true,
          is_unsafe_static_cast: is_unsafe_static_cast,
        }
      };
    let mut new_methods = Vec::new();
    new_methods.push(create_method("static_cast", &base_ptr_type, &target_ptr_type, true));
    new_methods.push(create_method("static_cast", &target_ptr_type, &base_ptr_type, false));
    if let CppTypeBase::Class(ref base) = base_type.base {
      if self.is_polymorphic_type(&base.name) {
        new_methods.push(create_method("dynamic_cast", &base_ptr_type, &target_ptr_type, false));
      }
    }

    if let CppTypeBase::Class(ref base) = base_type.base {
      if let Some(type_info) = self.find_type_info(|x| x.name == base.name) {
        if let CppTypeKind::Class { ref bases, .. } = type_info.kind {
          for base in bases {
            new_methods.append(&mut try!(self.add_casts_one(target_type, &base.base_type)));
          }
        }
      }
    }
    Ok(new_methods)
  }

  fn add_casts(&mut self) -> Result<()> {
    let mut new_methods = Vec::new();
    for type_info in &self.types {
      if let CppTypeKind::Class { ref bases, .. } = type_info.kind {
        let t = try!(type_info.default_class_type());
        for base in bases {
          new_methods.append(&mut try!(self.add_casts_one(&t, &base.base_type)));
        }
      }
    }
    self.methods.append(&mut new_methods);
    Ok(())
  }

  /// Checks if `class_name` types inherits `base_name` type directly or indirectly.
  pub fn inherits(&self, class_name: &str, base_name: &str) -> bool {
    for types in self.dependencies.iter().map(|x| &x.types).chain(once(&self.types)) {
      if let Some(info) = types.iter().find(|x| &x.name == class_name) {
        if let CppTypeKind::Class { ref bases, .. } = info.kind {
          for base1 in bases {
            if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = base1.base_type.base {
              if name == base_name {
                return true;
              }
              if self.inherits(name, base_name) {
                return true;
              }
            }
          }
        }
      }
    }
    false
  }

  fn detect_signals_and_slots(&mut self) -> Result<()> {
    let mut files = HashSet::new();
    for type1 in &self.types {
      if self.inherits(&type1.name, "QObject") &&
         !files.contains(&type1.origin_location.include_file_path) {
        files.insert(type1.origin_location.include_file_path.clone());
      }
    }

    #[derive(Debug, Clone)]
    enum SectionType {
      Signals,
      Slots,
      Other,
    }
    #[derive(Debug)]
    struct Section {
      line: i32,
      section_type: SectionType,
    }

    if files.is_empty() {
      return Ok(());
    }
    log::info("Detecting signals and slots");
    let re_signals = try!(Regex::new(r"(signals|Q_SIGNALS)\s*:"));
    let re_slots = try!(Regex::new(r"(slots|Q_SLOTS)\s*:"));
    let re_other = try!(Regex::new(r"(public|protected|private)\s*:"));
    let mut sections = HashMap::new();

    for file_path in files {
      let mut file_sections = Vec::new();
      log::info(format!("File: {}", &file_path));
      let file = try!(open_file(&file_path));
      let reader = BufReader::new(file.into_file());
      for (line_num, line) in reader.lines().enumerate() {
        let line =
          try!(line.chain_err(|| format!("failed while reading lines from {}", &file_path)));
        let section_type = if re_signals.is_match(&line) {
          Some(SectionType::Signals)
        } else if re_slots.is_match(&line) {
          Some(SectionType::Slots)
        } else if re_other.is_match(&line) {
          Some(SectionType::Other)
        } else {
          None
        };
        if let Some(section_type) = section_type {
          file_sections.push(Section {
            line: line_num as i32,
            section_type: section_type,
          });
        }
      }
      println!("sections: {:?}", file_sections);
      if !file_sections.is_empty() {
        sections.insert(file_path, file_sections);
      }
    }
    let mut all_types = HashSet::new();
    for type1 in &self.types {
      if let Some(sections) = sections.get(&type1.origin_location.include_file_path) {
        let sections: Vec<_> =
          sections.iter().filter(|x| x.line >= type1.origin_location.line as i32 - 1).collect();
        for method in &mut self.methods {
          let mut section_type = SectionType::Other;
          if let Some(ref info) = method.class_membership {
            if info.class_type.name == type1.name {
              if let Some(ref location) = method.origin_location {
                let matching_sections: Vec<_> = sections.clone()
                  .into_iter()
                  .filter(|x| x.line <= location.line as i32 - 1)
                  .collect();
                if !matching_sections.is_empty() {
                  let section = matching_sections[matching_sections.len() - 1];
                  section_type = section.section_type.clone();
                  match section.section_type {
                    SectionType::Signals => {
                      log::info(format!("Found signal: {}", method.short_text()));
                      let types: Vec<_> =
                        method.arguments.iter().map(|x| x.argument_type.clone()).collect();
                      if !all_types.contains(&types) &&
                         !self.dependencies
                        .iter()
                        .any(|d| d.signal_argument_types.iter().any(|t| t == &types)) {
                        all_types.insert(types);
                      }
                    }
                    SectionType::Slots => {
                      log::info(format!("Found slot: {}", method.short_text()));
                    }
                    SectionType::Other => {}
                  }
                }
              }
            }
          }
          if let Some(ref mut info) = method.class_membership {
            match section_type {
              SectionType::Signals => {
                info.is_signal = true;
              }
              SectionType::Slots => {
                info.is_slot = true;
              }
              SectionType::Other => {}
            }
          }

        }
      }
    }

    let mut types_with_omitted_args = HashSet::new();
    for t in &all_types {
      let mut types = t.clone();
      while let Some(_) = types.pop() {
        if !types_with_omitted_args.contains(&types) && !all_types.contains(&types) &&
           !self.dependencies.iter().any(|d| d.signal_argument_types.iter().any(|t| t == &types)) {
          types_with_omitted_args.insert(types.clone());
        }
      }
    }
    all_types.extend(types_with_omitted_args.into_iter());


    //    if all_types.is_empty() {
    //      return Ok(());
    //    }
    println!("Signal argument types:");
    for t in &all_types {
      println!("  ({})",
               t.iter().map(|x| x.to_cpp_pseudo_code()).join(", "));
    }
    self.signal_argument_types = all_types.into_iter().collect();
    Ok(())
  }

  /// Performs data conversion to make it more suitable
  /// for further wrapper generation.
  pub fn post_process(&mut self) -> Result<()> {
    try!(self.detect_signals_and_slots());
    try!(self.ensure_explicit_destructors());
    self.generate_methods_with_omitted_args();
    try!(self.instantiate_templates());
    try!(self.add_inherited_methods()); // TODO: add inherited fields too
    try!(self.add_field_accessors()); // TODO: fix doc generator for field accessors
    try!(self.add_casts());
    Ok(())
  }
}

impl TemplateArgumentsDeclaration {
  #[allow(dead_code)]
  pub fn count(&self) -> i32 {
    self.names.len() as i32
  }
}
