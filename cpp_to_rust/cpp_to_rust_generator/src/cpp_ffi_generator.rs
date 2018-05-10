use caption_strategy::{MethodCaptionStrategy, TypeCaptionStrategy};
use cpp_data::{CppDataWithDeps, CppOperator, CppTemplateInstantiation, CppTypeAllocationPlace,
               CppVisibility};
use cpp_type::{CppFunctionPointerType, CppType, CppTypeBase, CppTypeClassBase, CppTypeIndirection,
               CppTypeRole};
use cpp_ffi_data::{c_base_name, CppCast, CppFfiMethod, CppFfiMethodKind, CppFieldAccessorType,
                   QtSlotWrapper};
use cpp_method::{CppMethod, CppMethodArgument, CppMethodClassMembership, CppMethodKind,
                 ReturnValueAllocationPlace};
use common::errors::{unexpected, ChainErr, Result};
use common::log;
use common::utils::{add_to_multihash, MapIfOk};
use config::CppFfiGeneratorFilterFn;
use std::collections::{HashMap, HashSet};
use std::iter::once;
use new_impl::database::CppItemData;
use new_impl::processor::ProcessorData;
use new_impl::processor::ProcessorItem;
use cpp_data::CppClassField;
use cpp_data::CppBaseSpecifier;
use cpp_data::CppTypeData;

/// This object generates the C++ wrapper library
struct CppFfiGenerator<'a> {
  /// Input C++ data
  data: ProcessorData<'a>,
  /// Name of the wrapper library
  cpp_ffi_lib_name: String,
}

#[derive(Debug, Clone)]
struct CppMethodRefWithKind<'a> {
  method: &'a CppMethod,
  kind: CppFfiMethodKind,
}

struct CppMethodWithKind {
  method: CppMethod,
  kind: CppFfiMethodKind,
}

impl CppMethodWithKind {
  fn as_ref<'a>(&'a self) -> CppMethodRefWithKind<'a> {
    CppMethodRefWithKind {
      method: &self.method,
      kind: self.kind.clone(),
    }
  }
}

/// Runs the FFI generator
pub fn run(data: ProcessorData) -> Result<()> {
  let cpp_ffi_lib_name = format!("{}_ffi", &data.config.crate_properties().name());
  let mut generator = CppFfiGenerator {
    data: data,
    cpp_ffi_lib_name,
  };

  //  let mut extra_methods = Vec::new();
  //  extra_methods.append(&mut instantiate_templates(&generator.cpp_data)?);
  //  extra_methods.append(&mut generate_field_accessors(&generator.cpp_data)?);
  //  extra_methods.append(&mut generate_casts(&generator.cpp_data)?);

  generator.process_methods()?;
  Ok(())
}

pub fn cpp_ffi_generator() -> ProcessorItem {
  ProcessorItem {
    name: "cpp_ffi_generator".to_string(),
    is_main: true,
    run_after: Vec::new(),
    function: run,
  }
}

/// Tries to apply each of `template_instantiations` to `method`.
/// Only types at the specified `nested_level` are replaced.
/// Returns `Err` if any of `template_instantiations` is incompatible
/// with the method.
fn apply_instantiations_to_method(
  method: &CppMethod,
  nested_level1: usize,
  template_instantiations: &[CppTemplateInstantiation],
) -> Result<Vec<CppMethod>> {
  let mut new_methods = Vec::new();
  for ins in template_instantiations {
    log::llog(log::DebugTemplateInstantiation, || {
      format!("instantiation: {:?}", ins.template_arguments)
    });
    let mut new_method = method.clone();
    if let Some(ref args) = method.template_arguments {
      if args.iter().enumerate().all(|(index1, arg)| {
        if let CppTypeBase::TemplateParameter {
          nested_level,
          index,
          ..
        } = arg.base
        {
          nested_level1 == nested_level && index1 == index
        } else {
          false
        }
      }) {
        if args.len() != ins.template_arguments.len() {
          return Err("template arguments count mismatch".into());
        }
        new_method.template_arguments = Some(ins.template_arguments.clone());
      }
    }
    new_method.arguments.clear();
    for arg in &method.arguments {
      new_method.arguments.push(CppMethodArgument {
        name: arg.name.clone(),
        has_default_value: arg.has_default_value,
        argument_type: arg
          .argument_type
          .instantiate(nested_level1, &ins.template_arguments)?,
      });
    }
    new_method.return_type = method
      .return_type
      .instantiate(nested_level1, &ins.template_arguments)?;
    if let Some(ref mut info) = new_method.class_membership {
      info.class_type = info
        .class_type
        .instantiate_class(nested_level1, &ins.template_arguments)?;
    }
    let mut conversion_type = None;
    if let Some(ref mut operator) = new_method.operator {
      if let CppOperator::Conversion(ref mut cpp_type) = *operator {
        let r = cpp_type.instantiate(nested_level1, &ins.template_arguments)?;
        *cpp_type = r.clone();
        conversion_type = Some(r);
      }
    }
    if new_method
      .all_involved_types()
      .iter()
      .any(|t| t.base.is_or_contains_template_parameter())
    {
      return Err(
        format!(
          "extra template parameters left: {}",
          new_method.short_text()
        ).into(),
      );
    } else {
      if let Some(conversion_type) = conversion_type {
        new_method.name = format!("operator {}", conversion_type.to_cpp_code(None)?);
      }
      log::llog(log::DebugTemplateInstantiation, || {
        format!("success: {}", new_method.short_text())
      });
      new_methods.push(new_method);
    }
  }
  Ok(new_methods)
}

/// Generates methods as template instantiations of
/// methods of existing template classes and existing template methods.
fn instantiate_templates(data: &CppDataWithDeps) -> Result<Vec<CppMethodWithKind>> {
  log::status("Instantiating templates");
  let mut new_methods = Vec::new();

  for cpp_data in data.dependencies.iter().chain(once(&&data.current)) {
    for method in cpp_data.methods_and_implicit_destructors() {
      for type1 in method.all_involved_types() {
        if let CppTypeBase::Class(CppTypeClassBase {
          ref name,
          ref template_arguments,
        }) = type1.base
        {
          if let Some(ref template_arguments) = *template_arguments {
            assert!(!template_arguments.is_empty());
            if template_arguments
              .iter()
              .all(|x| x.base.is_template_parameter())
            {
              if let Some(template_instantiations) = data
                .current
                .processed
                .template_instantiations
                .iter()
                .find(|x| &x.class_name == name)
              {
                let nested_level = if let CppTypeBase::TemplateParameter { nested_level, .. } =
                  template_arguments[0].base
                {
                  nested_level
                } else {
                  return Err("only template parameters can be here".into());
                };
                log::llog(log::DebugTemplateInstantiation, || "");
                log::llog(log::DebugTemplateInstantiation, || {
                  format!("method: {}", method.short_text())
                });
                log::llog(log::DebugTemplateInstantiation, || {
                  format!(
                    "found template instantiations: {:?}",
                    template_instantiations
                  )
                });
                match apply_instantiations_to_method(
                  method,
                  nested_level,
                  &template_instantiations.instantiations,
                ) {
                  Ok(methods) => {
                    for method in methods {
                      let mut ok = true;
                      for type1 in method.all_involved_types() {
                        match data.check_template_type(&type1) {
                          Ok(_) => {}
                          Err(msg) => {
                            ok = false;
                            log::llog(log::DebugTemplateInstantiation, || {
                              format!("method is not accepted: {}", method.short_text())
                            });
                            log::llog(log::DebugTemplateInstantiation, || format!("  {}", msg));
                          }
                        }
                      }
                      if ok {
                        new_methods.push(CppMethodWithKind {
                          method: method,
                          kind: CppFfiMethodKind::Real,
                        });
                      }
                    }
                    break;
                  }
                  Err(msg) => log::llog(log::DebugTemplateInstantiation, || {
                    format!("failed: {}", msg)
                  }),
                }
                break;
              }
            }
          }
        }
      }
    }
  }
  Ok(new_methods)
}

/// Adds fictional getter and setter methods for each known public field of each class.
fn generate_field_accessors(field: &CppClassField) -> Result<Vec<CppMethodWithKind>> {
  // TODO: fix doc generator for field accessors
  //log::status("Adding field accessors");
  let mut new_methods = Vec::new();
  let create_method = |name, accessor_type, return_type, arguments| -> Result<CppMethodWithKind> {
    // TODO: generate CppFfiMethod, don't generate CppMethod
    Ok(CppMethodWithKind {
      method: CppMethod {
        name: name,
        class_membership: Some(CppMethodClassMembership {
          class_type: field.class_type.clone(),
          kind: CppMethodKind::Regular,
          is_virtual: false,
          is_pure_virtual: false,
          is_const: match accessor_type {
            CppFieldAccessorType::CopyGetter | CppFieldAccessorType::ConstRefGetter => true,
            CppFieldAccessorType::MutRefGetter | CppFieldAccessorType::Setter => false,
          },
          is_static: false,
          visibility: CppVisibility::Public,
          is_signal: false,
          is_slot: false,
        }),
        operator: None,
        return_type: return_type,
        arguments: arguments,
        allows_variadic_arguments: false,
        template_arguments: None,
        declaration_code: None,
      },
      kind: CppFfiMethodKind::FieldAccessor {
        accessor_type: accessor_type,
      },
    })
  };
  if field.visibility == CppVisibility::Public {
    if field.field_type.indirection == CppTypeIndirection::None && field.field_type.base.is_class()
    {
      let mut type2_const = field.field_type.clone();
      type2_const.is_const = true;
      type2_const.indirection = CppTypeIndirection::Ref;
      let mut type2_mut = field.field_type.clone();
      type2_mut.is_const = false;
      type2_mut.indirection = CppTypeIndirection::Ref;
      new_methods.push(create_method(
        field.name.clone(),
        CppFieldAccessorType::ConstRefGetter,
        type2_const,
        Vec::new(),
      )?);
      new_methods.push(create_method(
        format!("{}_mut", field.name),
        CppFieldAccessorType::MutRefGetter,
        type2_mut,
        Vec::new(),
      )?);
    } else {
      new_methods.push(create_method(
        field.name.clone(),
        CppFieldAccessorType::CopyGetter,
        field.field_type.clone(),
        Vec::new(),
      )?);
    }
    let arg = CppMethodArgument {
      argument_type: field.field_type.clone(),
      name: "value".to_string(),
      has_default_value: false,
    };
    new_methods.push(create_method(
      format!("set_{}", field.name),
      CppFieldAccessorType::Setter,
      CppType::void(),
      vec![arg],
    )?);
  }

  Ok(new_methods)
}

/// Convenience function to create `CppMethod` object for
/// `static_cast` or `dynamic_cast` from type `from` to type `to`.
/// See `CppMethod`'s documentation for more information
/// about `is_unsafe_static_cast` and `is_direct_static_cast`.
fn create_cast_method(cast: CppCast, from: &CppType, to: &CppType) -> CppMethodWithKind {
  CppMethodWithKind {
    method: CppMethod {
      name: cast.cpp_method_name().to_string(),
      class_membership: None,
      operator: None,
      return_type: to.clone(),
      arguments: vec![
        CppMethodArgument {
          name: "ptr".to_string(),
          argument_type: from.clone(),
          has_default_value: false,
        },
      ],
      allows_variadic_arguments: false,
      template_arguments: Some(vec![to.clone()]),
      declaration_code: None,
    },
    kind: CppFfiMethodKind::Cast(cast),
  }
}

/// Performs a portion of `generate_casts` operation.
/// Adds casts between `target_type` and `base_type` and calls
/// `generate_casts_one` recursively to add casts between `target_type`
/// and base types of `base_type`.
fn generate_casts_one(
  processor_data: &ProcessorData,
  target_type: &CppTypeClassBase,
  base_type: &CppTypeClassBase,
  direct_base_index: Option<usize>,
) -> Result<Vec<CppMethodWithKind>> {
  let target_ptr_type = CppType {
    base: CppTypeBase::Class(target_type.clone()),
    indirection: CppTypeIndirection::Ptr,
    is_const: false,
    is_const2: false,
  };
  let base_ptr_type = CppType {
    base: CppTypeBase::Class(base_type.clone()),
    indirection: CppTypeIndirection::Ptr,
    is_const: false,
    is_const2: false,
  };
  let mut new_methods = Vec::new();
  new_methods.push(create_cast_method(
    CppCast::Static {
      is_unsafe: true,
      direct_base_index: direct_base_index,
    },
    &base_ptr_type,
    &target_ptr_type,
  ));
  new_methods.push(create_cast_method(
    CppCast::Static {
      is_unsafe: false,
      direct_base_index: direct_base_index,
    },
    &target_ptr_type,
    &base_ptr_type,
  ));
  new_methods.push(create_cast_method(
    CppCast::Dynamic,
    &base_ptr_type,
    &target_ptr_type,
  ));

  for database in processor_data.all_databases() {
    for item in &database.items {
      if let CppItemData::ClassBase(ref base) = item.cpp_data {
        if &base.derived_class_type == base_type {
          new_methods.append(&mut generate_casts_one(
            processor_data,
            target_type,
            &base.base_class_type,
            None,
          )?);
        }
      }
    }
  }

  Ok(new_methods)
}

/// Adds `static_cast` and `dynamic_cast` functions for all appropriate pairs of types
/// in this `CppData`.
fn generate_casts(base: &CppBaseSpecifier, data: &ProcessorData) -> Result<Vec<CppMethodWithKind>> {
  //log::status("Adding cast functions");
  Ok(generate_casts_one(
    data,
    &base.derived_class_type,
    &base.base_class_type,
    Some(base.base_index),
  )?)
}

/*
/// Generates the FFI function signature for this method.
fn method_to_ffi_signature<'a>(
  method: CppMethodRefWithKind<'a>,
  cpp_data: &CppDataWithDeps,
  type_allocation_places_override: Option<CppTypeAllocationPlace>,
) -> Result<CppFfiMethod> {
  let get_place = |name| -> Result<ReturnValueAllocationPlace> {
    let v = if let Some(ref x) = type_allocation_places_override {
      x.clone()
    } else {
      cpp_data.type_allocation_place(name)?
    };
    Ok(match v {
      CppTypeAllocationPlace::Heap => ReturnValueAllocationPlace::Heap,
      CppTypeAllocationPlace::Stack => ReturnValueAllocationPlace::Stack,
    })
  };

  let place = if method.method.is_constructor() || method.method.is_destructor() {
    let info = method
      .method
      .class_membership
      .as_ref()
      .expect("class info expected here");
    get_place(&info.class_type.name)?
  } else if method.method.return_type.needs_allocation_place_variants() {
    if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = method.method.return_type.base {
      get_place(name)?
    } else {
      return Err(unexpected("class type expected here").into());
    }
  } else {
    ReturnValueAllocationPlace::NotApplicable
  };

  let c_signature = method.method.c_signature(place.clone())?;
  Ok(CppFfiMethod {
    kind: method.kind,
    allocation_place: place,
    c_signature: c_signature,
  })
}*/

impl<'a> CppFfiGenerator<'a> {
  /// Returns false if the method is excluded from processing
  /// for some reason
  fn should_process_method(&self, method: &CppMethod) -> Result<bool> {
    //    if method.is_fake_inherited_method {
    //      return Ok(false);
    //    }
    let class_name = method.class_name().unwrap_or(&String::new()).clone();
    //    for filter in &self.filters {
    //      let allowed = filter(method).chain_err(|| "cpp_ffi_generator_filter failed")?;
    //      if !allowed {
    //        log::llog(log::DebugFfiSkips, || {
    //          format!("Skipping blacklisted method: \n{}\n", method.short_text())
    //        });
    //        return Ok(false);
    //      }
    //    }
    if class_name == "QFlags" {
      return Ok(false);
    }
    if let Some(ref membership) = method.class_membership {
      if membership.visibility == CppVisibility::Private {
        return Ok(false);
      }
      if membership.visibility == CppVisibility::Protected {
        return Ok(false);
      }
      if membership.is_signal {
        return Ok(false);
      }
    }
    if method.template_arguments.is_some() {
      return Ok(false);
    }
    //if method.template_arguments_values.is_some() && !method.is_ffi_whitelisted {
    // TODO: re-enable after template test compilation (#24) is implemented
    // TODO: QObject::findChild and QObject::findChildren should be allowed
    //return Ok(false);
    //}
    if method
      .all_involved_types()
      .iter()
      .any(|x| x.base.is_or_contains_template_parameter())
    {
      return Ok(false);
    }
    Ok(true)
  }

  /// Generates FFI wrappers for all specified methods,
  /// resolving all name conflicts using additional method captions.
  fn process_methods(&mut self) -> Result<()> {
    let stack_allocated_types: Vec<_> = self
      .data
      .all_items()
      .iter()
      .filter_map(|item| {
        if let CppItemData::Type(ref type_data) = item.cpp_data {
          if let CppTypeData::Class { ref type_base } = *type_data {
            if item.is_stack_allocated_type {
              return Some(type_base.clone());
            }
          }
        }
        None
      })
      .collect();

    for (index, item) in &mut self.data.current_database.items.iter_mut().enumerate() {
      if item.cpp_ffi_methods.len() > 0 {
        continue;
      }
      if let CppItemData::Method(ref method) = item.cpp_data {
        let name = format!("{}_fn{}", self.cpp_ffi_lib_name, index);
        match method.to_ffi_method(&stack_allocated_types, &name) {
          Err(msg) => {
            // TODO: new logs?
          }
          Ok(r) => {
            item.cpp_ffi_methods.push(r);
          }
        }
      }
    }

    Ok(())

    /*
    let mut hash_name_to_methods: HashMap<String, Vec<_>> = HashMap::new();
    {
      let mut process_one = |method: CppMethodRefWithKind| match method_to_ffi_signature(
        method.clone(),
        &self.cpp_data,
        type_allocation_places_override.clone(),
      ) {
        Err(msg) => {
          log::llog(log::DebugFfiSkips, || {
            format!(
              "Unable to produce C function for method:\n{}\nError:{}\n",
              method.method.short_text(),
              msg
            )
          });
        }
        Ok(result) => match c_base_name(
          &result.cpp_method,
          &result.allocation_place,
          include_file_base_name,
        ) {
          Err(msg) => {
            log::llog(log::DebugFfiSkips, || {
              format!(
                "Unable to produce C function for method:\n{}\nError:{}\n",
                method.method.short_text(),
                msg
              )
            });
          }
          Ok(name) => {
            add_to_multihash(
              &mut hash_name_to_methods,
              format!("{}_{}", &self.cpp_ffi_lib_name, name),
              result,
            );
          }
        },
      };

      for method in methods {
        if !self.should_process_method(&method.method)? {
          continue;
        }
        process_one(method.clone());
        // generate methods with omitted arguments
        if let Some(last_arg) = method.method.arguments.last() {
          if last_arg.has_default_value {
            let mut method_copy = method.method.clone();
            while let Some(arg) = method_copy.arguments.pop() {
              if !arg.has_default_value {
                break;
              }
              process_one(CppMethodRefWithKind {
                method: &method_copy,
                kind: CppFfiMethodKind::RealWithOmittedArguments {
                  arguments_before_omitting: Some(method.method.arguments.clone()),
                },
              });
            }
          }
        }
      }
    }

    let mut processed_methods = Vec::new();
    for (key, mut values) in hash_name_to_methods {
      if values.len() == 1 {
        processed_methods.push(CppFfiMethod::new(values.remove(0), key.clone()));
        continue;
      }
      let mut found_strategy = None;
      for strategy in MethodCaptionStrategy::all() {
        let mut type_captions = HashSet::new();
        let mut ok = true;
        for value in &values {
          let caption = value.c_signature.caption(strategy.clone())?;
          if type_captions.contains(&caption) {
            ok = false;
            break;
          }
          type_captions.insert(caption);
        }
        if ok {
          found_strategy = Some(strategy);
          break;
        }
      }
      if let Some(strategy) = found_strategy {
        for x in values {
          let caption = x.c_signature.caption(strategy.clone())?;
          let final_name = if caption.is_empty() {
            key.clone()
          } else {
            format!("{}_{}", key, caption)
          };
          processed_methods.push(CppAndFfiMethod::new(x, final_name));
        }
      } else {
        log::error(format!("values dump: {:?}\n", values));
        log::error("All type caption strategies have failed! Involved functions:");
        for value in values {
          log::error(format!("  {}", value.cpp_method.short_text()));
        }
        return Err(unexpected("all type caption strategies have failed").into());
      }
    }
    processed_methods.sort_by(|a, b| a.c_name.cmp(&b.c_name));
    Ok(processed_methods)*/
  }

  // TODO: slot wrappers

  /*
  /// Generates slot wrappers for all encountered argument types
  /// (excluding types already handled in the dependencies).
  fn generate_slot_wrappers(&'a self) -> Result<Option<CppFfiHeaderData>> {
    let include_file_name = "slots";
    if self
      .cpp_data
      .current
      .processed
      .signal_argument_types
      .is_empty()
    {
      return Ok(None);
    }
    let mut qt_slot_wrappers = Vec::new();
    let mut methods = Vec::new();
    for types in &self.cpp_data.current.processed.signal_argument_types {
      let ffi_types = types.map_if_ok(|t| t.to_cpp_ffi_type(CppTypeRole::NotReturnType))?;
      let args_captions = types.map_if_ok(|t| t.caption(TypeCaptionStrategy::Full))?;
      let args_caption = if args_captions.is_empty() {
        "no_args".to_string()
      } else {
        args_captions.join("_")
      };
      let void_ptr = CppType {
        base: CppTypeBase::Void,
        indirection: CppTypeIndirection::Ptr,
        is_const: false,
        is_const2: false,
      };
      let func_arguments = once(void_ptr.clone())
        .chain(ffi_types.iter().map(|t| t.ffi_type.clone()))
        .collect();
      let class_name = format!("{}_SlotWrapper_{}", self.cpp_ffi_lib_name, args_caption);
      let function_type = CppFunctionPointerType {
        return_type: Box::new(CppType::void()),
        arguments: func_arguments,
        allows_variadic_arguments: false,
      };
      let create_function = |kind: CppMethodKind,
                             name: String,
                             is_slot: bool,
                             arguments: Vec<CppMethodArgument>|
       -> CppMethodWithKind {
        CppMethodWithKind {
          method: CppMethod {
            name: name,
            class_membership: Some(CppMethodClassMembership {
              class_type: CppTypeClassBase {
                name: class_name.clone(),
                template_arguments: None,
              },
              is_virtual: true,
              is_pure_virtual: false,
              is_const: false,
              is_static: false,
              visibility: CppVisibility::Public,
              is_signal: false,
              is_slot: is_slot,
              kind: kind,
            }),
            operator: None,
            return_type: CppType::void(),
            arguments: arguments,
            allows_variadic_arguments: false,
            include_file: include_file_name.to_string(),
            origin_location: None,
            template_arguments: None,
            template_arguments_values: None,
            declaration_code: None,
            doc: None,
            inheritance_chain: Vec::new(),
            is_ffi_whitelisted: false,
            //is_fake_inherited_method: false,
          },
          kind: CppFfiMethodKind::Real,
        }
      };
      methods.push(create_function(
        CppMethodKind::Constructor,
        class_name.clone(),
        false,
        vec![],
      ));
      methods.push(create_function(
        CppMethodKind::Destructor,
        format!("~{}", class_name),
        false,
        vec![],
      ));
      let method_set_args = vec![
        CppMethodArgument {
          name: "func".to_string(),
          argument_type: CppType {
            base: CppTypeBase::FunctionPointer(function_type.clone()),
            indirection: CppTypeIndirection::None,
            is_const: false,
            is_const2: false,
          },
          has_default_value: false,
        },
        CppMethodArgument {
          name: "data".to_string(),
          argument_type: void_ptr.clone(),
          has_default_value: false,
        },
      ];
      methods.push(create_function(
        CppMethodKind::Regular,
        "set".to_string(),
        false,
        method_set_args,
      ));

      let method_custom_slot = create_function(
        CppMethodKind::Regular,
        "custom_slot".to_string(),
        true,
        types
          .iter()
          .enumerate()
          .map(|(num, t)| CppMethodArgument {
            name: format!("arg{}", num),
            argument_type: t.clone(),
            has_default_value: false,
          })
          .collect(),
      );
      let receiver_id = method_custom_slot.method.receiver_id()?;
      methods.push(method_custom_slot);
      qt_slot_wrappers.push(QtSlotWrapper {
        class_name: class_name.clone(),
        arguments: ffi_types,
        function_type: function_type.clone(),
        receiver_id: receiver_id,
      });
      let cast_from = CppType {
        base: CppTypeBase::Class(CppTypeClassBase {
          name: class_name.clone(),
          template_arguments: None,
        }),
        indirection: CppTypeIndirection::Ptr,
        is_const: false,
        is_const2: false,
      };
      let cast_to = CppType {
        base: CppTypeBase::Class(CppTypeClassBase {
          name: "QObject".to_string(),
          template_arguments: None,
        }),
        indirection: CppTypeIndirection::Ptr,
        is_const: false,
        is_const2: false,
      };
      methods.push(create_cast_method(
        CppCast::Static {
          is_unsafe: false,
          is_direct: true,
        },
        &cast_from,
        &cast_to,
        include_file_name,
      ));
    }
    Ok(Some(CppFfiHeaderData {
      include_file_base_name: include_file_name.to_string(),
      methods: self.process_methods(
        include_file_name,
        Some(CppTypeAllocationPlace::Heap),
        methods.iter().map(|i| i.as_ref()),
      )?,
      qt_slot_wrappers: qt_slot_wrappers,
    }))
  }*/
}

// TODO: generate methods with omitted arguments
//for method in methods {
//if let Some(last_arg) = method.method.arguments.last() {
//if last_arg.has_default_value {
//let mut method_copy = method.method.clone();
//while let Some(arg) = method_copy.arguments.pop() {
//if !arg.has_default_value {
//break;
//}
//process_one(CppMethodRefWithKind {
//method: &method_copy,
//kind: CppFfiMethodKind::RealWithOmittedArguments {
//arguments_before_omitting: Some(method.method.arguments.clone()),
//},
//});
//}
//}
//}
//}
