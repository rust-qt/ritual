use caption_strategy::TypeCaptionStrategy;
use cpp_data::{CppTypeKind, EnumValue};
use cpp_ffi_data::{CppAndFfiMethod, CppFfiArgumentMeaning, CppFfiType, IndirectionChange};
use cpp_ffi_generator::CppAndFfiData;
use cpp_method::{CppMethod, ReturnValueAllocationPlace};
use cpp_operator::CppOperator;
use cpp_type::{CppType, CppTypeBase, CppBuiltInNumericType, CppTypeIndirection,
               CppSpecificNumericTypeKind, CppTypeClassBase, CppTypeRole};
use doc_formatter;
use errors::{Result, ChainErr, unexpected};
use log;
use qt_doc_parser::{QtDocData, QtDocResultForMethod};
use rust_info::{RustTypeDeclaration, RustTypeDeclarationKind, RustTypeWrapperKind, RustModule,
                RustMethod, RustMethodScope, RustMethodArgument, RustMethodArgumentsVariant,
                RustMethodArguments, TraitImpl, TraitName, RustEnumValue, RustMethodSelfArgKind};
use rust_type::{RustName, RustType, CompleteType, RustTypeIndirection, RustFFIFunction,
                RustFFIArgument, RustToCTypeConversion};
use string_utils::{CaseOperations, VecCaseOperations, WordIterator};
use utils::{add_to_multihash, MapIfOk};

use std::collections::{HashMap, HashSet, hash_map};

pub use serializable::{RustProcessedTypeKind, RustProcessedTypeInfo};

impl RustProcessedTypeInfo {
  fn is_declared_in(&self, modules: &[RustModule]) -> bool {
    for module in modules {
      if module.types.iter().any(|t| match t.kind {
        RustTypeDeclarationKind::CppTypeWrapper { ref cpp_type_name,
                                                  ref cpp_template_arguments,
                                                  .. } => {
          cpp_type_name == &self.cpp_name && cpp_template_arguments == &self.cpp_template_arguments
        }
        _ => false,
      }) {
        return true;
      }
      if self.is_declared_in(&module.submodules) {
        return true;
      }
    }
    false
  }
}

/// Mode of case conversion
enum Case {
  /// Class case: "OneTwo"
  Class,
  /// Snake case: "one_two"
  Snake,
}


fn operator_rust_name(operator: &CppOperator) -> Result<String> {
  Ok(match *operator {
    CppOperator::Conversion(ref type1) => {
      format!("as_{}",
              try!(type1.caption(TypeCaptionStrategy::Full)).to_snake_case())
    }
    _ => format!("op_{}", try!(operator.c_name())),
  })
}

/// If `remove_qt_prefix` is true, removes "Q" or "Qt"
/// if it is first word of the string and not the only one word.
/// Also converts case of the words.
#[cfg_attr(feature="clippy", allow(collapsible_if))]
fn remove_qt_prefix_and_convert_case(s: &str, case: Case, remove_qt_prefix: bool) -> String {
  let mut parts: Vec<_> = WordIterator::new(s).collect();
  if remove_qt_prefix && parts.len() > 1 {
    if parts[0] == "Q" || parts[0] == "q" || parts[0] == "Qt" {
      parts.remove(0);
    }
  }
  match case {
    Case::Snake => parts.to_snake_case(),
    Case::Class => parts.to_class_case(),
  }
}

/// Removes ".h" from include file name and performs the same
/// processing as `remove_qt_prefix_and_convert_case()` for snake case.
fn include_file_to_module_name(include_file: &str, remove_qt_prefix: bool) -> String {
  let mut r = include_file.to_string();
  if let Some(index) = r.find('.') {
    r = r[0..index].to_string();
  }
  remove_qt_prefix_and_convert_case(&r, Case::Snake, remove_qt_prefix)
}

/// Adds "_" to a string if it is a reserved word in Rust
#[cfg_attr(rustfmt, rustfmt_skip)]
fn sanitize_rust_identifier(name: &str) -> String {
  match name {
    "abstract" | "alignof" | "as" | "become" | "box" | "break" | "const" |
    "continue" | "crate" | "do" | "else" | "enum" | "extern" | "false" |
    "final" | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" |
    "macro" | "match" | "mod" | "move" | "mut" | "offsetof" | "override" |
    "priv" | "proc" | "pub" | "pure" | "ref" | "return" | "Self" | "self" |
    "sizeof" | "static" | "struct" | "super" | "trait" | "true" | "type" |
    "typeof" | "unsafe" | "unsized" | "use" | "virtual" | "where" | "while" |
    "yield" => format!("{}_", name),
    _ => name.to_string()
  }
}

/// Prepares enum variants for being represented in Rust:
/// - Converts variant names to proper case;
/// - Removes duplicate variants that have the same associated value.
/// Rust does not allow such duplicates.
/// - If there is only one variant, adds another variant.
/// Rust does not allow repr(C) enums having only one variant.
fn prepare_enum_values(values: &[EnumValue], name: &str) -> Vec<RustEnumValue> {
  let mut value_to_variant: HashMap<i64, RustEnumValue> = HashMap::new();
  for variant in values {
    let value = variant.value;
    match value_to_variant.entry(value) {
      hash_map::Entry::Occupied(entry) => {
        log::warning(format!("warning: {}: duplicated enum variant removed: {} (previous \
                              variant: {})",
                             name,
                             variant.name,
                             entry.get().name));
      }
      hash_map::Entry::Vacant(entry) => {
        entry.insert(RustEnumValue {
          name: sanitize_rust_identifier(&variant.name.to_class_case()),
          cpp_name: Some(variant.name.clone()),
          value: variant.value,
          doc: format!("C++ variant: {}", &variant.name),
        });

      }
    }
  }
  let more_than_one = value_to_variant.len() > 1;
  if value_to_variant.len() == 1 {
    let dummy_value = if value_to_variant.contains_key(&0) {
      1
    } else {
      0
    };
    value_to_variant.insert(dummy_value,
                            RustEnumValue {
                              name: "_Invalid".to_string(),
                              value: dummy_value as i64,
                              cpp_name: None,
                              doc: "This variant is added in Rust because enums with one \
                                            variant and C representation are not supported."
                                .to_string(),
                            });
  }
  let mut result: Vec<_> = value_to_variant.into_iter()
    .map(|(_val, variant)| variant)
    .collect();
  if more_than_one {
    let new_names = {
      let all_words: Vec<Vec<&str>> = result.iter()
        .map(|x| WordIterator::new(&x.name).collect())
        .collect();
      let tmp_buffer = all_words[0].clone();
      let mut common_prefix = &tmp_buffer[..];
      let mut common_suffix = &tmp_buffer[..];
      for item in &all_words {
        while !common_prefix.is_empty() &&
              (item.len() < common_prefix.len() || &item[..common_prefix.len()] != common_prefix) {
          common_prefix = &common_prefix[..common_prefix.len() - 1];
        }
        while !common_suffix.is_empty() &&
              (item.len() < common_suffix.len() ||
               &item[item.len() - common_suffix.len()..] != common_suffix) {
          common_suffix = &common_suffix[1..];
        }
      }
      let new_names: Vec<_> = all_words.iter()
        .map(|item| item[common_prefix.len()..item.len() - common_suffix.len()].join(""))
        .collect();
      if new_names.iter()
        .any(|item| if let Some(ch) = item.chars().next() {
          ch.is_digit(10)
        } else {
          true
        }) {
        None
      } else {
        Some(new_names)
      }
    };
    if let Some(new_names) = new_names {
      assert_eq!(new_names.len(), result.len());
      for i in 0..new_names.len() {
        result[i].name = sanitize_rust_identifier(&new_names[i].clone());
      }
    }

  }
  result.sort_by(|a, b| a.value.cmp(&b.value));
  result
}


pub struct RustGenerator {
  input_data: CppAndFfiData,
  config: RustGeneratorConfig,
  processed_types: Vec<RustProcessedTypeInfo>,
  dependency_types: Vec<RustProcessedTypeInfo>,
}

/// Results of adapting API for Rust wrapper.
/// This data is passed to Rust code generator.
pub struct RustGeneratorOutput {
  /// List of Rust modules to be generated.
  pub modules: Vec<RustModule>,
  /// List of FFI function imports to be generated.
  pub ffi_functions: Vec<(String, Vec<RustFFIFunction>)>,
  /// List of processed C++ types and their corresponding Rust names
  pub processed_types: Vec<RustProcessedTypeInfo>,
}

/// Config for `rust_generator` module.
pub struct RustGeneratorConfig {
  /// Name of generated crate
  pub crate_name: String,
  /// Flag instructing to remove leading "Q" and "Qt"
  /// from identifiers.
  pub remove_qt_prefix: bool,

  pub qt_doc_data: Option<QtDocData>,
}
// TODO: implement removal of arbitrary prefixes (#25)

/// Execute processing
#[cfg_attr(feature="clippy", allow(extend_from_slice))]
#[cfg_attr(feature="clippy", allow(block_in_if_condition_stmt))]
pub fn run(input_data: CppAndFfiData,
           dependency_rust_types: Vec<RustProcessedTypeInfo>,
           config: RustGeneratorConfig)
           -> Result<RustGeneratorOutput> {
  let generator = RustGenerator {
    processed_types: try!(process_types(&input_data, &config, &dependency_rust_types)),
    dependency_types: dependency_rust_types,
    input_data: input_data,
    config: config,
  };
  let mut modules = Vec::new();
  {
    let mut cpp_methods: Vec<&CppAndFfiMethod> = Vec::new();
    for header in &generator.input_data.cpp_ffi_headers {
      cpp_methods.extend(header.methods.iter());
    }
    let mut module_names_set = HashSet::new();
    for item in &generator.processed_types {
      if !module_names_set.contains(&item.rust_name.parts[1]) {
        module_names_set.insert(item.rust_name.parts[1].clone());
      }
    }
    cpp_methods = cpp_methods.into_iter()
      .filter(|method| {
        if let Some(ref info) = method.cpp_method.class_membership {
          if !generator.processed_types.iter().any(|t| {
            t.cpp_name == info.class_type.name &&
            t.cpp_template_arguments == info.class_type.template_arguments
          }) {
            log::warning("Warning: method is skipped because class type is not \
                                  available in Rust:");
            log::warning(format!("{}\n", method.short_text()));
            return false;
          }
        }
        true
      })
      .collect();
    for method in cpp_methods.clone() {
      if method.cpp_method.class_membership.is_none() {
        let rust_name = try!(calculate_rust_name(&method.cpp_method.name,
                                                 &method.cpp_method.include_file,
                                                 true,
                                                 method.cpp_method.operator.as_ref(),
                                                 &generator.config));
        if !module_names_set.contains(&rust_name.parts[1]) {
          module_names_set.insert(rust_name.parts[1].clone());
        }
      }
    }

    let mut module_names: Vec<_> = module_names_set.into_iter().collect();
    module_names.sort();
    let module_count = module_names.len();
    for (i, module_name) in module_names.into_iter().enumerate() {
      log::info(format!("({}/{}) Generating module: {}",
                        i + 1,
                        module_count,
                        module_name));
      let full_module_name = try!(RustName::new(vec![generator.config.crate_name.clone(),
                                                     module_name]));
      let (module, tmp_cpp_methods) =
        try!(generator.generate_module(cpp_methods, &full_module_name));
      cpp_methods = tmp_cpp_methods;
      if let Some(module) = module {
        modules.push(module);
      }
    }
    if !cpp_methods.is_empty() {
      log::warning("unprocessed cpp methods left:");
      for method in cpp_methods {
        log::warning(format!("  {}", method.cpp_method.short_text()));
        if let Some(ref info) = method.cpp_method.class_membership {
          let rust_name = try!(calculate_rust_name(&info.class_type.name,
                                                   &method.cpp_method.include_file,
                                                   false,
                                                   None,
                                                   &generator.config));
          log::warning(format!("  -> {}", rust_name.full_name(None)));
        } else {
          let rust_name = try!(calculate_rust_name(&method.cpp_method.name,
                                                   &method.cpp_method.include_file,
                                                   true,
                                                   method.cpp_method.operator.as_ref(),
                                                   &generator.config));
          log::warning(format!("  -> {}", rust_name.full_name(None)));
        }
      }
      return Err(unexpected("unprocessed cpp methods left").into());
    }
  }
  let mut any_not_declared = false;
  for type1 in &generator.processed_types {
    if !type1.is_declared_in(&modules) {
      log::warning(format!("type is not processed: {:?}", type1));
      any_not_declared = true;
    }
  }
  if any_not_declared {
    return Err(unexpected("unprocessed cpp types left").into());
  }
  Ok(RustGeneratorOutput {
    ffi_functions: generator.ffi(),
    modules: modules,
    processed_types: generator.processed_types,
  })
}

/// Generates `RustName` for specified function or type name,
/// including crate name and modules list.
fn calculate_rust_name(name: &str,
                       include_file: &str,
                       is_function: bool,
                       operator: Option<&CppOperator>,
                       config: &RustGeneratorConfig)
                       -> Result<RustName> {
  let mut split_parts: Vec<_> = name.split("::").collect();
  let original_last_part = try!(split_parts.pop().chain_err(|| "split_parts can't be empty"))
    .to_string();
  let last_part = if let Some(operator) = operator {
    try!(operator_rust_name(operator))
  } else {
    remove_qt_prefix_and_convert_case(&original_last_part,
                                      if is_function {
                                        Case::Snake
                                      } else {
                                        Case::Class
                                      },
                                      config.remove_qt_prefix)
  };

  let mut parts = Vec::new();
  parts.push(config.crate_name.clone());
  parts.push(include_file_to_module_name(include_file, config.remove_qt_prefix));
  for part in split_parts {
    parts.push(remove_qt_prefix_and_convert_case(&part.to_string(),
                                                 Case::Snake,
                                                 config.remove_qt_prefix));
  }

  if parts.len() > 2 && parts[1] == parts[2] {
    // special case
    parts.remove(2);
  }
  parts.push(last_part);
  RustName::new(parts)
}

/// Generates Rust names and type information for all available C++ types.
fn process_types(input_data: &CppAndFfiData,
                 config: &RustGeneratorConfig,
                 dependency_types: &[RustProcessedTypeInfo])
                 -> Result<Vec<RustProcessedTypeInfo>> {
  let mut result = Vec::new();
  for type_info in &input_data.cpp_data.types {
    if let CppTypeKind::Class { ref template_arguments, .. } = type_info.kind {
      if template_arguments.is_some() {
        continue;
      }
    }
    result.push(RustProcessedTypeInfo {
      cpp_name: type_info.name.clone(),
      cpp_template_arguments: None,
      kind: match type_info.kind {
        CppTypeKind::Class { ref size, .. } => {
          RustProcessedTypeKind::Class {
            size: try!(size.chain_err(|| "size must be present")),
            is_deletable: input_data.cpp_data.has_public_destructor(&CppTypeClassBase {
              name: type_info.name.clone(),
              template_arguments: None,
            }),
          }
        }
        CppTypeKind::Enum { ref values } => RustProcessedTypeKind::Enum { values: values.clone() },
      },
      rust_name: try!(calculate_rust_name(&type_info.name,
                                          &type_info.include_file,
                                          false,
                                          None,
                                          config)),
    });
  }
  let template_final_name =
    |result: &Vec<RustProcessedTypeInfo>, item: &RustProcessedTypeInfo| -> Result<RustName> {
      let mut name = item.rust_name.clone();
      let last_name = try!(name.parts.pop().chain_err(|| "name.parts can't be empty"));
      let mut arg_captions = Vec::new();
      if let Some(ref args) = item.cpp_template_arguments {
        for x in args {
          let rust_type = try!(complete_type(result,
                                             dependency_types,
                                             &try!(x.to_cpp_ffi_type(CppTypeRole::NotReturnType)),
                                             &CppFfiArgumentMeaning::Argument(0),
                                             &ReturnValueAllocationPlace::NotApplicable));
          arg_captions.push(try!(rust_type.rust_api_type.caption()).to_class_case());
        }
      } else {
        return Err("template arguments expected".into());
      }
      name.parts.push(last_name + &arg_captions.join(""));
      Ok(name)
    };
  let mut name_failed_items = Vec::new();
  for template_instantiations in &input_data.cpp_data.template_instantiations {
    if template_instantiations.class_name == "QFlags" {
      // special processing is implemented for QFlags
      continue;
    }
    for ins in &template_instantiations.instantiations {
      name_failed_items.push(RustProcessedTypeInfo {
        cpp_name: template_instantiations.class_name.clone(),
        cpp_template_arguments: Some(ins.template_arguments.clone()),
        kind: RustProcessedTypeKind::Class {
          size: ins.size,
          is_deletable: input_data.cpp_data.has_public_destructor(&CppTypeClassBase {
            name: template_instantiations.class_name.clone(),
            template_arguments: Some(ins.template_arguments.clone()),
          }),
        },
        rust_name: try!(calculate_rust_name(&template_instantiations.class_name,
                                            &template_instantiations.include_file,
                                            false,
                                            None,
                                            config)),
      });
    }
  }
  let mut any_success = true;
  while !name_failed_items.is_empty() {
    if !any_success {
      log::warning("Failed to generate Rust names for template types:");
      for r in name_failed_items {
        log::warning(format!("  {:?}\n  {}\n\n",
                             r,
                             if let Err(err) = template_final_name(&result, &r) {
                               err
                             } else {
                               return Err("template_final_name must return Err at this stage"
                                 .into());
                             }));
      }
      break;
    }
    any_success = false;
    let mut name_failed_items_new = Vec::new();
    for mut r in name_failed_items {
      match template_final_name(&result, &r) {
        Ok(name) => {
          r.rust_name = name;
          result.push(r);
          any_success = true;
        }
        Err(_) => name_failed_items_new.push(r),
      }

    }
    name_failed_items = name_failed_items_new;
  }
  Ok(result)
}

struct ProcessTypeResult {
  main_type: RustTypeDeclaration,
  overloading_types: Vec<RustTypeDeclaration>,
}
#[derive(Default)]
struct ProcessFunctionsResult {
  methods: Vec<RustMethod>,
  trait_impls: Vec<TraitImpl>,
  overloading_types: Vec<RustTypeDeclaration>,
}

/// Generates `CompleteType` from `CppFfiType`, adding
/// Rust API type, Rust FFI type and conversion between them.
fn complete_type(processed_types: &[RustProcessedTypeInfo],
                 dependency_types: &[RustProcessedTypeInfo],
                 cpp_ffi_type: &CppFfiType,
                 argument_meaning: &CppFfiArgumentMeaning,
                 allocation_place: &ReturnValueAllocationPlace)
                 -> Result<CompleteType> {
  let rust_ffi_type = try!(ffi_type(processed_types, dependency_types, &cpp_ffi_type.ffi_type));
  let mut rust_api_type = rust_ffi_type.clone();
  let mut rust_api_to_c_conversion = RustToCTypeConversion::None;
  if let RustType::Common { ref mut indirection,
                            ref mut base,
                            ref mut generic_arguments,
                            ref mut is_const,
                            ref mut is_const2 } = rust_api_type {
    match cpp_ffi_type.conversion {
      IndirectionChange::NoChange => {
        if argument_meaning == &CppFfiArgumentMeaning::This {
          assert!(indirection == &RustTypeIndirection::Ptr);
          *indirection = RustTypeIndirection::Ref { lifetime: None };
          rust_api_to_c_conversion = RustToCTypeConversion::RefToPtr;
        }
      }
      IndirectionChange::ValueToPointer => {
        assert!(indirection == &RustTypeIndirection::Ptr);
        if argument_meaning == &CppFfiArgumentMeaning::ReturnValue {
          if let Some(info) = find_type_info(processed_types,
                                             dependency_types,
                                             |x| &x.rust_name == base) {
            if let RustProcessedTypeKind::Class { ref is_deletable, .. } = info.kind {
              if !*is_deletable {
                return Err(format!("{} is not deletable", base.full_name(None)).into());
              }
            } else {
              return Err(unexpected("class type expected here").into());
            }
          } else {
            return Err(unexpected("find_type_info failed in complete_type() after success in \
                                   ffi_type()")
              .into());
          }
          match *allocation_place {
            ReturnValueAllocationPlace::Stack => {
              *indirection = RustTypeIndirection::None;
              rust_api_to_c_conversion = RustToCTypeConversion::ValueToPtr;
            }
            ReturnValueAllocationPlace::Heap => {
              *indirection = RustTypeIndirection::None;
              rust_api_to_c_conversion = RustToCTypeConversion::CppBoxToPtr;
              assert!(generic_arguments.is_none());
              assert!(!*is_const);
              assert!(!*is_const2);
              let new_generic_argument = RustType::Common {
                base: base.clone(),
                generic_arguments: None,
                is_const: false,
                is_const2: false,
                indirection: RustTypeIndirection::None,
              };
              *base = try!(RustName::new(vec!["cpp_utils".to_string(), "CppBox".to_string()]));
              *generic_arguments = Some(vec![new_generic_argument]);

            }
            ReturnValueAllocationPlace::NotApplicable => {
              return Err(unexpected("NotApplicable conflicts with ValueToPointer").into());
            }
          }
        } else {
          *indirection = RustTypeIndirection::Ref { lifetime: None };
          *is_const = true;
          *is_const2 = true;
          rust_api_to_c_conversion = RustToCTypeConversion::RefToPtr;
        }
      }
      IndirectionChange::ReferenceToPointer => {
        match *indirection {
          RustTypeIndirection::Ptr => {
            *indirection = RustTypeIndirection::Ref { lifetime: None };
          }
          RustTypeIndirection::PtrPtr => {
            *indirection = RustTypeIndirection::PtrRef { lifetime: None };
          }
          _ => return Err(unexpected("invalid indirection for ReferenceToPointer").into()),
        }
        rust_api_to_c_conversion = RustToCTypeConversion::RefToPtr;
      }
      IndirectionChange::QFlagsToUInt => {}
    }
  }
  if cpp_ffi_type.conversion == IndirectionChange::QFlagsToUInt {
    rust_api_to_c_conversion = RustToCTypeConversion::QFlagsToUInt;
    let enum_type = if let CppTypeBase::Class(CppTypeClassBase { ref template_arguments, .. }) =
                           cpp_ffi_type.original_type.base {
      let args = try!(template_arguments.as_ref()
        .chain_err(|| "QFlags type must have template arguments"));
      if args.len() != 1 {
        return Err("QFlags type must have exactly 1 template argument".into());
      }
      if let CppTypeBase::Enum { ref name } = args[0].base {
        match find_type_info(processed_types, dependency_types, |x| &x.cpp_name == name) {
          None => return Err(format!("type has no Rust equivalent: {}", name).into()),
          Some(info) => info.rust_name.clone(),
        }
      } else {
        return Err(unexpected("invalid original type for QFlags").into());
      }
    } else {
      return Err(unexpected("invalid original type for QFlags").into());
    };
    rust_api_type = RustType::Common {
      base: try!(RustName::new(vec!["qt_core".to_string(),
                                    "flags".to_string(),
                                    "Flags".to_string()])),
      generic_arguments: Some(vec![RustType::Common {
                                     base: enum_type,
                                     generic_arguments: None,
                                     indirection: RustTypeIndirection::None,
                                     is_const: false,
                                     is_const2: false,
                                   }]),
      indirection: RustTypeIndirection::None,
      is_const: false,
      is_const2: false,
    }
  }

  Ok(CompleteType {
    cpp_ffi_type: cpp_ffi_type.ffi_type.clone(),
    cpp_type: cpp_ffi_type.original_type.clone(),
    cpp_to_ffi_conversion: cpp_ffi_type.conversion.clone(),
    rust_ffi_type: rust_ffi_type,
    rust_api_type: rust_api_type,
    rust_api_to_c_conversion: rust_api_to_c_conversion,
  })
}

fn find_type_info<'a, F>(processed_types: &'a [RustProcessedTypeInfo],
                         dependency_types: &'a [RustProcessedTypeInfo],
                         f: F)
                         -> Option<&'a RustProcessedTypeInfo>
  where F: Fn(&RustProcessedTypeInfo) -> bool
{
  match processed_types.iter().find(|x| f(x)) {
    None => dependency_types.iter().find(|x| f(x)),
    Some(info) => Some(info),
  }
}

/// Converts `CppType` to its exact Rust equivalent (FFI-compatible)
fn ffi_type(processed_types: &[RustProcessedTypeInfo],
            dependency_types: &[RustProcessedTypeInfo],
            cpp_ffi_type: &CppType)
            -> Result<RustType> {
  let rust_name = match cpp_ffi_type.base {
    CppTypeBase::Void => {
      match cpp_ffi_type.indirection {
        CppTypeIndirection::None => return Ok(RustType::Void),
        _ => try!(RustName::new(vec!["libc".to_string(), "c_void".to_string()])),
      }
    }
    CppTypeBase::BuiltInNumeric(ref numeric) => {
      if numeric == &CppBuiltInNumericType::Bool {
        try!(RustName::new(vec!["bool".to_string()]))
      } else {
        let own_name = match *numeric {
          CppBuiltInNumericType::Bool => unreachable!(),
          CppBuiltInNumericType::Char => "c_char",
          CppBuiltInNumericType::SChar => "c_schar",
          CppBuiltInNumericType::UChar => "c_uchar",
          CppBuiltInNumericType::WChar => "wchar_t",
          CppBuiltInNumericType::Short => "c_short",
          CppBuiltInNumericType::UShort => "c_ushort",
          CppBuiltInNumericType::Int => "c_int",
          CppBuiltInNumericType::UInt => "c_uint",
          CppBuiltInNumericType::Long => "c_long",
          CppBuiltInNumericType::ULong => "c_ulong",
          CppBuiltInNumericType::LongLong => "c_longlong",
          CppBuiltInNumericType::ULongLong => "c_ulonglong",
          CppBuiltInNumericType::Float => "c_float",
          CppBuiltInNumericType::Double => "c_double",
          _ => return Err(format!("unsupported numeric type: {:?}", numeric).into()),
        };
        try!(RustName::new(vec!["libc".to_string(), own_name.to_string()]))
      }
    }
    CppTypeBase::SpecificNumeric { ref bits, ref kind, .. } => {
      let letter = match *kind {
        CppSpecificNumericTypeKind::Integer { ref is_signed } => if *is_signed { "i" } else { "u" },
        CppSpecificNumericTypeKind::FloatingPoint => "f",
      };
      try!(RustName::new(vec![format!("{}{}", letter, bits)]))
    }
    CppTypeBase::PointerSizedInteger { ref is_signed, .. } => {
      try!(RustName::new(vec![if *is_signed { "isize" } else { "usize" }.to_string()]))
    }
    CppTypeBase::Enum { ref name } => {
      match find_type_info(processed_types, dependency_types, |x| &x.cpp_name == name) {
        None => return Err(format!("type has no Rust equivalent: {}", name).into()),
        Some(info) => info.rust_name.clone(),
      }
    }
    CppTypeBase::Class(ref name_and_args) => {
      match find_type_info(processed_types, dependency_types, |x| {
        &x.cpp_name == &name_and_args.name &&
        &x.cpp_template_arguments == &name_and_args.template_arguments
      }) {
        None => return Err(format!("type has no Rust equivalent: {:?}", name_and_args).into()),
        Some(info) => info.rust_name.clone(),
      }
    }
    CppTypeBase::FunctionPointer { ref return_type,
                                   ref arguments,
                                   ref allows_variadic_arguments } => {
      if *allows_variadic_arguments {
        return Err("function pointers with variadic arguments are not supported".into());
      }
      let mut rust_args = Vec::new();
      for arg in arguments {
        rust_args.push(try!(ffi_type(processed_types, dependency_types, arg)));
      }
      let rust_return_type = try!(ffi_type(processed_types, dependency_types, return_type));
      return Ok(RustType::FunctionPointer {
        arguments: rust_args,
        return_type: Box::new(rust_return_type),
      });
    }
    CppTypeBase::TemplateParameter { .. } => return Err(unexpected("invalid cpp type").into()),
  };
  Ok(RustType::Common {
    base: rust_name,
    is_const: cpp_ffi_type.is_const,
    is_const2: cpp_ffi_type.is_const2,
    indirection: match cpp_ffi_type.indirection {
      CppTypeIndirection::None => RustTypeIndirection::None,
      CppTypeIndirection::Ptr => RustTypeIndirection::Ptr,
      CppTypeIndirection::PtrPtr => RustTypeIndirection::PtrPtr,
      _ => {
        return Err(format!("invalid FFI type indirection: {:?}",
                           cpp_ffi_type.indirection)
          .into())
      }
    },
    generic_arguments: None,
  })
}


impl RustGenerator {
  /// Generates exact Rust equivalent of CppAndFfiMethod object
  /// (FFI-compatible)
  fn ffi_function(&self, data: &CppAndFfiMethod) -> Result<RustFFIFunction> {
    let mut args = Vec::new();
    for arg in &data.c_signature.arguments {
      let rust_type = try!(ffi_type(&self.processed_types,
                                    &self.dependency_types,
                                    &arg.argument_type.ffi_type));
      args.push(RustFFIArgument {
        name: sanitize_rust_identifier(&arg.name),
        argument_type: rust_type,
      });
    }
    Ok(RustFFIFunction {
      return_type: try!(ffi_type(&self.processed_types,
                                 &self.dependency_types,
                                 &data.c_signature.return_type.ffi_type)),
      name: data.c_name.clone(),
      arguments: args,
    })
  }


  /// Converts specified C++ type to Rust.
  /// Returns:
  /// - main_type - representation of the target type, including
  /// directly implemented methods of the type and trait implementations;
  /// - overloading_types - traits and their implementations that
  /// emulate C++ method overloading.
  fn process_type<'a>(&'a self,
                      info: &'a RustProcessedTypeInfo,
                      mut cpp_methods: Vec<&'a CppAndFfiMethod>)
                      -> Result<(ProcessTypeResult, Vec<&'a CppAndFfiMethod>)> {
    Ok(match info.kind {
      RustProcessedTypeKind::Enum { ref values } => {
        let mut is_flaggable = false;
        let template_arg_sample = CppType {
          is_const: false,
          is_const2: false,
          indirection: CppTypeIndirection::None,
          base: CppTypeBase::Enum { name: info.cpp_name.clone() },
        };

        for flag_owner_name in &["QFlags", "QUrlTwoFlags"] {
          if let Some(instantiations) = self.input_data
            .cpp_data
            .template_instantiations
            .iter()
            .find(|x| &x.class_name == &flag_owner_name.to_string()) {
            if instantiations.instantiations
              .iter()
              .any(|ins| ins.template_arguments.iter().any(|arg| arg == &template_arg_sample)) {
              is_flaggable = true;
              break;
            }
          }

        }

        // TODO: export Qt doc for enum and its variants (#35)
        let doc = format!("C++ type: {}",
                          doc_formatter::wrap_inline_cpp_code(&info.cpp_name));
        (ProcessTypeResult {
          main_type: RustTypeDeclaration {
            name: try!(info.rust_name.last_name()).clone(),
            kind: RustTypeDeclarationKind::CppTypeWrapper {
              kind: RustTypeWrapperKind::Enum {
                values: prepare_enum_values(values, &info.cpp_name),
                is_flaggable: is_flaggable,
              },
              cpp_type_name: info.cpp_name.clone(),
              cpp_template_arguments: None,
              methods: Vec::new(),
              traits: Vec::new(),
            },
            doc: doc,
          },
          overloading_types: Vec::new(),
        },
         cpp_methods)
      }
      RustProcessedTypeKind::Class { ref size, .. } => {
        let methods_scope = RustMethodScope::Impl { type_name: info.rust_name.clone() };
        let class_type = CppTypeClassBase {
          name: info.cpp_name.clone(),
          template_arguments: info.cpp_template_arguments.clone(),
        };
        let mut good_methods = Vec::new();
        let mut tmp_cpp_methods = Vec::new();
        for method in cpp_methods {
          if let Some(ref info) = method.cpp_method.class_membership {
            if &info.class_type == &class_type {
              good_methods.push(method);
              continue;
            }
          }
          tmp_cpp_methods.push(method);
        }
        cpp_methods = tmp_cpp_methods;
        let functions_result =
          try!(self.process_functions(good_methods.into_iter(), &methods_scope));
        // TODO: export Qt doc for class (detailed description) (#35)
        let doc = format!("C++ type: {}.",
                          doc_formatter::wrap_inline_cpp_code(&CppType {
                              base: CppTypeBase::Class(class_type.clone()),
                              indirection: CppTypeIndirection::None,
                              is_const: false,
                              is_const2: false,
                            }
                            .to_cpp_pseudo_code()));

        (ProcessTypeResult {
          main_type: RustTypeDeclaration {
            name: try!(info.rust_name.last_name()).clone(),
            kind: RustTypeDeclarationKind::CppTypeWrapper {
              kind: RustTypeWrapperKind::Struct { size: *size },
              cpp_type_name: info.cpp_name.clone(),
              cpp_template_arguments: info.cpp_template_arguments.clone(),
              methods: functions_result.methods,
              traits: functions_result.trait_impls,
            },
            doc: doc,
          },
          overloading_types: functions_result.overloading_types,
        },
         cpp_methods)
      }
    })
  }

  /// Generates a Rust module with specified name from specified
  /// C++ header. If the module should have nested modules,
  /// this function calls itself recursively with nested module name
  /// but the same header data.
  pub fn generate_module<'a, 'b>(&'a self,
                                 mut cpp_methods: Vec<&'a CppAndFfiMethod>,
                                 module_name: &'b RustName)
                                 -> Result<(Option<RustModule>, Vec<&'a CppAndFfiMethod>)> {
    log::info(format!("Generating Rust module {}", module_name.full_name(None)));

    let mut direct_submodules = HashSet::new();
    let mut module = RustModule {
      name: try!(module_name.last_name()).clone(),
      types: Vec::new(),
      functions: Vec::new(),
      submodules: Vec::new(),
    };
    let mut rust_overloading_types = Vec::new();
    let mut good_methods = Vec::new();
    {
      // Checks if the name should be processed.
      // Returns true if the name is directly in this module.
      // If the name is in this module's submodule, adds
      // name of the direct submodule to direct_submodules list.
      let mut check_name = |rust_name: &RustName| {
        if module_name.includes(rust_name) {
          if module_name.includes_directly(rust_name) {
            return true;
          } else {
            let direct_submodule = &rust_name.parts[module_name.parts.len()];
            if !direct_submodules.contains(direct_submodule) {
              direct_submodules.insert(direct_submodule.clone());
            }
          }
        }
        false
      }; // end of check_name()

      for type_data in &self.processed_types {
        if check_name(&type_data.rust_name) {
          let (mut result, tmp_cpp_methods) = try!(self.process_type(type_data, cpp_methods));
          cpp_methods = tmp_cpp_methods;
          module.types.push(result.main_type);
          rust_overloading_types.append(&mut result.overloading_types);
        }
      }

      let mut tmp_cpp_methods = Vec::new();
      for method in cpp_methods {
        if method.cpp_method.class_membership.is_none() {
          let rust_name = try!(calculate_rust_name(&method.cpp_method.name,
                                                   &method.cpp_method.include_file,
                                                   true,
                                                   method.cpp_method.operator.as_ref(),
                                                   &self.config));

          if check_name(&rust_name) {
            good_methods.push(method);
            continue;
          }
        }
        tmp_cpp_methods.push(method);
      }
      cpp_methods = tmp_cpp_methods;
    }
    for name in direct_submodules {
      let mut new_name = module_name.clone();
      new_name.parts.push(name);
      let (submodule, tmp_cpp_methods) = try!(self.generate_module(cpp_methods, &new_name));
      cpp_methods = tmp_cpp_methods;
      if let Some(submodule) = submodule {
        module.submodules.push(submodule);
      }
    }
    let mut free_functions_result =
      try!(self.process_functions(good_methods.into_iter(), &RustMethodScope::Free));
    assert!(free_functions_result.trait_impls.is_empty());
    module.functions = free_functions_result.methods;
    rust_overloading_types.append(&mut free_functions_result.overloading_types);
    if !rust_overloading_types.is_empty() {
      rust_overloading_types.sort_by(|a, b| a.name.cmp(&b.name));
      module.submodules.push(RustModule {
        name: "overloading".to_string(),
        types: rust_overloading_types,
        functions: Vec::new(),
        submodules: Vec::new(),
      });
    }
    module.types.sort_by(|a, b| a.name.cmp(&b.name));
    module.submodules.sort_by(|a, b| a.name.cmp(&b.name));
    if module.types.is_empty() && module.functions.is_empty() && module.submodules.is_empty() {
      log::warning(format!("Skipping empty module: {}", module.name));
      return Ok((None, cpp_methods));
    }
    Ok((Some(module), cpp_methods))
  }

  /// Converts one function to a RustMethod
  fn generate_function(&self,
                       method: &CppAndFfiMethod,
                       scope: &RustMethodScope,
                       generate_doc: bool)
                       -> Result<RustMethod> {
    let mut arguments = Vec::new();
    for (arg_index, arg) in method.c_signature.arguments.iter().enumerate() {
      if arg.meaning != CppFfiArgumentMeaning::ReturnValue {
        let arg_type = try!(complete_type(&self.processed_types,
                                          &self.dependency_types,
                                          &arg.argument_type,
                                          &arg.meaning,
                                          &method.allocation_place));
        arguments.push(RustMethodArgument {
          ffi_index: Some(arg_index as i32),
          argument_type: arg_type,
          name: if arg.meaning == CppFfiArgumentMeaning::This {
            "self".to_string()
          } else {
            sanitize_rust_identifier(&arg.name.to_snake_case())
          },
        });
      }
    }
    let (mut return_type, return_arg_index) = if let Some((arg_index, arg)) = method.c_signature
      .arguments
      .iter()
      .enumerate()
      .find(|&(_arg_index, arg)| arg.meaning == CppFfiArgumentMeaning::ReturnValue) {
      // an argument has return value meaning, so
      // FFI return type must be void
      assert!(method.c_signature.return_type == CppFfiType::void());
      (try!(complete_type(&self.processed_types,
                          &self.dependency_types,
                          &arg.argument_type,
                          &arg.meaning,
                          &method.allocation_place)),
       Some(arg_index as i32))
    } else {
      // none of the arguments has return value meaning,
      // so FFI return value must be used
      let return_type = try!(complete_type(&self.processed_types,
                                           &self.dependency_types,
                                           &method.c_signature.return_type,
                                           &CppFfiArgumentMeaning::ReturnValue,
                                           &method.allocation_place));
      (return_type, None)
    };
    if return_type.rust_api_type.is_ref() {
      let mut next_lifetime_num = 0;
      for arg in &mut arguments {
        if arg.argument_type.rust_api_type.is_ref() {
          arg.argument_type.rust_api_type =
            arg.argument_type.rust_api_type.with_lifetime(format!("l{}", next_lifetime_num));
          next_lifetime_num += 1;
        }
      }
      let return_lifetime = if next_lifetime_num == 0 {
        log::warning(format!("Method returns a reference but doesn't receive a reference: {}",
                             method.short_text()));
        log::warning("Assuming static lifetime of return value.");
        "static".to_string()
      } else {
        "l0".to_string()
      };
      return_type.rust_api_type = return_type.rust_api_type.with_lifetime(return_lifetime);
    }

    let doc = if generate_doc {
      let doc_item = doc_formatter::DocItem {
        cpp_fn: method.short_text(),
        rust_fns: Vec::new(),
        doc: self.get_qt_doc_for_method(&method.cpp_method),
        inherited_from: method.cpp_method.inherited_from.clone(),
      };
      doc_formatter::method_doc(vec![doc_item], &method.cpp_method.full_name())
    } else {
      String::new()
    };
    Ok(RustMethod {
      name: try!(self.method_rust_name(method)),
      scope: scope.clone(),
      arguments: RustMethodArguments::SingleVariant(RustMethodArgumentsVariant {
        arguments: arguments,
        cpp_method: method.clone(),
        return_type: return_type,
        return_type_ffi_index: return_arg_index,
      }),
      doc: doc,
    })
  }

  /// Returns method name. For class member functions, the name doesn't
  /// include class name and scope. For free functions, the name includes
  /// modules.
  fn method_rust_name(&self, method: &CppAndFfiMethod) -> Result<RustName> {
    let mut name = if method.cpp_method.class_membership.is_none() {
      try!(calculate_rust_name(&method.cpp_method.name,
                               &method.cpp_method.include_file,
                               true,
                               method.cpp_method.operator.as_ref(),
                               &self.config))
    } else {
      let x = if method.cpp_method.is_constructor() {
        "new".to_string()
      } else if let Some(ref operator) = method.cpp_method.operator {
        try!(operator_rust_name(operator))
      } else {
        method.cpp_method.name.to_snake_case()
      };
      try!(RustName::new(vec![x]))
    };
    let sanitized = sanitize_rust_identifier(try!(name.last_name()));
    if &sanitized != try!(name.last_name()) {
      try!(name.parts.pop().chain_err(|| "name can't be empty"));
      name.parts.push(sanitized);
    }
    Ok(name)
  }

  fn process_destructor(&self,
                        method: &CppAndFfiMethod,
                        scope: &RustMethodScope)
                        -> Result<TraitImpl> {
    if let RustMethodScope::Impl { ref type_name } = *scope {
      match method.allocation_place {
        ReturnValueAllocationPlace::Stack => {
          let mut method = try!(self.generate_function(method, scope, true));
          method.name = try!(RustName::new(vec!["drop".to_string()]));
          method.scope = RustMethodScope::TraitImpl {
            type_name: type_name.clone(),
            trait_name: TraitName::Drop,
          };
          Ok(TraitImpl {
            target_type: type_name.clone(),
            trait_name: TraitName::Drop,
            methods: vec![method],
          })
        }
        ReturnValueAllocationPlace::Heap => {
          Ok(TraitImpl {
            target_type: type_name.clone(),
            trait_name: TraitName::CppDeletable { deleter_name: method.c_name.clone() },
            methods: Vec::new(),
          })
        }
        ReturnValueAllocationPlace::NotApplicable => {
          return Err(unexpected("destructor must have allocation place").into())
        }
      }
    } else {
      return Err(unexpected("destructor must be in class scope").into());
    }
  }

  // Generates a single overloaded method from all specified methods or
  // accepts a single method without change. Adds self argument caption if needed.
  // All passed methods must be valid for overloading:
  // - they must have the same name and be in the same scope;
  // - they must have the same self argument type;
  // - they must not have exactly the same argument types.
  fn process_method(&self,
                    mut filtered_methods: Vec<RustMethod>,
                    scope: &RustMethodScope,
                    self_arg_kind_caption: Option<&'static str>)
                    -> Result<(RustMethod, Option<RustTypeDeclaration>)> {
    filtered_methods.sort_by(|a, b| {
      if let RustMethodArguments::SingleVariant(ref args) = a.arguments {
        let a_args = args;
        if let RustMethodArguments::SingleVariant(ref args) = b.arguments {
          a_args.cpp_method.c_name.cmp(&args.cpp_method.c_name)
        } else {
          unreachable!()
        }
      } else {
        unreachable!()
      }
    });
    let methods_count = filtered_methods.len();
    let mut type_declaration = None;
    let method = if methods_count > 1 {
      let first_method = filtered_methods[0].clone();
      let (self_argument, cpp_method_name) =
        if let RustMethodArguments::SingleVariant(ref args) = first_method.arguments {
          let self_argument = if !args.arguments.is_empty() && args.arguments[0].name == "self" {
            Some(args.arguments[0].clone())
          } else {
            None
          };
          (self_argument, args.cpp_method.cpp_method.full_name())
        } else {
          unreachable!()
        };
      let mut args_variants = Vec::new();
      let mut method_name = first_method.name.clone();
      let mut trait_name = try!(first_method.name.last_name()).clone();
      if let Some(self_arg_kind_caption) = self_arg_kind_caption {
        trait_name = format!("{}_{}", trait_name, self_arg_kind_caption);
        let name = try!(method_name.parts.pop().chain_err(|| "name can't be empty"));
        method_name.parts.push(format!("{}_{}", name, self_arg_kind_caption));
      }
      trait_name = trait_name.to_class_case() + "Args";
      if let RustMethodScope::Impl { ref type_name } = *scope {
        trait_name = format!("{}{}", try!(type_name.last_name()), trait_name);
      }
      let method_name_with_scope = match first_method.scope {
        RustMethodScope::Impl { ref type_name } => {
          format!("{}::{}",
                  try!(type_name.last_name()),
                  try!(method_name.last_name()))
        }
        RustMethodScope::TraitImpl { .. } => {
          return Err(unexpected("TraitImpl is totally not expected here").into())
        }
        RustMethodScope::Free => try!(method_name.last_name()).clone(),
      };
      let mut grouped_by_cpp_method: HashMap<_, Vec<_>> = HashMap::new();
      for method in filtered_methods {
        assert!(method.name == first_method.name);
        assert!(method.scope == first_method.scope);
        if let RustMethodArguments::SingleVariant(mut args) = method.arguments {
          if let Some(ref self_argument) = self_argument {
            assert!(args.arguments.len() > 0 && &args.arguments[0] == self_argument);
            args.arguments.remove(0);
          }
          fn allocation_place_marker(marker_name: &'static str) -> Result<RustMethodArgument> {
            Ok(RustMethodArgument {
              name: "allocation_place_marker".to_string(),
              ffi_index: None,
              argument_type: CompleteType {
                cpp_type: CppType::void(),
                cpp_ffi_type: CppType::void(),
                cpp_to_ffi_conversion: IndirectionChange::NoChange,
                rust_ffi_type: RustType::Void,
                rust_api_type: RustType::Common {
                  base: try!(RustName::new(vec!["cpp_utils".to_string(), marker_name.to_string()])),
                  generic_arguments: None,
                  is_const: false,
                  is_const2: false,
                  indirection: RustTypeIndirection::None,
                },
                rust_api_to_c_conversion: RustToCTypeConversion::None,
              },
            })
          }
          match args.cpp_method.allocation_place {
            ReturnValueAllocationPlace::Stack => {
              args.arguments.push(try!(allocation_place_marker("AsStruct")));
            }
            ReturnValueAllocationPlace::Heap => {
              args.arguments.push(try!(allocation_place_marker("AsBox")));
            }
            ReturnValueAllocationPlace::NotApplicable => {}
          }
          let mut cpp_method_key = args.cpp_method.cpp_method.clone();
          if let Some(v) = cpp_method_key.arguments_before_omitting {
            cpp_method_key.arguments = v;
            cpp_method_key.arguments_before_omitting = None;
          }
          add_to_multihash(&mut grouped_by_cpp_method, cpp_method_key, args.clone());
          args_variants.push(args);
        } else {
          unreachable!()
        }
      }

      let mut doc_items = Vec::new();
      let mut grouped_by_cpp_method_vec: Vec<_> = grouped_by_cpp_method.into_iter().collect();
      grouped_by_cpp_method_vec.sort_by(|&(ref a, _), &(ref b, _)| {
        a.short_text().cmp(&b.short_text())
      });
      for (cpp_method, variants) in grouped_by_cpp_method_vec {
        doc_items.push(doc_formatter::DocItem {
          doc: self.get_qt_doc_for_method(&cpp_method),
          cpp_fn: cpp_method.short_text(),
          rust_fns: try!(variants.iter().map_if_ok(|args| -> Result<_> {
            Ok(doc_formatter::rust_method_variant(args,
                                                  try!(method_name.last_name()),
                                                  try!(first_method.self_arg_kind()),
                                                  &self.config.crate_name))
          })),
          inherited_from: cpp_method.inherited_from.clone(),
        });
      }
      let doc = doc_formatter::method_doc(doc_items, &cpp_method_name);

      // overloaded methods
      let shared_arguments_for_trait = match self_argument {
        None => Vec::new(),
        Some(ref arg) => {
          let mut renamed_self = arg.clone();
          renamed_self.name = "original_self".to_string();
          vec![renamed_self]
        }
      };
      let shared_arguments = match self_argument {
        None => Vec::new(),
        Some(arg) => vec![arg],
      };
      let trait_lifetime = if shared_arguments.iter()
        .any(|x| x.argument_type.rust_api_type.is_ref()) {
        Some("a".to_string())
      } else {
        None
      };
      let method_link = match first_method.scope {
        RustMethodScope::Impl { ref type_name } => {
          format!("../struct.{}.html#method.{}",
                  try!(type_name.last_name()),
                  try!(method_name.last_name()))
        }
        RustMethodScope::TraitImpl { .. } => {
          return Err(unexpected("TraitImpl is totally not expected here").into())
        }
        RustMethodScope::Free => format!("../fn.{}.html", try!(method_name.last_name())),
      };
      let first_return_type = args_variants[0].return_type.rust_api_type.clone();
      let trait_return_type = if args_variants.iter()
        .all(|x| &x.return_type.rust_api_type == &first_return_type) {
        Some(first_return_type)
      } else {
        None
      };
      type_declaration = Some(RustTypeDeclaration {
        name: trait_name.clone(),
        kind: RustTypeDeclarationKind::MethodParametersTrait {
          shared_arguments: shared_arguments_for_trait,
          impls: args_variants,
          lifetime: trait_lifetime.clone(),
          return_type: trait_return_type.clone(),
        },
        doc: format!("This trait represents a set of arguments accepted by [{name}]({link}) \
                      method.",
                     name = method_name_with_scope,
                     link = method_link),
      });

      RustMethod {
        name: method_name,
        scope: first_method.scope,
        arguments: RustMethodArguments::MultipleVariants {
          params_trait_name: trait_name.clone(),
          params_trait_lifetime: trait_lifetime,
          params_trait_return_type: trait_return_type,
          shared_arguments: shared_arguments,
          variant_argument_name: "args".to_string(),
        },
        doc: doc,
      }
    } else {
      let mut method = try!(filtered_methods.pop().chain_err(|| "filtered_methods can't be empty"));
      if let Some(self_arg_kind_caption) = self_arg_kind_caption {
        let name = try!(method.name.parts.pop().chain_err(|| "name can't be empty"));
        method.name.parts.push(format!("{}_{}", name, self_arg_kind_caption));
      }

      if let RustMethodArguments::SingleVariant(ref args) = method.arguments {
        let doc_item = doc_formatter::DocItem {
          cpp_fn: args.cpp_method.cpp_method.short_text(),
          rust_fns: Vec::new(),
          doc: self.get_qt_doc_for_method(&args.cpp_method.cpp_method),
          inherited_from: args.cpp_method.cpp_method.inherited_from.clone(),
        };
        method.doc = doc_formatter::method_doc(vec![doc_item],
                                               &args.cpp_method.cpp_method.full_name());
      } else {
        unreachable!();
      }

      method
    };
    Ok((method, type_declaration))
  }

  fn get_qt_doc_for_method(&self, cpp_method: &CppMethod) -> Option<QtDocResultForMethod> {
    if let Some(ref qt_doc_data) = self.config.qt_doc_data {
      if let Some(ref inherited_from) = cpp_method.inherited_from {
        if let Some(ref declaration_code) = inherited_from.declaration_code {
          match qt_doc_data.doc_for_method(&inherited_from.doc_id,
                                           declaration_code,
                                           &inherited_from.short_text) {
            Ok(doc) => Some(doc),
            Err(msg) => {
              log::warning(format!("Failed to get documentation for method: {}: {}",
                                   &inherited_from.short_text,
                                   msg));
              None
            }
          }
        } else {
          None
        }
      } else if let Some(ref declaration_code) = cpp_method.declaration_code {
        match qt_doc_data.doc_for_method(&cpp_method.doc_id(),
                                         declaration_code,
                                         &cpp_method.short_text()) {
          Ok(doc) => Some(doc),
          Err(msg) => {
            log::warning(format!("Failed to get documentation for method: {}: {}",
                                 &cpp_method.short_text(),
                                 msg));
            None
          }

        }
      } else {
        None
      }
    } else {
      None
    }
  }

  /// Generates methods, trait implementations and overloading types
  /// for all specified methods. All methods must either be in the same
  /// RustMethodScope::Impl scope or be free functions in the same module.
  #[cfg_attr(feature="clippy", allow(for_kv_map))]
  fn process_functions<'b, I>(&self,
                              methods: I,
                              scope: &RustMethodScope)
                              -> Result<ProcessFunctionsResult>
    where I: Iterator<Item = &'b CppAndFfiMethod>
  {
    // Step 1: convert all methods to SingleVariant Rust methods and
    // split them by last name.
    let mut single_rust_methods: HashMap<String, Vec<RustMethod>> = HashMap::new();
    let mut result = ProcessFunctionsResult::default();
    for method in methods {
      if method.cpp_method.is_destructor() {
        match self.process_destructor(method, scope) {
          Ok(r) => result.trait_impls.push(r),
          Err(msg) => {
            log::warning(format!("Failed to generate destructor: {}\n{:?}\n", msg, method))
          }
        }
        continue;
      }
      match self.generate_function(method, scope, false) {
        Ok(rust_method) => {
          let name = try!(rust_method.name.last_name()).clone();
          add_to_multihash(&mut single_rust_methods, name, rust_method);
        }
        Err(err) => log::warning(err.to_string()),
      }
    }
    for (_, current_methods) in single_rust_methods {
      assert!(!current_methods.is_empty());
      // Step 2: for each method name, split methods by type of
      // their self argument. Overloading can't be emulated if self types
      // differ.
      let mut self_kind_to_methods: HashMap<_, Vec<_>> = HashMap::new();
      for method in current_methods {
        add_to_multihash(&mut self_kind_to_methods,
                         try!(method.self_arg_kind()),
                         method);
      }
      let all_self_args: Vec<_> = self_kind_to_methods.keys().cloned().collect();
      for (self_arg_kind, overloaded_methods) in self_kind_to_methods {
        let self_arg_kind_caption = if all_self_args.len() == 1 ||
                                       self_arg_kind == RustMethodSelfArgKind::ConstRef {
          None
        } else if self_arg_kind == RustMethodSelfArgKind::Static {
          Some("static")
        } else if self_arg_kind == RustMethodSelfArgKind::MutRef {
          if all_self_args.iter().any(|x| *x == RustMethodSelfArgKind::ConstRef) {
            Some("mut")
          } else {
            None
          }
        } else {
          return Err("unsupported self arg kinds combination".into());
        };

        assert!(!overloaded_methods.is_empty());
        // Step 3: remove method duplicates with the same argument types. For example,
        // there can be method1(libc::c_int) and method1(i32). It's valid in C++,
        // but can't be overloaded in Rust if types are the same.
        let mut all_real_args = HashMap::new();
        all_real_args.insert(ReturnValueAllocationPlace::Stack, HashSet::new());
        all_real_args.insert(ReturnValueAllocationPlace::Heap, HashSet::new());
        all_real_args.insert(ReturnValueAllocationPlace::NotApplicable, HashSet::new());
        let mut filtered_methods = Vec::new();
        for method in overloaded_methods {
          let ok = if let RustMethodArguments::SingleVariant(ref args) = method.arguments {
            let real_args: Vec<_> = try!(args.arguments
              .iter()
              .map_if_ok(|x| x.argument_type.rust_api_type.dealias_libc()));
            let set = try!(all_real_args.get_mut(&args.cpp_method.allocation_place)
              .chain_err(|| "all_real_args must contain every possible allocation place"));
            if set.contains(&real_args) {
              log::warning(format!("Removing method because another method with the same \
                                    argument types exists:\n{:?}",
                                   args.cpp_method.short_text()));
              false
            } else {
              set.insert(real_args);
              true
            }
          } else {
            unreachable!()
          };
          if ok {
            filtered_methods.push(method);
          }
        }
        // Step 4: generate overloaded method if count of methods is still > 1,
        // or accept a single method without change.
        let (method, type_declaration) =
          try!(self.process_method(filtered_methods, scope, self_arg_kind_caption));
        if method.doc.is_empty() {
          return Err(unexpected(format!("doc is empty! {:?}", method)).into());
        }
        result.methods.push(method);
        if let Some(r) = type_declaration {
          result.overloading_types.push(r);
        }
      }
    }
    result.methods.sort_by(|a, b| {
      a.name.last_name().unwrap_or(&String::new()).cmp(b.name.last_name().unwrap_or(&String::new()))
    });
    result.trait_impls.sort_by(|a, b| a.trait_name.to_string().cmp(&b.trait_name.to_string()));
    Ok(result)
  }

  /// Generates Rust representations of all FFI functions
  pub fn ffi(&self) -> Vec<(String, Vec<RustFFIFunction>)> {
    log::info("Generating Rust FFI functions.");
    let mut ffi_functions = Vec::new();

    for header in &self.input_data.cpp_ffi_headers {
      let mut functions = Vec::new();
      for method in &header.methods {
        match self.ffi_function(method) {
          Ok(function) => {
            functions.push(function);
          }
          Err(msg) => {
            log::warning(format!("Can't generate Rust FFI function for method:\n{}\n{}\n",
                                 method.short_text(),
                                 msg));
          }
        }
      }
      ffi_functions.push((header.include_file.clone(), functions));
    }
    ffi_functions
  }
}

// ---------------------------------
#[test]
fn remove_qt_prefix_and_convert_case_test() {
  assert_eq!(remove_qt_prefix_and_convert_case(&"OneTwo".to_string(), Case::Class, false),
             "OneTwo");
  assert_eq!(remove_qt_prefix_and_convert_case(&"OneTwo".to_string(), Case::Snake, false),
             "one_two");
  assert_eq!(remove_qt_prefix_and_convert_case(&"OneTwo".to_string(), Case::Class, true),
             "OneTwo");
  assert_eq!(remove_qt_prefix_and_convert_case(&"OneTwo".to_string(), Case::Snake, true),
             "one_two");
  assert_eq!(remove_qt_prefix_and_convert_case(&"QDirIterator".to_string(), Case::Class, false),
             "QDirIterator");
  assert_eq!(remove_qt_prefix_and_convert_case(&"QDirIterator".to_string(), Case::Snake, false),
             "q_dir_iterator");
  assert_eq!(remove_qt_prefix_and_convert_case(&"QDirIterator".to_string(), Case::Class, true),
             "DirIterator");
  assert_eq!(remove_qt_prefix_and_convert_case(&"QDirIterator".to_string(), Case::Snake, true),
             "dir_iterator");
}

#[cfg(test)]
fn calculate_rust_name_test_part(name: &'static str,
                                 include_file: &'static str,
                                 is_function: bool,
                                 expected: &[&'static str]) {
  assert_eq!(calculate_rust_name(&name.to_string(),
                                 &include_file.to_string(),
                                 is_function,
                                 None,
                                 &RustGeneratorConfig {
                                   crate_name: "qt_core".to_string(),
                                   remove_qt_prefix: true,
                                   qt_doc_data: None,
                                 })
               .unwrap(),
             RustName::new(expected.into_iter().map(|x| x.to_string()).collect()).unwrap());
}

#[test]
fn calculate_rust_name_test() {
  calculate_rust_name_test_part("myFunc1",
                                "QtGlobal",
                                true,
                                &["qt_core", "global", "my_func1"]);
  calculate_rust_name_test_part("QPointF",
                                "QPointF",
                                false,
                                &["qt_core", "point_f", "PointF"]);
  calculate_rust_name_test_part("QStringList::Iterator",
                                "QStringList",
                                false,
                                &["qt_core", "string_list", "Iterator"]);
  calculate_rust_name_test_part("QStringList::Iterator",
                                "QString",
                                false,
                                &["qt_core", "string", "string_list", "Iterator"]);
  calculate_rust_name_test_part("ns::func1",
                                "QRect",
                                true,
                                &["qt_core", "rect", "ns", "func1"]);
}

#[test]
fn prepare_enum_values_test_simple() {
  let r = prepare_enum_values(&[EnumValue {
                                  name: "var1".to_string(),
                                  value: 1,
                                },
                                EnumValue {
                                  name: "other_var2".to_string(),
                                  value: 2,
                                }],
                              "");
  assert_eq!(r.len(), 2);
  assert_eq!(r[0].name, "Var1");
  assert_eq!(r[0].value, 1);
  assert_eq!(r[1].name, "OtherVar2");
  assert_eq!(r[1].value, 2);
}

#[test]
fn prepare_enum_values_test_duplicates() {
  let r = prepare_enum_values(&[EnumValue {
                                  name: "var1".to_string(),
                                  value: 1,
                                },
                                EnumValue {
                                  name: "other_var2".to_string(),
                                  value: 2,
                                },
                                EnumValue {
                                  name: "other_var_dup".to_string(),
                                  value: 2,
                                }],
                              "");
  assert_eq!(r.len(), 2);
  assert_eq!(r[0].name, "Var1");
  assert_eq!(r[0].value, 1);
  assert_eq!(r[1].name, "OtherVar2");
  assert_eq!(r[1].value, 2);
}

#[test]
fn prepare_enum_values_test_prefix() {
  let r = prepare_enum_values(&[EnumValue {
                                  name: "OptionGood".to_string(),
                                  value: 1,
                                },
                                EnumValue {
                                  name: "OptionBad".to_string(),
                                  value: 2,
                                },
                                EnumValue {
                                  name: "OptionNecessaryEvil".to_string(),
                                  value: 3,
                                }],
                              "");
  assert_eq!(r.len(), 3);
  assert_eq!(r[0].name, "Good");
  assert_eq!(r[1].name, "Bad");
  assert_eq!(r[2].name, "NecessaryEvil");
}

#[test]
fn prepare_enum_values_test_suffix() {
  let r = prepare_enum_values(&[EnumValue {
                                  name: "BestFriend".to_string(),
                                  value: 1,
                                },
                                EnumValue {
                                  name: "GoodFriend".to_string(),
                                  value: 2,
                                },
                                EnumValue {
                                  name: "NoFriend".to_string(),
                                  value: 3,
                                }],
                              "");
  assert_eq!(r.len(), 3);
  assert_eq!(r[0].name, "Best");
  assert_eq!(r[1].name, "Good");
  assert_eq!(r[2].name, "No");
}

#[test]
fn prepare_enum_values_test_prefix_digits() {
  let r = prepare_enum_values(&[EnumValue {
                                  name: "Base32".to_string(),
                                  value: 1,
                                },
                                EnumValue {
                                  name: "Base64".to_string(),
                                  value: 2,
                                }],
                              "");
  assert_eq!(r.len(), 2);
  assert_eq!(r[0].name, "Base32");
  assert_eq!(r[1].name, "Base64");
}

#[test]
fn prepare_enum_values_test_suffix_empty() {
  let r = prepare_enum_values(&[EnumValue {
                                  name: "NonRecursive".to_string(),
                                  value: 1,
                                },
                                EnumValue {
                                  name: "Recursive".to_string(),
                                  value: 2,
                                }],
                              "");
  assert_eq!(r.len(), 2);
  assert_eq!(r[0].name, "NonRecursive");
  assert_eq!(r[1].name, "Recursive");
}

#[test]
fn prepare_enum_values_test_suffix_partial() {
  let r = prepare_enum_values(&[EnumValue {
                                  name: "PreciseTimer".to_string(),
                                  value: 1,
                                },
                                EnumValue {
                                  name: "CoarseTimer".to_string(),
                                  value: 2,
                                }],
                              "");
  assert_eq!(r.len(), 2);
  assert_eq!(r[0].name, "Precise");
  assert_eq!(r[1].name, "Coarse");
}
