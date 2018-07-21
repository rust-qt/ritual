use common::errors::{unexpected, Result};
use cpp_data::CppBaseSpecifier;
use cpp_data::CppClassField;
use cpp_data::CppTypeDataKind;
use cpp_data::CppVisibility;
use cpp_ffi_data::CppFfiArgumentMeaning;
use cpp_ffi_data::CppFfiMethodArgument;
use cpp_ffi_data::CppFfiType;
use cpp_ffi_data::{CppCast, CppFfiMethod, CppFfiMethodKind, CppFieldAccessorType};
use cpp_method::ReturnValueAllocationPlace;
use cpp_method::{CppMethod, CppMethodArgument, CppMethodClassMembership, CppMethodKind};
use cpp_type::CppTypeRole;
use cpp_type::{CppType, CppTypeBase, CppTypeClassBase, CppTypeIndirection};
use new_impl::database::CppItemData;
use new_impl::processor::ProcessingStep;
use new_impl::processor::ProcessorData;

/// This object generates the C++ wrapper library
struct CppFfiGenerator<'a> {
  /// Input C++ data
  data: ProcessorData<'a>,
  /// Name of the wrapper library
  cpp_ffi_lib_name: String,
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

pub fn cpp_ffi_generator() -> ProcessingStep {
  ProcessingStep::new("cpp_ffi_generator", Vec::new(), run)
}

/// Convenience function to create `CppMethod` object for
/// `static_cast` or `dynamic_cast` from type `from` to type `to`.
/// See `CppMethod`'s documentation for more information
/// about `is_unsafe_static_cast` and `is_direct_static_cast`.
fn create_cast_method(
  cast: CppCast,
  from: &CppType,
  to: &CppType,
  name: &str,
) -> Result<CppFfiMethod> {
  let method = CppMethod {
    name: cast.cpp_method_name().to_string(),
    class_membership: None,
    operator: None,
    return_type: to.clone(),
    arguments: vec![CppMethodArgument {
      name: "ptr".to_string(),
      argument_type: from.clone(),
      has_default_value: false,
    }],
    allows_variadic_arguments: false,
    template_arguments: Some(vec![to.clone()]),
    declaration_code: None,
    doc: None,
  };
  // no need for stack_allocated_types since all cast methods operate on pointers
  let mut r = to_ffi_method(&method, &[], name)?;
  if let CppFfiMethodKind::Method {
    ref mut cast_data, ..
  } = r.kind
  {
    *cast_data = Some(cast);
  } else {
    return Err(
      unexpected("to_ffi_method must return a value with CppFfiMethodKind::Method").into(),
    );
  }
  Ok(r)
}

/// Performs a portion of `generate_casts` operation.
/// Adds casts between `target_type` and `base_type` and calls
/// `generate_casts_one` recursively to add casts between `target_type`
/// and base types of `base_type`.
fn generate_casts_one(
  data: &[CppBaseSpecifier],
  target_type: &CppTypeClassBase,
  base_type: &CppTypeClassBase,
  direct_base_index: Option<usize>,
  name_prefix: &str,
) -> Result<Vec<CppFfiMethod>> {
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
    &format!("{}_cast1", name_prefix),
  )?);
  new_methods.push(create_cast_method(
    CppCast::Static {
      is_unsafe: false,
      direct_base_index: direct_base_index,
    },
    &target_ptr_type,
    &base_ptr_type,
    &format!("{}_cast2", name_prefix),
  )?);
  new_methods.push(create_cast_method(
    CppCast::Dynamic,
    &base_ptr_type,
    &target_ptr_type,
    &format!("{}_cast3", name_prefix),
  )?);

  for base in data {
    if &base.derived_class_type == base_type {
      new_methods.append(&mut generate_casts_one(
        data,
        target_type,
        &base.base_class_type,
        None,
        &format!("{}_base{}", name_prefix, base.base_index),
      )?);
    }
  }

  Ok(new_methods)
}

/// Adds `static_cast` and `dynamic_cast` functions for all appropriate pairs of types
/// in this `CppData`.
fn generate_casts(
  base: &CppBaseSpecifier,
  data: &[CppBaseSpecifier],
  name_prefix: &str,
) -> Result<Vec<CppFfiMethod>> {
  //log::status("Adding cast functions");
  generate_casts_one(
    data,
    &base.derived_class_type,
    &base.base_class_type,
    Some(base.base_index),
    name_prefix,
  )
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

/// Creates FFI method signature for this method:
/// - converts all types to FFI types;
/// - adds "this" argument explicitly if present;
/// - adds "output" argument for return value if `allocation_place` is `Stack`.
pub fn to_ffi_method(
  method: &CppMethod,
  stack_allocated_types: &[CppTypeClassBase],
  name: &str,
) -> Result<CppFfiMethod> {
  if method.allows_variadic_arguments {
    return Err("Variable arguments are not supported".into());
  }
  let mut r = CppFfiMethod {
    arguments: Vec::new(),
    return_type: CppFfiType::void(),
    name: name.to_string(),
    allocation_place: ReturnValueAllocationPlace::NotApplicable,
    checks: Default::default(),
    kind: CppFfiMethodKind::Method {
      cpp_method: method.clone(),
      omitted_arguments: None,
      cast_data: None,
    },
  };
  if let Some(ref info) = method.class_membership {
    if !info.is_static && info.kind != CppMethodKind::Constructor {
      r.arguments.push(CppFfiMethodArgument {
        name: "this_ptr".to_string(),
        argument_type: CppType {
          base: CppTypeBase::Class(info.class_type.clone()),
          is_const: info.is_const,
          is_const2: false,
          indirection: CppTypeIndirection::Ptr,
        }.to_cpp_ffi_type(CppTypeRole::NotReturnType)?,
        meaning: CppFfiArgumentMeaning::This,
      });
    }
  }
  for (index, arg) in method.arguments.iter().enumerate() {
    let c_type = arg
      .argument_type
      .to_cpp_ffi_type(CppTypeRole::NotReturnType)?;
    r.arguments.push(CppFfiMethodArgument {
      name: arg.name.clone(),
      argument_type: c_type,
      meaning: CppFfiArgumentMeaning::Argument(index as i8),
    });
  }
  let real_return_type = if let Some(info) = method.class_info_if_constructor() {
    CppType {
      is_const: false,
      is_const2: false,
      indirection: CppTypeIndirection::None,
      base: CppTypeBase::Class(info.class_type.clone()),
    }
  } else {
    method.return_type.clone()
  };
  let c_type = real_return_type.to_cpp_ffi_type(CppTypeRole::ReturnType)?;
  if real_return_type.needs_allocation_place_variants() {
    if let CppTypeBase::Class(ref base) = real_return_type.base {
      if stack_allocated_types.iter().any(|t| t == base) {
        r.arguments.push(CppFfiMethodArgument {
          name: "output".to_string(),
          argument_type: c_type,
          meaning: CppFfiArgumentMeaning::ReturnValue,
        });
        r.allocation_place = ReturnValueAllocationPlace::Stack;
      } else {
        r.return_type = c_type;
        r.allocation_place = ReturnValueAllocationPlace::Heap;
      }
    } else {
      return Err(
        unexpected("return value needs allocation_place variants but is not a class type").into(),
      );
    }
  } else {
    r.return_type = c_type;
  }
  Ok(r)
}

/// Adds fictional getter and setter methods for each known public field of each class.
fn generate_field_accessors(
  field: &CppClassField,
  stack_allocated_types: &[CppTypeClassBase],
  name_prefix: &str,
) -> Result<Vec<CppFfiMethod>> {
  // TODO: fix doc generator for field accessors
  //log::status("Adding field accessors");
  let mut new_methods = Vec::new();
  let create_method = |name, accessor_type, return_type, arguments| -> Result<CppFfiMethod> {
    let fake_method = CppMethod {
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
      doc: None,
    };
    let mut ffi_method = to_ffi_method(
      &fake_method,
      stack_allocated_types,
      &format!(
        "{}_{}",
        name_prefix,
        format!("{:?}", accessor_type).to_lowercase()
      ),
    )?;
    ffi_method.kind = CppFfiMethodKind::FieldAccessor {
      accessor_type: accessor_type,
      field: field.clone(),
    };
    Ok(ffi_method)
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

fn should_process_item(item: &CppItemData) -> Result<bool> {
  if let CppItemData::Method(ref method) = *item {
    if let Some(class_name) = method.class_name() {
      if class_name == "QFlags" {
        return Ok(false);
      }
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
  }
  if item
    .all_involved_types()
    .iter()
    .any(|x| x.base.is_or_contains_template_parameter())
  {
    return Ok(false);
  }
  Ok(true)

  //if method.template_arguments_values.is_some() && !method.is_ffi_whitelisted {
  // TODO: re-enable after template test compilation (#24) is implemented
  // TODO: QObject::findChild and QObject::findChildren should be allowed
  //return Ok(false);
  //}
}

impl<'a> CppFfiGenerator<'a> {
  /// Returns false if the method is excluded from processing
  /// for some reason

  /// Generates FFI wrappers for all specified methods,
  /// resolving all name conflicts using additional method captions.
  fn process_methods(&mut self) -> Result<()> {
    let stack_allocated_types: Vec<_> = self
      .data
      .all_items()
      .iter()
      .filter_map(|item| {
        if let CppItemData::Type(ref type_data) = item.cpp_data {
          if let CppTypeDataKind::Class { ref type_base } = type_data.kind {
            if type_data.is_stack_allocated_type {
              return Some(type_base.clone());
            }
          }
        }
        None
      })
      .collect();

    let all_class_bases: Vec<_> = self
      .data
      .all_items()
      .iter()
      .filter_map(|item| {
        if let CppItemData::ClassBase(ref base) = item.cpp_data {
          Some(base.clone())
        } else {
          None
        }
      })
      .collect();

    for (index, item) in &mut self.data.current_database.items.iter_mut().enumerate() {
      if item.cpp_ffi_methods.is_some() {
        self.data.html_logger.add(
          &[item.cpp_data.to_string(), "already processed".to_string()],
          "already_processed",
        )?;
        continue;
      }
      if !should_process_item(&item.cpp_data)? {
        self.data.html_logger.add(
          &[item.cpp_data.to_string(), "skipped".to_string()],
          "skipped",
        )?;
        continue;
      }

      let name = format!("{}_item{}", self.cpp_ffi_lib_name, index);
      let result = match item.cpp_data {
        CppItemData::Type(_) | CppItemData::EnumValue(_) => {
          Ok(Vec::new())
          // no FFI methods for these items
        }
        CppItemData::Method(ref method) => {
          to_ffi_method(method, &stack_allocated_types, &name).map(|r| vec![r])
        }
        CppItemData::ClassField(ref field) => {
          generate_field_accessors(field, &stack_allocated_types, &name)
        }
        CppItemData::ClassBase(ref base) => generate_casts(base, &all_class_bases, &name),
        CppItemData::QtSignalArguments(ref signal_srguments) => unimplemented!(),
        CppItemData::TemplateInstantiation(..) => continue,
      };

      match result {
        Err(msg) => {
          item.cpp_ffi_methods = Some(Vec::new());
          self
            .data
            .html_logger
            .add(&[item.cpp_data.to_string(), msg.to_string()], "error")?;
        }
        Ok(r) => {
          self.data.html_logger.add(
            &[
              item.cpp_data.to_string(),
              match r.len() {
                0 => "no methods".to_string(),
                1 => format!("added method: {:?}", r[0]),
                _ => format!("added methods ({}): {:?}", r.len(), r),
              },
            ],
            "success",
          )?;
          item.cpp_ffi_methods = Some(r);
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
