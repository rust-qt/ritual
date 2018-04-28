use cpp_data::{CppBaseSpecifier, CppData, CppDataWithDeps, CppTemplateInstantiation,
               CppTemplateInstantiations, CppTypeAllocationPlace, CppTypeData, CppTypeKind,
               CppVisibility, ParserCppData, ProcessedCppData};
use cpp_method::{CppMethod, CppMethodClassMembership, CppMethodKind};
use cpp_type::{CppType, CppTypeBase, CppTypeClassBase, CppTypeIndirection};
use common::log;
use common::errors::{unexpected, Result};

use std::collections::{HashMap, HashSet};
use std::iter::once;
use common::string_utils::JoinWithSeparator;

struct CppPostProcessor<'a> {
  parser_data: ParserCppData,
  dependencies: Vec<&'a CppData>,
}
/*
/// Derives `ProcessedCppData` from `ParserCppData`.
pub fn cpp_post_process<'a>(
  parser_data: ParserCppData,
  dependencies: Vec<&'a CppData>,
  allocation_place_overrides: &HashMap<String, CppTypeAllocationPlace>,
) -> Result<CppDataWithDeps<'a>> {
  let processor = CppPostProcessor {
    parser_data: parser_data,
    dependencies: dependencies,
  };

  let inherited_methods = processor.detect_inherited_methods2()?;
  let implicit_destructors = processor.ensure_explicit_destructors(&inherited_methods)?;
  let type_allocation_places =
    processor.choose_allocation_places(allocation_place_overrides, &inherited_methods)?;

  let result = ProcessedCppData {
    implicit_destructors: implicit_destructors,
    template_instantiations: processor.find_template_instantiations(),
    inherited_methods: inherited_methods,
    signal_argument_types: processor.detect_signal_argument_types()?,
    type_allocation_places: type_allocation_places,
  };

  Ok(CppDataWithDeps {
    current: CppData {
      parser: processor.parser_data,
      processed: result,
    },
    dependencies: processor.dependencies,
  })
}
*/
impl<'a> CppPostProcessor<'a> {
  /// Checks if specified class has virtual destructor (own or inherited).
  pub fn has_virtual_destructor(&self, class_name: &str, inherited_methods: &[CppMethod]) -> bool {
    for method in self
      .parser_data
      .methods
      .iter()
      .chain(inherited_methods.iter())
    {
      if let Some(ref info) = method.class_membership {
        if info.kind == CppMethodKind::Destructor && &info.class_type.name == class_name {
          return info.is_virtual;
        }
      }
    }
    false
  }
  /// Checks if specified class has any virtual methods (own or inherited).
  pub fn has_virtual_methods(&self, class_name: &str, inherited_methods: &[CppMethod]) -> bool {
    for method in self
      .parser_data
      .methods
      .iter()
      .chain(inherited_methods.iter())
    {
      if let Some(ref info) = method.class_membership {
        if &info.class_type.name == class_name && info.is_virtual {
          return true;
        }
      }
    }
    false
  }
  /*
  /// Adds destructors for every class that does not have explicitly
  /// defined destructor, allowing to create wrappings for
  /// destructors implicitly available in C++.
  fn ensure_explicit_destructors(&self, inherited_methods: &[CppMethod]) -> Result<Vec<CppMethod>> {
    let mut methods = Vec::new();
    for type1 in &self.parser_data.types {
      if let CppTypeKind::Class { .. } = type1.kind {
        let class_name = &type1.name;
        let found_destructor = self
          .parser_data
          .methods
          .iter()
          .any(|m| m.is_destructor() && m.class_name() == Some(class_name));
        if !found_destructor {
          let is_virtual = self.has_virtual_destructor(class_name, inherited_methods);
          methods.push(CppMethod {
            name: format!("~{}", class_name),
            class_membership: Some(CppMethodClassMembership {
              class_type: type1.default_class_type()?,
              is_virtual: is_virtual,
              is_pure_virtual: false,
              is_const: false,
              is_static: false,
              visibility: CppVisibility::Public,
              is_signal: false,
              is_slot: false,
              kind: CppMethodKind::Destructor,
            }),
            operator: None,
            return_type: CppType::void(),
            arguments: vec![],
            allows_variadic_arguments: false,
            include_file: type1.include_file.clone(),
            origin_location: None,
            template_arguments: None,
            template_arguments_values: None,
            declaration_code: None,
            doc: None,
            inheritance_chain: Vec::new(),
            //is_fake_inherited_method: false,
            is_ffi_whitelisted: false,
          });
        }
      }
    }
    Ok(methods)
  }
*/
  /// Searches for template instantiations in this library's API,
  /// excluding results that were already processed in dependencies.
  #[cfg_attr(feature = "clippy", allow(block_in_if_condition_stmt))]
  fn find_template_instantiations(&self) -> Vec<CppTemplateInstantiations> {
    fn check_type(type1: &CppType, deps: &[&CppData], result: &mut Vec<CppTemplateInstantiations>) {
      if let CppTypeBase::Class(CppTypeClassBase {
        ref name,
        ref template_arguments,
      }) = type1.base
      {
        if let Some(ref template_arguments) = *template_arguments {
          if !template_arguments
            .iter()
            .any(|x| x.base.is_or_contains_template_parameter())
          {
            if !deps.iter().any(|data| {
              data.processed.template_instantiations.iter().any(|i| {
                &i.class_name == name
                  && i.instantiations
                    .iter()
                    .any(|x| &x.template_arguments == template_arguments)
              })
            }) {
              if !result.iter().any(|x| &x.class_name == name) {
                log::llog(log::DebugParser, || {
                  format!(
                    "Found template instantiation: {}<{:?}>",
                    name, template_arguments
                  )
                });
                result.push(CppTemplateInstantiations {
                  class_name: name.clone(),
                  instantiations: vec![
                    CppTemplateInstantiation {
                      template_arguments: template_arguments.clone(),
                    },
                  ],
                });
              } else {
                let item = result
                  .iter_mut()
                  .find(|x| &x.class_name == name)
                  .expect("previously found");
                if !item
                  .instantiations
                  .iter()
                  .any(|x| &x.template_arguments == template_arguments)
                {
                  log::llog(log::DebugParser, || {
                    format!(
                      "Found template instantiation: {}<{:?}>",
                      name, template_arguments
                    )
                  });
                  item.instantiations.push(CppTemplateInstantiation {
                    template_arguments: template_arguments.clone(),
                  });
                }
              }
            }
          }
          for arg in template_arguments {
            check_type(arg, deps, result);
          }
        }
      }
    }
    let mut result = Vec::new();
    for m in &self.parser_data.methods {
      check_type(&m.return_type, &self.dependencies, &mut result);
      for arg in &m.arguments {
        check_type(&arg.argument_type, &self.dependencies, &mut result);
      }
    }
    for t in &self.parser_data.types {
      if let CppTypeKind::Class {
        ref bases,
        ref fields,
        ..
      } = t.kind
      {
        for base in bases {
          check_type(&base.base_type, &self.dependencies, &mut result);
        }
        for field in fields {
          check_type(&field.field_type, &self.dependencies, &mut result);
        }
      }
    }
    result
  }

  fn detect_inherited_methods2(&self) -> Result<Vec<CppMethod>> {
    let mut remaining_classes: Vec<&CppTypeData> = self
      .parser_data
      .types
      .iter()
      .filter(|t| {
        if let CppTypeKind::Class { ref bases, .. } = t.kind {
          !bases.is_empty()
        } else {
          false
        }
      })
      .collect();
    let mut ordered_classes = Vec::new();
    while !remaining_classes.is_empty() {
      let mut any_added = false;
      let mut remaining_classes2 = Vec::new();
      for class in &remaining_classes {
        if let CppTypeKind::Class { ref bases, .. } = class.kind {
          if bases.iter().any(|base| {
            if base.visibility != CppVisibility::Private
              && base.base_type.indirection == CppTypeIndirection::None
            {
              if let CppTypeBase::Class(ref base_info) = base.base_type.base {
                remaining_classes.iter().any(|c| c.name == base_info.name)
              } else {
                false
              }
            } else {
              false
            }
          }) {
            remaining_classes2.push(*class);
          } else {
            ordered_classes.push(*class);
            any_added = true;
          }
        } else {
          unreachable!()
        }
      }
      remaining_classes = remaining_classes2;
      if !any_added {
        return Err("Cyclic dependency detected while detecting inherited methods".into());
      }
    }

    let mut result = Vec::new();
    for class in ordered_classes {
      log::llog(log::DebugInheritance, || {
        format!("Detecting inherited methods for {}\n", class.name)
      });
      let own_methods: Vec<&CppMethod> = self
        .parser_data
        .methods
        .iter()
        .filter(|m| m.class_name() == Some(&class.name))
        .collect();
      let bases = if let CppTypeKind::Class { ref bases, .. } = class.kind {
        bases
      } else {
        unreachable!()
      };
      let bases_with_methods: Vec<(&CppBaseSpecifier, Vec<&CppMethod>)> = bases
        .iter()
        .filter(|base| {
          base.visibility != CppVisibility::Private
            && base.base_type.indirection == CppTypeIndirection::None
        })
        .map(|base| {
          let methods = if let CppTypeBase::Class(ref base_class_base) = base.base_type.base {
            once(&self.parser_data)
              .chain(self.dependencies.iter().map(|d| &d.parser))
              .map(|p| &p.methods)
              .flat_map(|m| m)
              .filter(|m| {
                if let Some(ref info) = m.class_membership {
                  &info.class_type == base_class_base
                } else {
                  false
                }
              })
              .collect()
          } else {
            Vec::new()
          };
          (base, methods)
        })
        .filter(|x| !x.1.is_empty())
        .collect();

      for &(ref base, ref methods) in &bases_with_methods {
        if let CppTypeBase::Class(ref base_class_base) = base.base_type.base {
          for method in methods {
            if let CppTypeKind::Class {
              ref using_directives,
              ..
            } = class.kind
            {
              let use_method = if using_directives
                .iter()
                .any(|dir| dir.class_name == base_class_base.name && dir.method_name == method.name)
              {
                true // excplicitly inherited with a using directive
              } else if own_methods.iter().any(|m| m.name == method.name) {
                // not inherited because method with the same name exists in the derived class
                false
              } else if bases_with_methods.iter().any(|&(ref base2, ref methods2)| {
                base != base2 && methods2.iter().any(|m| m.name == method.name)
              }) {
                // not inherited because method with the same name exists in one of
                // the other bases
                false
              } else {
                // no aliased method found and no using directives
                true
              };
              // TODO: detect diamond inheritance
              if use_method {
                let mut new_method = (*method).clone();
                if let Some(ref mut info) = new_method.class_membership {
                  info.class_type = class.default_class_type()?;
                } else {
                  return Err(unexpected("no class membership").into());
                }
                new_method.include_file = class.include_file.clone();
                new_method.origin_location = None;
                new_method.declaration_code = None;
                new_method.inheritance_chain.push((*base).clone());
                //new_method.is_fake_inherited_method = true;
                log::llog(log::DebugInheritance, || {
                  format!("Method added: {}", new_method.short_text())
                });
                log::llog(log::DebugInheritance, || {
                  format!(
                    "Base method: {} ({:?})\n",
                    method.short_text(),
                    method.origin_location
                  )
                });
                result.push(new_method);
              }
            } else {
              unreachable!()
            }
          }
        } else {
          unreachable!()
        }
      }
    }
    Ok(result)
  }

  fn detect_signal_argument_types(&self) -> Result<Vec<Vec<CppType>>> {
    let mut all_types = HashSet::new();
    for method in &self.parser_data.methods {
      if let Some(ref method_info) = method.class_membership {
        if method_info.is_signal {
          let types: Vec<_> = method
            .arguments
            .iter()
            .map(|x| x.argument_type.clone())
            .collect();
          if !all_types.contains(&types) && !self.dependencies.iter().any(|d| {
            d.processed
              .signal_argument_types
              .iter()
              .any(|t| t == &types)
          }) {
            all_types.insert(types);
          }
        }
      }
    }

    let mut types_with_omitted_args = HashSet::new();
    for t in &all_types {
      let mut types = t.clone();
      while let Some(_) = types.pop() {
        if !types_with_omitted_args.contains(&types) && !all_types.contains(&types)
          && !self.dependencies.iter().any(|d| {
            d.processed
              .signal_argument_types
              .iter()
              .any(|t| t == &types)
          }) {
          types_with_omitted_args.insert(types.clone());
        }
      }
    }
    all_types.extend(types_with_omitted_args.into_iter());

    log::llog(log::DebugSignals, || "Signal argument types:");
    for t in &all_types {
      log::llog(log::DebugSignals, || {
        format!(
          "  ({})",
          t.iter().map(|x| x.to_cpp_pseudo_code()).join(", ")
        )
      });
    }
    Ok(all_types.into_iter().collect())
  }

  /// Detects the preferred type allocation place for each type based on
  /// API of all known methods. Keys of `overrides` are C++ type names.
  /// If `overrides` contains type allocation place for a type, it's used instead of
  /// the place that would be automatically selected.
  pub fn choose_allocation_places(
    &self,
    overrides: &HashMap<String, CppTypeAllocationPlace>,
    inherited_methods: &[CppMethod],
  ) -> Result<HashMap<String, CppTypeAllocationPlace>> {
    log::status("Detecting type allocation places");

    #[derive(Default)]
    struct TypeStats {
      // has_derived_classes: bool,
      has_virtual_methods: bool,
      pointers_count: usize,
      not_pointers_count: usize,
    };
    fn check_type(cpp_type: &CppType, data: &mut HashMap<String, TypeStats>) {
      if let CppTypeBase::Class(CppTypeClassBase {
        ref name,
        ref template_arguments,
      }) = cpp_type.base
      {
        if !data.contains_key(name) {
          data.insert(name.clone(), TypeStats::default());
        }
        match cpp_type.indirection {
          CppTypeIndirection::None | CppTypeIndirection::Ref => {
            data.get_mut(name).unwrap().not_pointers_count += 1
          }
          CppTypeIndirection::Ptr => data.get_mut(name).unwrap().pointers_count += 1,
          _ => {}
        }
        if let Some(ref args) = *template_arguments {
          for arg in args {
            check_type(arg, data);
          }
        }
      }
    }

    let mut data = HashMap::new();
    for type1 in &self.parser_data.types {
      if self.has_virtual_methods(&type1.name, inherited_methods) {
        if !data.contains_key(&type1.name) {
          data.insert(type1.name.clone(), TypeStats::default());
        }
        data.get_mut(&type1.name).unwrap().has_virtual_methods = true;
      }
    }
    for method in &self.parser_data.methods {
      check_type(&method.return_type, &mut data);
      for arg in &method.arguments {
        check_type(&arg.argument_type, &mut data);
      }
    }
    let mut results = HashMap::new();
    {
      let mut logger = log::default_logger();
      if logger.is_on(log::DebugAllocationPlace) {
        for (name, stats) in &data {
          logger.log(
            log::DebugAllocationPlace,
            format!(
              "{}\t{}\t{}\t{}",
              name, stats.has_virtual_methods, stats.pointers_count, stats.not_pointers_count
            ),
          );
        }
      }
    }

    for type1 in &self.parser_data.types {
      if !type1.is_class() {
        continue;
      }
      let name = &type1.name;
      let result = if overrides.contains_key(name) {
        overrides[name].clone()
      } else if let Some(ref stats) = data.get(name) {
        if stats.has_virtual_methods {
          CppTypeAllocationPlace::Heap
        } else if stats.pointers_count == 0 {
          CppTypeAllocationPlace::Stack
        } else {
          let min_safe_data_count = 5;
          let min_not_pointers_percent = 0.3;
          if stats.pointers_count + stats.not_pointers_count < min_safe_data_count {
            log::llog(log::DebugAllocationPlace, || {
              format!("Can't determine type allocation place for '{}':", name)
            });
            log::llog(log::DebugAllocationPlace, || {
              format!(
                "  Not enough data (pointers={}, not pointers={})",
                stats.pointers_count, stats.not_pointers_count
              )
            });
          } else if stats.not_pointers_count as f32
            / (stats.pointers_count + stats.not_pointers_count) as f32
            > min_not_pointers_percent
          {
            log::llog(log::DebugAllocationPlace, || {
              format!("Can't determine type allocation place for '{}':", name)
            });
            log::llog(log::DebugAllocationPlace, || {
              format!(
                "  Many not pointers (pointers={}, not pointers={})",
                stats.pointers_count, stats.not_pointers_count
              )
            });
          }
          CppTypeAllocationPlace::Heap
        }
      } else {
        log::llog(log::DebugAllocationPlace, || {
          format!(
            "Can't determine type allocation place for '{}' (no data)",
            name
          )
        });
        CppTypeAllocationPlace::Heap
      };
      results.insert(name.clone(), result);
    }
    log::llog(log::DebugAllocationPlace, || {
      format!(
        "Allocation place is heap for: {}",
        results
          .iter()
          .filter(|&(_, v)| v == &CppTypeAllocationPlace::Heap)
          .map(|(k, _)| k)
          .join(", ")
      )
    });
    log::llog(log::DebugAllocationPlace, || {
      format!(
        "Allocation place is stack for: {}",
        results
          .iter()
          .filter(|&(_, v)| v == &CppTypeAllocationPlace::Stack)
          .map(|(k, _)| k)
          .join(", ")
      )
    });

    Ok(results)
  }
}
