/*

impl<'a> CppDataWithDeps<'a> {

  /// Helper function that performs a portion of add_inherited_methods implementation.
  fn inherited_methods_from(&self,
                            base_name: &str,
                            all_base_methods: &[&CppMethod])
                            -> Result<Vec<CppMethod>> {
    // TODO: speed up this method (#12)
    let mut new_methods = Vec::new();
    {
      for type1 in &self.current.parser.types {
        if let CppTypeKind::Class {
          ref bases,
          ref using_directives,
          ..
        } = type1.kind {
          for base in bases {
            if let CppTypeBase::Class(CppTypeClassBase {
                                        ref name,
                                        ref template_arguments,
                                      }) = base.base_type.base {
              if name == base_name {
                log::llog(log::DebugInheritance, || {
                  format!("Adding inherited methods from {} to {}",
                          base_name,
                          type1.name)
                });
                let derived_name = &type1.name;
                let base_template_arguments = template_arguments;
                let base_methods = all_base_methods
                    .into_iter()
                    .filter(|method| if let Some(ref info) = method.class_membership {
                      &info.class_type.template_arguments == base_template_arguments
                    } else {
                      false
                    });
                let mut current_new_methods = Vec::new();
                for base_class_method in base_methods {
                  let mut using_directive_enables = false;
                  let mut using_directive_disables = false;
                  for dir in using_directives {
                    if &dir.method_name == &base_class_method.name {
                      if &dir.class_name == base_name {
                        log::llog(log::DebugInheritance, || {
                          format!("UsingDirective enables inheritance of {}",
                                  base_class_method.short_text())
                        });
                        using_directive_enables = true;
                      } else {
                        log::llog(log::DebugInheritance, || {
                          format!("UsingDirective disables inheritance of {}",
                                  base_class_method.short_text())
                        });
                        using_directive_disables = true;
                      }
                    }
                  }
                  if using_directive_disables {
                    continue;
                  }

                  let mut ok = true;
                  for method in self.current.all_methods() {
                    if method.class_name() == Some(derived_name) &&
                        method.name == base_class_method.name {
                      // without using directive, any method with the same name
                      // disables inheritance of base class method;
                      // with using directive, only method with the same arguments
                      // disables inheritance of base class method.
                      if !using_directive_enables ||
                          method.argument_types_equal(base_class_method) {
                        log::llog(log::DebugInheritance,
                                  || "Method is not added because it's overriden in derived class");
                        log::llog(log::DebugInheritance,
                                  || format!("Base method: {}", base_class_method.short_text()));
                        log::llog(log::DebugInheritance,
                                  || format!("Derived method: {}\n", method.short_text()));
                        ok = false;
                      }
                      break;
                    }
                  }
                  if ok {
                    let mut new_method: CppMethod = (*base_class_method).clone();
                    if let Some(ref mut info) = new_method.class_membership {
                      info.class_type = type1.default_class_type()?;
                    } else {
                      return Err(unexpected("no class membership").into());
                    }
                    new_method.include_file = type1.include_file.clone();
                    new_method.origin_location = None;
                    new_method.declaration_code = None;
                    new_method.inheritance_chain.push(base.clone());
                    new_method.is_fake_inherited_method = true;
                    log::llog(log::DebugInheritance,
                              || format!("Method added: {}", new_method.short_text()));
                    log::llog(log::DebugInheritance, || {
                      format!("Base method: {} ({:?})\n",
                              base_class_method.short_text(),
                              base_class_method.origin_location)
                    });
                    current_new_methods.push(new_method.clone());
                  }
                }
                new_methods.append(&mut self
                    .inherited_methods_from(derived_name,
                                            &current_new_methods
                                                .iter()
                                                .collect::<Vec<_>>())?);
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
    log::status("Adding inherited methods");
    let mut all_new_methods = Vec::new();
    for (is_self, cpp_data) in
        self
            .dependencies
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
            let base_methods = cpp_data2
                .methods
                .iter()
                .filter(|method| if let Some(ref info) = method.class_membership {
                  &info.class_type.name == &type1.name && !info.kind.is_constructor() &&
                      !info.kind.is_destructor() &&
                      method.operator != Some(CppOperator::Assignment)
                } else {
                  false
                });
            all_new_methods.append(&mut self
                .inherited_methods_from(&type1.name,
                                        &base_methods
                                            .collect::<Vec<_>>())?);
          }
        }
      }
    }
    while let Some(method) = all_new_methods.pop() {
      let mut duplicates = Vec::new();
      while let Some(index) =
      all_new_methods
          .iter()
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
        if duplicates
            .iter()
            .find(|m| m.inheritance_chain.last() != duplicates[0].inheritance_chain.last())
            .is_none() {
          // all methods are inherited from one base class
          self.methods.append(&mut duplicates);
        } else {
          let signature_mismatch = duplicates
              .iter()
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
            if duplicates
                .iter()
                .all(|x| {
                  x.inheritance_chain[0].is_virtual &&
                      &x.inheritance_chain[0].base_type == first_base
                }) {
              allow_method = true;
            }
          }
          if allow_method {
            log::llog(log::DebugInheritance,
                      || "Allowing duplicated inherited method (virtual diamond inheritance)");
            log::llog(log::DebugInheritance, || duplicates[0].short_text());
            for duplicate in &duplicates {
              log::llog(log::DebugInheritance,
                        || format!("  {}", duplicate.inheritance_chain_text()));
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
            log::llog(log::DebugInheritance,
                      || "Removed ambiguous inherited methods (presumed inaccessible):");
            if signature_mismatch {
              for duplicate in &duplicates {
                log::llog(log::DebugInheritance,
                          || format!("  {}", duplicate.short_text()));
                log::llog(log::DebugInheritance,
                          || format!("  {}", duplicate.inheritance_chain_text()));
              }
            } else {
              log::llog(log::DebugInheritance, || duplicates[0].short_text());
              for duplicate in &duplicates {
                log::llog(log::DebugInheritance,
                          || format!("  {}", duplicate.inheritance_chain_text()));
              }
            }
          }
        }
      }
    }
    Ok(())
  }

  /// Checks if `class_name` types inherits `base_name` type directly or indirectly.
  pub fn inherits(&self, class_name: &str, base_name: &str) -> bool {
    for types in self.all_types() {
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
*/
