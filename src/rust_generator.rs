use cpp_ffi_generator::{CppAndFfiData, CppFfiHeaderData};
use cpp_ffi_data::CppAndFfiMethod;
use cpp_type::{CppType, CppTypeBase, CppBuiltInNumericType, CppTypeIndirection,
               CppSpecificNumericTypeKind, CppTypeClassBase};
use cpp_ffi_data::{CppFfiType, IndirectionChange};
use rust_type::{RustName, RustType, CompleteType, RustTypeIndirection, RustFFIFunction,
                RustFFIArgument, RustToCTypeConversion};
use cpp_data::{CppTypeKind, EnumValue, CppTypeData};
use rust_info::{RustTypeDeclaration, RustTypeDeclarationKind, RustTypeWrapperKind, RustModule,
                RustMethod, RustMethodScope, RustMethodArgument, RustMethodArgumentsVariant,
                RustMethodArguments, TraitImpl, TraitName, RustEnumValue};
use cpp_method::{CppMethod, ReturnValueAllocationPlace};
use cpp_ffi_data::CppFfiArgumentMeaning;
use utils::{CaseOperations, VecCaseOperations, WordIterator, add_to_multihash, JoinWithString};
use caption_strategy::TypeCaptionStrategy;
use log;
use qt_doc_parser::{QtDocData, QtDocResultForMethod};
use doc_formatter;
use std::collections::{HashMap, HashSet};
pub use serializable::{RustProcessedTypeKind, RustProcessedTypeInfo};

/// Mode of case conversion
enum Case {
  /// Class case: "OneTwo"
  Class,
  /// Snake case: "one_two"
  Snake,
}




/// If remove_qt_prefix is true, removes "Q" or "Qt"
/// if it is first word of the string and not the only one word.
/// Also converts case of the words.
fn remove_qt_prefix_and_convert_case(s: &String, case: Case, remove_qt_prefix: bool) -> String {
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
/// processing as remove_qt_prefix_and_convert_case() for snake case.
fn include_file_to_module_name(include_file: &String, remove_qt_prefix: bool) -> String {
  let mut r = include_file.clone();
  if r.ends_with(".h") {
    r = r[0..r.len() - 2].to_string();
  }
  remove_qt_prefix_and_convert_case(&r, Case::Snake, remove_qt_prefix)
}

/// Adds "_" to a string if it is a reserved word in Rust
#[cfg_attr(rustfmt, rustfmt_skip)]
fn sanitize_rust_identifier(name: &String) -> String {
  match name.as_ref() {
    "abstract" | "alignof" | "as" | "become" | "box" | "break" | "const" |
    "continue" | "crate" | "do" | "else" | "enum" | "extern" | "false" |
    "final" | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" |
    "macro" | "match" | "mod" | "move" | "mut" | "offsetof" | "override" |
    "priv" | "proc" | "pub" | "pure" | "ref" | "return" | "Self" | "self" |
    "sizeof" | "static" | "struct" | "super" | "trait" | "true" | "type" |
    "typeof" | "unsafe" | "unsized" | "use" | "virtual" | "where" | "while" |
    "yield" => format!("{}_", name),
    _ => name.clone()
  }
}

/// Prepares enum variants for being represented in Rust:
/// - Converts variant names to proper case;
/// - Removes duplicate variants that have the same associated value.
/// Rust does not allow such duplicates.
/// - If there is only one variant, adds another variant.
/// Rust does not allow repr(C) enums having only one variant.
fn prepare_enum_values(values: &Vec<EnumValue>, name: &String) -> Vec<RustEnumValue> {
  // TODO: tests for prepare_enum_values
  // TODO: remove shared prefix from variants
  let mut value_to_variant: HashMap<i64, RustEnumValue> = HashMap::new();
  for variant in values {
    let value = variant.value;
    if value_to_variant.contains_key(&value) {
      log::warning(format!("warning: {}: duplicated enum variant removed: {} (previous variant: \
                            {})",
                           name,
                           variant.name,
                           value_to_variant.get(&value).unwrap().name));
    } else {
      value_to_variant.insert(value,
                              RustEnumValue {
                                name: sanitize_rust_identifier(&variant.name.to_class_case()),
                                cpp_name: Some(variant.name.clone()),
                                value: variant.value,
                                doc: format!("C++ variant: {}", &variant.name),
                              });
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
                              doc: format!("This variant is added in Rust because enums with one \
                                            variant and C representation are not supported."),
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
        .find(|item| item.is_empty() || item.chars().next().unwrap().is_digit(10))
        .is_none() {
        Some(new_names)
      } else {
        None
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

/// Config for rust_generator module.
pub struct RustGeneratorConfig {
  /// Name of generated crate
  pub crate_name: String,
  /// List of module names that should not be generated
  pub module_blacklist: Vec<String>,
  /// Flag instructing to remove leading "Q" and "Qt"
  /// from identifiers.
  pub remove_qt_prefix: bool,

  pub qt_doc_data: Option<QtDocData>,
}
// TODO: when supporting other libraries, implement removal of arbitrary prefixes

/// Execute processing
pub fn run(input_data: CppAndFfiData,

           dependency_cpp_types: &Vec<CppTypeData>,
           dependency_rust_types: Vec<RustProcessedTypeInfo>,
           config: RustGeneratorConfig)
           -> RustGeneratorOutput {
  let generator = RustGenerator {
    processed_types: generate_type_map(&input_data, dependency_cpp_types, &config),
    dependency_types: dependency_rust_types,
    input_data: input_data,
    config: config,
  };
  let mut modules = Vec::new();
  for header in &generator.input_data.cpp_ffi_headers {
    if let Some(module) = generator.generate_module_from_header(header) {
      modules.push(module);
    }
  }
  RustGeneratorOutput {
    ffi_functions: generator.ffi(),
    modules: modules,
    processed_types: generator.processed_types,
  }
}

/// Generates RustName for specified function or type name,
/// including crate name and modules list.
fn calculate_rust_name(name: &String,
                       include_file: &String,
                       is_function: bool,
                       config: &RustGeneratorConfig)
                       -> RustName {
  let mut split_parts: Vec<_> = name.split("::").collect();
  let last_part = remove_qt_prefix_and_convert_case(&split_parts.pop().unwrap().to_string(),
                                                    if is_function {
                                                      Case::Snake
                                                    } else {
                                                      Case::Class
                                                    },
                                                    config.remove_qt_prefix);

  let mut parts = Vec::new();
  parts.push(config.crate_name.clone());
  parts.push(include_file_to_module_name(&include_file, config.remove_qt_prefix));
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

/// Generates Rust names for all available C++ type
/// and free functions. Class member methods are not included in
/// the map. Their Rust equivalents depend on their classes'
/// equivalents.
fn generate_type_map(input_data: &CppAndFfiData,
                     dependency_cpp_types: &Vec<CppTypeData>,
                     config: &RustGeneratorConfig)
                     -> Vec<RustProcessedTypeInfo> {
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
        CppTypeKind::Class { ref size, .. } => RustProcessedTypeKind::Class { size: size.unwrap() },
        CppTypeKind::Enum { ref values } => RustProcessedTypeKind::Enum { values: values.clone() },
      },
      rust_name: calculate_rust_name(&type_info.name, &type_info.include_file, false, config),
    });
  }
  for (class_name, list) in &input_data.cpp_data.template_instantiations {
    println!("TEST1: {} {:?}", class_name, list);
    let include_file = match input_data.cpp_data
      .types
      .iter()
      .chain(dependency_cpp_types.iter())
      .find(|x| &x.name == class_name) {
      Some(class_type_info) => &class_type_info.include_file,
      None => {
        log::warning(format!("Failed to process template instantiation: type info not found: \
                              {:?}",
                             class_name));
        continue;
      }
    };

    for ins in list {
      // TODO: use Rust names for template args
      let name = format!("{}_{}",
                         class_name,
                         ins.template_arguments
                           .iter()
                           .map(|x| x.caption(TypeCaptionStrategy::Full))
                           .join("_"));
      result.push(RustProcessedTypeInfo {
        cpp_name: class_name.clone(),
        cpp_template_arguments: Some(ins.template_arguments.clone()),
        kind: RustProcessedTypeKind::Class { size: ins.size },
        rust_name: calculate_rust_name(&name, include_file, false, config),
      });
      println!("TEST2: {:?}", result.last().unwrap());
    }
  }
  result
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

impl RustGenerator {
  /// Generates CompleteType from CppFfiType, adding
  /// Rust API type, Rust FFI type and conversion between them.
  fn complete_type(&self,
                   cpp_ffi_type: &CppFfiType,
                   argument_meaning: &CppFfiArgumentMeaning)
                   -> Result<CompleteType, String> {
    let rust_ffi_type = try!(self.ffi_type(&cpp_ffi_type.ffi_type));
    let mut rust_api_type = rust_ffi_type.clone();
    let mut rust_api_to_c_conversion = RustToCTypeConversion::None;
    if let RustType::Common { ref mut indirection, .. } = rust_api_type {
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
          *indirection = RustTypeIndirection::None;
          rust_api_to_c_conversion = RustToCTypeConversion::ValueToPtr;
        }
        IndirectionChange::ReferenceToPointer => {
          assert!(indirection == &RustTypeIndirection::Ptr);
          *indirection = RustTypeIndirection::Ref { lifetime: None };
          rust_api_to_c_conversion = RustToCTypeConversion::RefToPtr;
        }
        IndirectionChange::QFlagsToUInt => {}
      }
    }
    if cpp_ffi_type.conversion == IndirectionChange::QFlagsToUInt {
      rust_api_to_c_conversion = RustToCTypeConversion::QFlagsToUInt;
      let enum_type =
        if let CppTypeBase::Class(CppTypeClassBase { ref template_arguments, .. }) =
               cpp_ffi_type.original_type.base {
          let args = template_arguments.as_ref().unwrap();
          assert!(args.len() == 1);
          if let CppTypeBase::Enum { ref name } = args[0].base {
            match self.find_type_info(|x| &x.cpp_name == name) {
              None => return Err(format!("Type has no Rust equivalent: {}", name)),
              Some(info) => info.rust_name.clone(),
            }
          } else {
            panic!("invalid original type for QFlags");
          }
        } else {
          panic!("invalid original type for QFlags");
        };
      rust_api_type = RustType::Common {
        base: RustName::new(vec!["qt_core".to_string(), "flags".to_string(), "Flags".to_string()]),
        generic_arguments: Some(vec![RustType::Common {
                                       base: enum_type,
                                       generic_arguments: None,
                                       indirection: RustTypeIndirection::None,
                                       is_const: false,
                                     }]),
        indirection: RustTypeIndirection::None,
        is_const: false,
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

  fn find_type_info<F>(&self, f: F) -> Option<&RustProcessedTypeInfo>
    where F: Fn(&RustProcessedTypeInfo) -> bool
  {
    match self.processed_types.iter().find(|x| f(x)) {
      None => self.dependency_types.iter().find(|x| f(x)),
      Some(info) => Some(info),
    }
  }

  /// Converts CppType to its exact Rust equivalent (FFI-compatible)
  fn ffi_type(&self, cpp_ffi_type: &CppType) -> Result<RustType, String> {
    let rust_name = match cpp_ffi_type.base {
      CppTypeBase::Void => {
        match cpp_ffi_type.indirection {
          CppTypeIndirection::None => return Ok(RustType::Void),
          _ => RustName::new(vec!["libc".to_string(), "c_void".to_string()]),
        }
      }
      CppTypeBase::BuiltInNumeric(ref numeric) => {
        if numeric == &CppBuiltInNumericType::Bool {
          RustName::new(vec!["bool".to_string()])
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
            _ => return Err(format!("unsupported numeric type: {:?}", numeric)),
          };
          RustName::new(vec!["libc".to_string(), own_name.to_string()])
        }
      }
      CppTypeBase::SpecificNumeric { ref bits, ref kind, .. } => {
        let letter = match *kind {
          CppSpecificNumericTypeKind::Integer { ref is_signed } => {
            if *is_signed { "i" } else { "u" }
          }
          CppSpecificNumericTypeKind::FloatingPoint => "f",
        };
        RustName::new(vec![format!("{}{}", letter, bits)])
      }
      CppTypeBase::PointerSizedInteger { ref is_signed, .. } => {
        RustName::new(vec![if *is_signed { "isize" } else { "usize" }.to_string()])
      }
      CppTypeBase::Enum { ref name } => {
        match self.find_type_info(|x| &x.cpp_name == name) {
          None => return Err(format!("Type has no Rust equivalent: {}", name)),
          Some(ref info) => info.rust_name.clone(),
        }
      }
      CppTypeBase::Class(ref name_and_args) => {
        match self.find_type_info(|x| {
          &x.cpp_name == &name_and_args.name &&
          &x.cpp_template_arguments == &name_and_args.template_arguments
        }) {
          None => return Err(format!("Type has no Rust equivalent: {:?}", name_and_args)),
          Some(ref info) => info.rust_name.clone(),
        }
      }
      CppTypeBase::FunctionPointer { ref return_type,
                                     ref arguments,
                                     ref allows_variadic_arguments } => {
        if *allows_variadic_arguments {
          return Err(format!("Function pointers with variadic arguments are not supported"));
        }
        let mut rust_args = Vec::new();
        for arg in arguments {
          rust_args.push(try!(self.ffi_type(arg)));
        }
        let rust_return_type = try!(self.ffi_type(return_type));
        return Ok(RustType::FunctionPointer {
          arguments: rust_args,
          return_type: Box::new(rust_return_type),
        });
      }
      CppTypeBase::TemplateParameter { .. } => panic!("invalid cpp type"),
    };
    return Ok(RustType::Common {
      base: rust_name,
      is_const: cpp_ffi_type.is_const,
      indirection: match cpp_ffi_type.indirection {
        CppTypeIndirection::None => RustTypeIndirection::None,
        CppTypeIndirection::Ptr => RustTypeIndirection::Ptr,
        CppTypeIndirection::PtrPtr => RustTypeIndirection::PtrPtr,
        _ => return Err(format!("unsupported level of indirection: {:?}", cpp_ffi_type)),
      },
      generic_arguments: None,
    });
  }

  /// Generates exact Rust equivalent of CppAndFfiMethod object
  /// (FFI-compatible)
  fn ffi_function(&self, data: &CppAndFfiMethod) -> Result<RustFFIFunction, String> {
    let mut args = Vec::new();
    for arg in &data.c_signature.arguments {
      let rust_type = try!(self.ffi_type(&arg.argument_type.ffi_type));
      args.push(RustFFIArgument {
        name: sanitize_rust_identifier(&arg.name),
        argument_type: rust_type,
      });
    }
    Ok(RustFFIFunction {
      return_type: try!(self.ffi_type(&data.c_signature.return_type.ffi_type)),
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
  fn process_type(&self,
                  info: &RustProcessedTypeInfo,
                  c_header: &CppFfiHeaderData)
                  -> ProcessTypeResult {
    match info.kind {
      RustProcessedTypeKind::Enum { ref values } => {
        let mut is_flaggable = false;
        let template_arg_sample = CppType {
          is_const: false,
          indirection: CppTypeIndirection::None,
          base: CppTypeBase::Enum { name: info.cpp_name.clone() },
        };

        for flag_owner_name in &["QFlags", "QUrlTwoFlags"] {
          if let Some(instantiations) = self.input_data
            .cpp_data
            .template_instantiations
            .get(&flag_owner_name.to_string()) {
            if instantiations.iter()
              .find(|ins| {
                ins.template_arguments
                  .iter()
                  .find(|&arg| arg == &template_arg_sample)
                  .is_some()
              })
              .is_some() {
              is_flaggable = true;
              break;
            }
          }

        }

        // TODO: export Qt doc for enum and its variants
        let doc = format!("C++ type: {}", &info.cpp_name);
        ProcessTypeResult {
          main_type: RustTypeDeclaration {
            name: info.rust_name.last_name().clone(),
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
        }
      }
      RustProcessedTypeKind::Class { ref size } => {
        let methods_scope = RustMethodScope::Impl { type_name: info.rust_name.clone() };
        let class_type = CppTypeClassBase {
          name: info.cpp_name.clone(),
          template_arguments: info.cpp_template_arguments.clone(),
        };
        let methods = c_header.methods
          .iter()
          .filter(|&x| {
            if let Some(ref info) = x.cpp_method.class_membership {
              &info.class_type == &class_type
            } else {
              false
            }
          });
        let functions_result = self.process_functions(methods, &methods_scope);
        // TODO: use type_to_cpp_code_permissive to get more beautiful templates
        // TODO: export Qt doc
        let doc = format!("C++ type: {}", class_type.to_cpp_code().unwrap());

        ProcessTypeResult {
          main_type: RustTypeDeclaration {
            name: info.rust_name.last_name().clone(),
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
        }
      }
    }
  }

  /// Generates a Rust module (including nested modules) from
  /// specified C++ header.
  pub fn generate_module_from_header(&self, c_header: &CppFfiHeaderData) -> Option<RustModule> {
    let module_last_name = include_file_to_module_name(&c_header.include_file,
                                                       self.config.remove_qt_prefix);
    if self.config.module_blacklist.iter().find(|&x| x == &module_last_name).is_some() {
      log::info(format!("Skipping module {}", module_last_name));
      return None;
    }
    let module_name = RustName::new(vec![self.config.crate_name.clone(), module_last_name]);
    self.generate_module(c_header, &module_name)
  }

  /// Generates a Rust module with specified name from specified
  /// C++ header. If the module should have nested modules,
  /// this function calls itself recursively with nested module name
  /// but the same header data.
  pub fn generate_module(&self,
                         c_header: &CppFfiHeaderData,
                         module_name: &RustName)
                         -> Option<RustModule> {
    // TODO: check that all methods and types has been processed
    log::info(format!("Generating Rust module {}", module_name.full_name(None)));

    let mut direct_submodules = HashSet::new();
    let mut module = RustModule {
      name: module_name.last_name().clone(),
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
          let mut result = self.process_type(&type_data, c_header);
          module.types.push(result.main_type);
          rust_overloading_types.append(&mut result.overloading_types);
        }
      }


      for method in &c_header.methods {
        if method.cpp_method.class_membership.is_none() {
          let rust_name = calculate_rust_name(&method.cpp_method.name,
                                              &method.cpp_method.include_file,
                                              true,
                                              &self.config);

          if check_name(&rust_name) {
            good_methods.push(method);
          }
        }
      }
    }
    for name in direct_submodules {
      let mut new_name = module_name.clone();
      new_name.parts.push(name);
      if let Some(r) = self.generate_module(c_header, &new_name) {
        module.submodules.push(r);
      }
    }
    let mut free_functions_result =
      self.process_functions(good_methods.into_iter(), &RustMethodScope::Free);
    assert!(free_functions_result.trait_impls.is_empty());
    module.functions = free_functions_result.methods;
    rust_overloading_types.append(&mut free_functions_result.overloading_types);
    if rust_overloading_types.len() > 0 {
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
      return None;
    }
    Some(module)
  }

  /// Converts one function to a RustMethod
  fn generate_function(&self,
                       method: &CppAndFfiMethod,
                       scope: &RustMethodScope,
                       generate_doc: bool)
                       -> Result<RustMethod, String> {
    if method.cpp_method.is_operator() {
      // TODO: implement operator traits
      return Err(format!("operators are not supported yet"));
    }
    let mut arguments = Vec::new();
    let mut return_type_info = None;
    for (arg_index, arg) in method.c_signature.arguments.iter().enumerate() {
      match self.complete_type(&arg.argument_type, &arg.meaning) {
        Ok(complete_type) => {
          if arg.meaning == CppFfiArgumentMeaning::ReturnValue {
            assert!(return_type_info.is_none());
            return_type_info = Some((complete_type, Some(arg_index as i32)));
          } else {
            arguments.push(RustMethodArgument {
              ffi_index: Some(arg_index as i32),
              argument_type: complete_type,
              name: if arg.meaning == CppFfiArgumentMeaning::This {
                "self".to_string()
              } else {
                sanitize_rust_identifier(&arg.name.to_snake_case())
              },
            });
          }
        }
        Err(msg) => {
          return Err(format!("Can't generate Rust method for method:\n{}\n{}\n",
                             method.short_text(),
                             msg));
        }
      }
    }
    if return_type_info.is_none() {
      // none of the arguments has return value meaning,
      // so FFI return value must be used
      match self.complete_type(&method.c_signature.return_type,
                               &CppFfiArgumentMeaning::ReturnValue) {
        Ok(mut r) => {
          if method.allocation_place == ReturnValueAllocationPlace::Heap &&
             !method.cpp_method.is_destructor() {
            if let RustType::Common { ref mut indirection, .. } = r.rust_api_type {
              assert!(*indirection == RustTypeIndirection::None);
              *indirection = RustTypeIndirection::Ptr;
            } else {
              panic!("unexpected void type");
            }
            assert!(r.cpp_type.indirection == CppTypeIndirection::None);
            assert!(r.cpp_to_ffi_conversion == IndirectionChange::ValueToPointer);
            assert!(r.rust_api_to_c_conversion == RustToCTypeConversion::ValueToPtr);
            r.rust_api_to_c_conversion = RustToCTypeConversion::None;

          }
          return_type_info = Some((r, None));
        }
        Err(msg) => {
          return Err(format!("Can't generate Rust method for method:\n{}\n{}\n",
                             method.short_text(),
                             msg));
        }
      }
    } else {
      // an argument has return value meaning, so
      // FFI return type must be void
      assert!(method.c_signature.return_type == CppFfiType::void());
    }
    let mut return_type_info1 = return_type_info.unwrap();
    if return_type_info1.0.rust_api_type.is_ref() {
      if arguments.iter().find(|arg| arg.argument_type.rust_api_type.is_ref()).is_none() {
        log::warning(format!("Method returns a reference but doesn't receive a reference: {}",
                             method.short_text()));
        log::warning("Assuming static lifetime of return value.");
        return_type_info1.0.rust_api_type =
          return_type_info1.0.rust_api_type.with_lifetime("static".to_string());
      }
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
      name: self.method_rust_name(method),
      scope: scope.clone(),
      arguments: RustMethodArguments::SingleVariant(RustMethodArgumentsVariant {
        arguments: arguments,
        cpp_method: method.clone(),
        return_type: return_type_info1.0,
        return_type_ffi_index: return_type_info1.1,
      }),
      doc: doc,
    })
  }

  /// Returns method name. For class member functions, the name doesn't
  /// include class name and scope. For free functions, the name includes
  /// modules.
  fn method_rust_name(&self, method: &CppAndFfiMethod) -> RustName {
    let mut name = if method.cpp_method.class_membership.is_none() {
      calculate_rust_name(&method.cpp_method.name,
                          &method.cpp_method.include_file,
                          true,
                          &self.config)
    } else {
      let x = if method.cpp_method.is_constructor() {
        "new".to_string()
      } else {
        method.cpp_method.name.to_snake_case()
      };
      RustName::new(vec![x])
    };
    let sanitized = sanitize_rust_identifier(name.last_name());
    if &sanitized != name.last_name() {
      name.parts.pop().unwrap();
      name.parts.push(sanitized);
    }
    name
  }

  fn process_destructor(&self,
                        method: &CppAndFfiMethod,
                        scope: &RustMethodScope)
                        -> Result<TraitImpl, String> {
    if let &RustMethodScope::Impl { ref type_name } = scope {
      match method.allocation_place {
        ReturnValueAllocationPlace::Stack => {
          let mut method = try!(self.generate_function(method, scope, true));
          method.name = RustName::new(vec!["drop".to_string()]);
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
          panic!("destructor must have allocation place")
        }
      }
    } else {
      panic!("destructor must be in class scope");
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
                    use_self_arg_caption: bool)
                    -> (RustMethod, Option<RustTypeDeclaration>) {
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
          let self_argument = if args.arguments.len() > 0 && args.arguments[0].name == "self" {
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
      let mut trait_name = first_method.name.last_name().clone();
      if use_self_arg_caption {
        trait_name = format!("{}_{}", trait_name, first_method.self_arg_kind().caption());
        let name = method_name.parts.pop().unwrap();
        let caption = first_method.self_arg_kind().caption();
        method_name.parts.push(format!("{}_{}", name, caption));
      }
      trait_name = trait_name.to_class_case() + "Args";
      if let &RustMethodScope::Impl { ref type_name } = scope {
        trait_name = format!("{}{}", type_name.last_name(), trait_name);
      }
      let method_name_with_scope = match first_method.scope {
        RustMethodScope::Impl { ref type_name } => {
          format!("{}::{}", type_name.last_name(), method_name.last_name())
        }
        RustMethodScope::TraitImpl { .. } => panic!("TraitImpl is totally not expected here"),
        RustMethodScope::Free => method_name.last_name().clone(),
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
          fn allocation_place_marker(marker_name: &'static str) -> RustMethodArgument {
            RustMethodArgument {
              name: "allocation_place_marker".to_string(),
              ffi_index: None,
              argument_type: CompleteType {
                cpp_type: CppType::void(),
                cpp_ffi_type: CppType::void(),
                cpp_to_ffi_conversion: IndirectionChange::NoChange,
                rust_ffi_type: RustType::Void,
                rust_api_type: RustType::Common {
                  base: RustName::new(vec!["cpp_box".to_string(), marker_name.to_string()]),
                  generic_arguments: None,
                  is_const: false,
                  indirection: RustTypeIndirection::None,
                },
                rust_api_to_c_conversion: RustToCTypeConversion::None,
              },
            }
          }
          match args.cpp_method.allocation_place {
            ReturnValueAllocationPlace::Stack => {
              args.arguments.push(allocation_place_marker("RustManaged"));
            }
            ReturnValueAllocationPlace::Heap => {
              args.arguments.push(allocation_place_marker("CppPointer"));
            }
            ReturnValueAllocationPlace::NotApplicable => {}
          }
          let mut cpp_method_key = args.cpp_method.cpp_method.clone();
          if cpp_method_key.arguments_before_omitting.is_some() {
            cpp_method_key.arguments = cpp_method_key.arguments_before_omitting.unwrap();
            cpp_method_key.arguments_before_omitting = None;
          }
          add_to_multihash(&mut grouped_by_cpp_method, &cpp_method_key, args.clone());
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
        let cpp_short_text = cpp_method.short_text();
        if let Some(ref qt_doc_data) = self.config.qt_doc_data {
          if let Some(ref declaration_code) = cpp_method.declaration_code {
            let doc = match qt_doc_data.doc_for_method(&cpp_method.doc_id(),
                                                       declaration_code,
                                                       &cpp_short_text) {
              Ok(doc) => Some(doc),
              Err(msg) => {
                log::warning(format!("Failed to get documentation for method: {}: {}",
                                     &cpp_method.short_text(),
                                     msg));
                None
              }
            };
            doc_items.push(doc_formatter::DocItem {
              doc: doc,
              cpp_fn: cpp_short_text,
              rust_fns: variants.iter()
                .map(|args| {
                  doc_formatter::rust_method_variant(args,
                                                     method_name.last_name(),
                                                     first_method.self_arg_kind(),
                                                     &self.config.crate_name)
                })
                .collect(),
              inherited_from: cpp_method.inherited_from.clone(),
            });

          }
        }
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
        .find(|x| x.argument_type.rust_api_type.is_ref())
        .is_some() {
        Some("a".to_string())
      } else {
        None
      };
      let method_link = match first_method.scope {
        RustMethodScope::Impl { ref type_name } => {
          format!("../struct.{}.html#method.{}",
                  type_name.last_name(),
                  method_name.last_name())
        }
        RustMethodScope::TraitImpl { .. } => panic!("TraitImpl is totally not expected here"),
        RustMethodScope::Free => format!("../fn.{}.html", method_name.last_name()),
      };
      type_declaration = Some(RustTypeDeclaration {
        name: trait_name.clone(),
        kind: RustTypeDeclarationKind::MethodParametersTrait {
          shared_arguments: shared_arguments_for_trait,
          impls: args_variants,
          lifetime: trait_lifetime.clone(),
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
          shared_arguments: shared_arguments,
          variant_argument_name: "args".to_string(),
        },
        doc: doc,
      }
    } else {
      let mut method = filtered_methods.pop().unwrap();
      if use_self_arg_caption {
        let name = method.name.parts.pop().unwrap();
        let caption = method.self_arg_kind().caption();
        method.name.parts.push(format!("{}_{}", name, caption));
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
    (method, type_declaration)
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
  fn process_functions<'b, I>(&self, methods: I, scope: &RustMethodScope) -> ProcessFunctionsResult
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
          let name = rust_method.name.last_name().clone();
          add_to_multihash(&mut single_rust_methods, &name, rust_method);
        }
        Err(msg) => log::warning(msg),
      }
    }
    for (_method_name, current_methods) in single_rust_methods {
      assert!(!current_methods.is_empty());
      // Step 2: for each method name, split methods by type of
      // their self argument. Overloading can't be emulated if self types
      // differ.
      let mut self_kind_to_methods: HashMap<_, Vec<_>> = HashMap::new();
      for method in current_methods {
        add_to_multihash(&mut self_kind_to_methods, &method.self_arg_kind(), method);
      }
      let use_self_arg_caption = self_kind_to_methods.len() > 1;

      for (_self_arg_kind, overloaded_methods) in self_kind_to_methods {
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
            let real_args: Vec<_> = args.arguments
              .iter()
              .map(|x| x.argument_type.rust_api_type.dealias_libc())
              .collect();
            if all_real_args.get_mut(&args.cpp_method.allocation_place)
              .unwrap()
              .contains(&real_args) {
              log::warning(format!("Removing method because another method with the same \
                                    argument types exists:\n{:?}",
                                   args.cpp_method.short_text()));
              false
            } else {
              all_real_args.get_mut(&args.cpp_method.allocation_place).unwrap().insert(real_args);
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
          self.process_method(filtered_methods, scope, use_self_arg_caption);
        result.methods.push(method);
        if let Some(r) = type_declaration {
          result.overloading_types.push(r);
        }
      }
    }
    result.methods.sort_by(|a, b| a.name.last_name().cmp(b.name.last_name()));
    result.trait_impls.sort_by(|a, b| a.trait_name.to_string().cmp(&b.trait_name.to_string()));
    result
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
                                 &RustGeneratorConfig {
                                   crate_name: "qt_core".to_string(),
                                   remove_qt_prefix: true,
                                   module_blacklist: Vec::new(),
                                   qt_doc_data: None,
                                 }),
             RustName::new(expected.into_iter().map(|x| x.to_string()).collect()));
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
  let r = prepare_enum_values(&vec![EnumValue {
                                      name: "var1".to_string(),
                                      value: 1,
                                    },
                                    EnumValue {
                                      name: "other_var2".to_string(),
                                      value: 2,
                                    }],
                              &String::new());
  assert_eq!(r.len(), 2);
  assert_eq!(r[0].name, "Var1");
  assert_eq!(r[0].value, 1);
  assert_eq!(r[1].name, "OtherVar2");
  assert_eq!(r[1].value, 2);
}

#[test]
fn prepare_enum_values_test_duplicates() {
  let r = prepare_enum_values(&vec![EnumValue {
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
                              &String::new());
  assert_eq!(r.len(), 2);
  assert_eq!(r[0].name, "Var1");
  assert_eq!(r[0].value, 1);
  assert_eq!(r[1].name, "OtherVar2");
  assert_eq!(r[1].value, 2);
}

#[test]
fn prepare_enum_values_test_prefix() {
  let r = prepare_enum_values(&vec![EnumValue {
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
                              &String::new());
  assert_eq!(r.len(), 3);
  assert_eq!(r[0].name, "Good");
  assert_eq!(r[1].name, "Bad");
  assert_eq!(r[2].name, "NecessaryEvil");
}

#[test]
fn prepare_enum_values_test_suffix() {
  let r = prepare_enum_values(&vec![EnumValue {
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
                              &String::new());
  assert_eq!(r.len(), 3);
  assert_eq!(r[0].name, "Best");
  assert_eq!(r[1].name, "Good");
  assert_eq!(r[2].name, "No");
}

#[test]
fn prepare_enum_values_test_prefix_digits() {
  let r = prepare_enum_values(&vec![EnumValue {
                                      name: "Base32".to_string(),
                                      value: 1,
                                    },
                                    EnumValue {
                                      name: "Base64".to_string(),
                                      value: 2,
                                    }],
                              &String::new());
  assert_eq!(r.len(), 2);
  assert_eq!(r[0].name, "Base32");
  assert_eq!(r[1].name, "Base64");
}

#[test]
fn prepare_enum_values_test_suffix_empty() {
  let r = prepare_enum_values(&vec![EnumValue {
                                      name: "NonRecursive".to_string(),
                                      value: 1,
                                    },
                                    EnumValue {
                                      name: "Recursive".to_string(),
                                      value: 2,
                                    }],
                              &String::new());
  assert_eq!(r.len(), 2);
  assert_eq!(r[0].name, "NonRecursive");
  assert_eq!(r[1].name, "Recursive");
}

#[test]
fn prepare_enum_values_test_suffix_partial() {
  let r = prepare_enum_values(&vec![EnumValue {
                                      name: "PreciseTimer".to_string(),
                                      value: 1,
                                    },
                                    EnumValue {
                                      name: "CoarseTimer".to_string(),
                                      value: 2,
                                    }],
                              &String::new());
  assert_eq!(r.len(), 2);
  assert_eq!(r[0].name, "Precise");
  assert_eq!(r[1].name, "Coarse");
}

// TODO: remove ReturnType associated type if all return types are the same

// TODO: if name conflict involves 1 static and 1 non-static function,
// don't use "from_const"/"from_mut" and just add "static" to static fn;
// if there is only 1 const and 1 mut fn, just add "const" to const fn.

// TODO: methods should accept AsRef/AsMut insead of plain references

// TODO: implement AsRef/AsMut for CppBox and for up-casting derived classes

// TODO: alternative Option-based overloading strategy

// TODO: QList::indexOf - duplicate documentation

// TODO: rename allocation place markers to AsStruct and AsBox

// TODO: wrap operators as normal functions, for now

// TODO: AbstractItemModel::parent documentation doesn't show QObject::parent variant

// TODO: Window::base_size; qt_gui::rgb::gray - no documentation!
