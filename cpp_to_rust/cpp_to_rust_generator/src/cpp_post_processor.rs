use common::errors::{unexpected, Result};
use common::log;
use cpp_data::{
  CppBaseSpecifier, CppData, CppDataWithDeps, CppTemplateInstantiation, CppTemplateInstantiations,
  CppTypeAllocationPlace, CppTypeData, CppTypeKind, CppVisibility, ParserCppData,
};
use cpp_method::{CppMethod, CppMethodClassMembership, CppMethodKind};
use cpp_type::{CppType, CppTypeBase, CppTypeClassBase, CppTypeIndirection};

use common::string_utils::JoinWithSeparator;
use std::collections::{HashMap, HashSet};
use std::iter::once;

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
                  instantiations: vec![CppTemplateInstantiation {
                    template_arguments: template_arguments.clone(),
                  }],
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
}
