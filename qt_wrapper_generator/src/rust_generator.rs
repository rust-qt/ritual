use cpp_ffi_generator::{CppAndFfiData, CppFfiHeaderData};
use cpp_and_ffi_method::CppAndFfiMethod;
use cpp_type::{CppTypeBase, CppBuiltInNumericType, CppTypeIndirection, CppSpecificNumericTypeKind};
use cpp_ffi_type::{CppFfiType, IndirectionChange};
use utils::JoinWithString;
use rust_type::{RustName, RustType, CompleteType, RustTypeIndirection, RustFFIFunction,
                RustFFIArgument, RustToCTypeConversion};
use cpp_data::{CppTypeKind, EnumValue, CppTypeData};
use std::path::PathBuf;
use std::collections::{HashMap, HashSet};
use log;
use rust_code_generator::RustCodeGenerator;
use rust_info::{RustTypeDeclaration, RustTypeDeclarationKind, RustTypeWrapperKind, RustModule,
                RustMethod, RustMethodScope, RustMethodArgument, RustMethodArgumentsVariant,
                RustMethodArguments};
use cpp_method::{CppMethod, CppMethodScope};
use cpp_ffi_function_argument::CppFfiArgumentMeaning;

fn include_file_to_module_name(include_file: &String) -> String {
  let mut r = include_file.clone();
  if r.ends_with(".h") {
    r = r[0..r.len() - 2].to_string();
  }
  r.to_snake_case()
}

#[cfg_attr(rustfmt, rustfmt_skip)]
fn sanitize_rust_var_name(name: &String) -> String {
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

extern crate inflector;
use self::inflector::Inflector;

trait CaseFix {
  fn to_class_case1(&self) -> Self;
}
impl CaseFix for String {
  fn to_class_case1(&self) -> Self {
    let mut x = self.to_camel_case();
    if x.len() > 0 {
      let c = x.remove(0);
      let cu: String = c.to_uppercase().collect();
      x = cu + &x;
    }
    x
  }
}

pub struct RustGenerator {
  input_data: CppAndFfiData,
  output_path: PathBuf,
  modules: Vec<RustModule>,
  crate_name: String,
  cpp_to_rust_type_map: HashMap<String, RustName>,
  processed_cpp_types: HashSet<String>,
  code_generator: RustCodeGenerator,
}

impl RustGenerator {
  pub fn new(input_data: CppAndFfiData, output_path: PathBuf) -> Self {
    let crate_name = "qt_core".to_string();
    RustGenerator {
      input_data: input_data,
      output_path: output_path.clone(),
      modules: Vec::new(),
      crate_name: crate_name.clone(),
      cpp_to_rust_type_map: HashMap::new(),
      processed_cpp_types: HashSet::new(),
      code_generator: RustCodeGenerator::new(crate_name, output_path),
    }
  }

  fn cpp_type_to_complete_type(&self, cpp_ffi_type: &CppFfiType) -> Result<CompleteType, String> {
    let rust_ffi_type = try!(self.cpp_type_to_rust_ffi_type(cpp_ffi_type));

    // TODO: convert pointers back to references or values
    let mut rust_api_type = rust_ffi_type.clone();
    let mut rust_api_to_c_conversion = RustToCTypeConversion::None;
    if let RustType::NonVoid { ref mut indirection, .. } = rust_api_type {
      match cpp_ffi_type.conversion.indirection_change {
        IndirectionChange::NoChange => {}
        IndirectionChange::ValueToPointer => {
          assert!(indirection == &RustTypeIndirection::Ptr);
          *indirection = RustTypeIndirection::None;
          rust_api_to_c_conversion = RustToCTypeConversion::ValueToPtr;
        }
        IndirectionChange::ReferenceToPointer => {
          assert!(indirection == &RustTypeIndirection::Ptr);
          *indirection = RustTypeIndirection::Ref;
          rust_api_to_c_conversion = RustToCTypeConversion::RefToPtr;
        }
        IndirectionChange::QFlagsToUInt => {}
      }
    }
    if cpp_ffi_type.conversion.indirection_change == IndirectionChange::QFlagsToUInt {
      rust_api_to_c_conversion = RustToCTypeConversion::QFlagsToUInt;
      let enum_type = if let CppTypeBase::Class { ref template_arguments, .. } =
                             cpp_ffi_type.original_type.base {
        let args = template_arguments.as_ref().unwrap();
        assert!(args.len() == 1);
        if let CppTypeBase::Enum { ref name } = args[0].base {
          match self.cpp_to_rust_type_map.get(name) {
            None => return Err(format!("Type has no Rust equivalent: {}", name)),
            Some(rust_name) => rust_name.clone(),
          }
        } else {
          panic!("invalid original type for QFlags");
        }
      } else {
        panic!("invalid original type for QFlags");
      };
      rust_api_type = RustType::NonVoid {
        base: RustName {
          crate_name: "qt_core".to_string(),
          module_name: "q_flags".to_string(),
          own_name: "QFlags".to_string(),
        },
        generic_arguments: Some(vec![RustType::NonVoid {
                                       base: enum_type,
                                       generic_arguments: None,
                                       indirection: RustTypeIndirection::None,
                                       is_option: false,
                                       is_const: false,
                                     }]),
        indirection: RustTypeIndirection::None,
        is_option: false,
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


  fn cpp_type_to_rust_ffi_type(&self, cpp_ffi_type: &CppFfiType) -> Result<RustType, String> {
    let rust_name = match cpp_ffi_type.ffi_type.base {
      CppTypeBase::Void => {
        match cpp_ffi_type.ffi_type.indirection {
          CppTypeIndirection::None => return Ok(RustType::Void),
          _ => {
            RustName {
              crate_name: "libc".to_string(),
              module_name: "".to_string(),
              own_name: "c_void".to_string(),
            }
          }
        }
      }
      CppTypeBase::BuiltInNumeric(ref numeric) => {
        if numeric == &CppBuiltInNumericType::Bool {
          RustName {
            crate_name: "".to_string(),
            module_name: "".to_string(),
            own_name: "bool".to_string(),
          }
        } else {
          RustName {
            crate_name: "libc".to_string(),
            module_name: "".to_string(),
            own_name: match *numeric {
                        CppBuiltInNumericType::Bool => "c_schar", // TODO: get real type of bool
                        CppBuiltInNumericType::CharS => "c_char",
                        CppBuiltInNumericType::CharU => "c_char",
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
                      }
                      .to_string(),
          }
        }
      }
      CppTypeBase::SpecificNumeric { ref bits, ref kind, .. } => {
        let letter = match *kind {
          CppSpecificNumericTypeKind::Integer { ref is_signed } => {
            if *is_signed {
              "i"
            } else {
              "u"
            }
          }
          CppSpecificNumericTypeKind::FloatingPoint => "f",
        };
        RustName {
          crate_name: "".to_string(),
          module_name: "".to_string(),
          own_name: format!("{}{}", letter, bits),
        }
      }
      CppTypeBase::PointerSizedInteger { ref is_signed, .. } => {
        RustName {
          crate_name: "".to_string(),
          module_name: "".to_string(),
          own_name: if *is_signed {
                      "isize"
                    } else {
                      "usize"
                    }
                    .to_string(),
        }
      }
      CppTypeBase::Enum { ref name } => {
        match self.cpp_to_rust_type_map.get(name) {
          None => return Err(format!("Type has no Rust equivalent: {}", name)),
          Some(rust_name) => rust_name.clone(),
        }
      }
      CppTypeBase::Class { ref name, ref template_arguments } => {
        if template_arguments.is_some() {
          return Err(format!("template types are not supported here yet"));
        }
        match self.cpp_to_rust_type_map.get(name) {
          None => return Err(format!("Type has no Rust equivalent: {}", name)),
          Some(rust_name) => rust_name.clone(),
        }
      }
      CppTypeBase::FunctionPointer { .. } => {
        return Err(format!("function pointers are not supported here yet"))
      }
      CppTypeBase::TemplateParameter { .. } => panic!("invalid cpp type"),
    };
    return Ok(RustType::NonVoid {
      base: rust_name,
      is_const: cpp_ffi_type.ffi_type.is_const,
      indirection: match cpp_ffi_type.ffi_type.indirection {
        CppTypeIndirection::None => RustTypeIndirection::None,
        CppTypeIndirection::Ptr => RustTypeIndirection::Ptr,
        _ => return Err(format!("unsupported level of indirection: {:?}", cpp_ffi_type)),
      },
      is_option: false,
      generic_arguments: None,
    });
  }


  fn generate_rust_ffi_function(&self,
                                data: &CppAndFfiMethod,
                                module_name: &String)
                                -> Result<RustFFIFunction, String> {
    let mut args = Vec::new();
    for arg in &data.c_signature.arguments {
      let rust_type = try!(self.cpp_type_to_complete_type(&arg.argument_type)).rust_ffi_type;
      args.push(RustFFIArgument {
        name: sanitize_rust_var_name(&arg.name),
        argument_type: rust_type,
      });
    }
    Ok(RustFFIFunction {
      return_type: try!(self.cpp_type_to_complete_type(&data.c_signature.return_type))
                     .rust_ffi_type,
      name: RustName {
        crate_name: self.crate_name.clone(),
        module_name: module_name.clone(),
        own_name: data.c_name.clone(),
      },
      arguments: args,
    })
  }


  fn generate_type_map(&mut self) {
    for type_info in &self.input_data.cpp_data.types {
      let eliminated_name_prefix = format!("{}::", type_info.include_file);
      let mut new_name = type_info.name.clone();
      if new_name.starts_with(&eliminated_name_prefix) {
        new_name = new_name[eliminated_name_prefix.len()..].to_string();
      }
      new_name = new_name.replace("::", "_").to_class_case1();
      if let CppTypeKind::Class { size, .. } = type_info.kind {
        if size.is_none() {
          log::warning(format!("Rust type is not generated for a struct with unknown \
                                        size: {}",
                               type_info.name));
          continue;
        }
      }
      self.cpp_to_rust_type_map.insert(type_info.name.clone(),
                                       RustName {
                                         crate_name: self.crate_name.clone(),
                                         module_name:
                                           include_file_to_module_name(&type_info.include_file),
                                         own_name: new_name,
                                       });
    }
  }

  fn process_type(&self,
                  type_info: &CppTypeData,
                  methods: &Vec<CppAndFfiMethod>)
                  -> Option<RustTypeDeclaration> {
    let rust_type_name = self.cpp_to_rust_type_map.get(&type_info.name).unwrap();
    match type_info.kind {
      CppTypeKind::Enum { ref values } => {
        let mut value_to_variant: HashMap<i64, EnumValue> = HashMap::new();
        for variant in values {
          let value = variant.value;
          if value_to_variant.contains_key(&value) {
            log::warning(format!("warning: {}: duplicated enum variant removed: {} \
                                  (previous variant: {})",
                                 type_info.name,
                                 variant.name,
                                 value_to_variant.get(&value).unwrap().name));
          } else {
            value_to_variant.insert(value,
                                    EnumValue {
                                      name: variant.name.to_class_case1(),
                                      value: variant.value,
                                    });
          }
        }
        if value_to_variant.len() == 1 {
          let dummy_value = if value_to_variant.contains_key(&0) {
            1
          } else {
            0
          };
          value_to_variant.insert(dummy_value,
                                  EnumValue {
                                    name: "_Invalid".to_string(),
                                    value: dummy_value as i64,
                                  });
        }
        let mut values: Vec<_> = value_to_variant.into_iter()
                                                 .map(|(val, variant)| variant)
                                                 .collect();
        values.sort_by(|a, b| a.value.cmp(&b.value));
        return Some(RustTypeDeclaration {
          name: rust_type_name.own_name.clone(),
          kind: RustTypeDeclarationKind::CppTypeWrapper {
            kind: RustTypeWrapperKind::Enum { values: values },
            cpp_type_name: type_info.name.clone(),
            cpp_template_arguments: None,
          },
          methods: Vec::new(),
          traits: Vec::new(),
        });
      }
      CppTypeKind::Class { ref size, .. } => {
        let methods_scope = RustMethodScope::Impl { type_name: rust_type_name.clone() };
        return Some(RustTypeDeclaration {
          name: rust_type_name.own_name.clone(),
          kind: RustTypeDeclarationKind::CppTypeWrapper {
            kind: RustTypeWrapperKind::Struct { size: size.unwrap() },
            cpp_type_name: type_info.name.clone(),
            cpp_template_arguments: None,
          },
          methods: self.generate_functions(methods.iter()
                                                  .filter(|&x| {
                                                    x.cpp_method
                                                     .scope
                                                     .class_name() ==
                                                    Some(&type_info.name)
                                                  })
                                                  .collect(),
                                           &methods_scope),
          traits: Vec::new(),
        });
      }
    };


  }

  pub fn generate_all(&mut self) {
    self.generate_type_map();
    for header in &self.input_data.cpp_ffi_headers.clone() {
      self.generate_modules_from_header(header);
    }
    self.generate_ffi();
    self.code_generator.generate_lib_file(&self.output_path,
                                           &self.modules.iter().map(|x| x.name.clone()).collect());
  }

  pub fn generate_modules_from_header(&mut self, c_header: &CppFfiHeaderData) {
    let module_name = include_file_to_module_name(&c_header.include_file);
    if module_name == "flags" && self.crate_name == "qt_core" {
      log::info(format!("Skipping module {}::{}", self.crate_name, module_name));
      return;
    }
    let mut types = Vec::new();
    for type_info in &self.input_data
                          .cpp_data_by_headers
                          .get(&c_header.include_file)
                          .unwrap()
                          .types {
      if let Some(rust_type_name) = self.cpp_to_rust_type_map.get(&type_info.name) {
        if module_name == rust_type_name.module_name {
          types.push(type_info.clone());
        } else {
          panic!("unexpected module name mismatch: {}, {:?}",
                 module_name,
                 rust_type_name);
        }
      } else {
        // type is skipped: no rust name
      }
    }
    if let Some(module) = self.generate_module(&types,
                                               &c_header.methods,
                                               &module_name,
                                               &module_name,
                                               &String::new()) {
      self.code_generator.generate_module_file(&module);
      self.modules.push(module);
    }
  }

  pub fn generate_module(&mut self,
                         types: &Vec<CppTypeData>,
                         methods: &Vec<CppAndFfiMethod>,
                         module_name: &String,
                         full_modules_name: &String,
                         cpp_namespace_prefix: &String)
                         -> Option<RustModule> {
    log::info(format!("Generating Rust module {}::{}",
                      self.crate_name,
                      full_modules_name));

    struct SubModuleData {
      rust_name: String,
      types: Vec<CppTypeData>,
      methods: Vec<CppAndFfiMethod>,
    }

    let mut cpp_namespace_to_sub_module = HashMap::new();
    let mut good_types = Vec::new();
    let mut good_methods = Vec::new();
    {
      let mut check_namespace_name = |x: &String,
                                      t: Option<&CppTypeData>,
                                      m: Option<&CppAndFfiMethod>| {
        let cpp_name = x[cpp_namespace_prefix.len()..].to_string();
        if let Some(index) = cpp_name.find("::") {
          let new_namespace = cpp_name[0..index].to_string();
          if !cpp_namespace_to_sub_module.contains_key(&new_namespace) {
            let rust_name = new_namespace.to_snake_case();
            if &rust_name == module_name {
              //special case
              return true;
            }
            cpp_namespace_to_sub_module.insert(new_namespace.clone(),
                                               SubModuleData {
                                                 rust_name: rust_name,
                                                 types: Vec::new(),
                                                 methods: Vec::new(),
                                               });
            if let Some(t) = t {
              cpp_namespace_to_sub_module.get_mut(&new_namespace).unwrap().types.push(t.clone());
            }
            if let Some(m) = m {
              cpp_namespace_to_sub_module.get_mut(&new_namespace).unwrap().methods.push(m.clone());
            }
          }
          return false;
        }
        return true;
      };
      for type_data in types {
        if check_namespace_name(&type_data.name, Some(type_data), None) {
          good_types.push(type_data.clone());
        }
      }
      for method in methods {
        if method.cpp_method.scope == CppMethodScope::Global {
          if check_namespace_name(&method.cpp_method.name, None, Some(method)) {
            good_methods.push(method.clone());
          }
        }
      }
    }
    let mut submodules = Vec::new();
    for (cpp_namespace, submodule) in cpp_namespace_to_sub_module {
      let cpp_prefix = format!("{}{}::", cpp_namespace_prefix, cpp_namespace);
      if let Some(module) = self.generate_module(&submodule.types,
                                                 &submodule.methods,
                                                 &submodule.rust_name,
                                                 &format!("{}::{}",
                                                          full_modules_name,
                                                          submodule.rust_name),
                                                 &cpp_prefix) {
        submodules.push(module);
      }
    }

    let mut rust_types = Vec::new();

    for type_data in &good_types {
      if let Some(result) = self.process_type(type_data, &good_methods) {
        rust_types.push(result);
        // TODO: save RustTypeDeclaration vector instead of processed_cpp_types
        self.processed_cpp_types.insert(type_data.name.clone());
      }
    }
    let module = RustModule {
      name: module_name.clone(),
      full_modules_name: full_modules_name.clone(),
      crate_name: self.crate_name.clone(),
      types: rust_types,
      functions: self.generate_functions(good_methods.iter()
                                                     .filter(|&x| {
                                                       x.cpp_method
                                                        .scope ==
                                                       CppMethodScope::Global
                                                     })
                                                     .collect(),
                                         &RustMethodScope::Free),
      submodules: submodules,
    };
    return Some(module);
  }

  pub fn generate_functions(&self,
                            methods: Vec<&CppAndFfiMethod>,
                            scope: &RustMethodScope)
                            -> Vec<RustMethod> {
    let mut r = Vec::new();
    let mut method_names = HashSet::new();
    for method in &methods {
      // TODO: use cpp name instead?
      if !method_names.contains(&method.c_method_name) {
        method_names.insert(method.c_method_name.clone());
      }
    }
    for method_name in method_names {
      let current_methods: Vec<_> = methods.clone()
                                           .into_iter()
                                           .filter(|m| &m.c_method_name == &method_name)
                                           .collect();
      if current_methods.len() == 1 {
        let method = current_methods[0];
        if method.cpp_method.kind.is_destructor() || method.cpp_method.kind.is_operator() {
          // TODO: implement Drop trait or other traits
          continue;
        }
        let mut arguments = Vec::new();
        let mut return_type_info = None;
        let mut fail = false;
        for (arg_index, arg) in method.c_signature.arguments.iter().enumerate() {
          match self.cpp_type_to_complete_type(&arg.argument_type) {
            Ok(complete_type) => {
              if arg.meaning == CppFfiArgumentMeaning::ReturnValue {
                assert!(return_type_info.is_none());
                return_type_info = Some((complete_type, Some(arg_index as i32)));
              } else {
                arguments.push(RustMethodArgument {
                  ffi_index: arg_index as i32,
                  argument_type: complete_type,
                  name: if arg.meaning == CppFfiArgumentMeaning::This {
                    "self".to_string()
                  } else {
                    sanitize_rust_var_name(&arg.name)
                  },
                });
              }
            }
            Err(msg) => {
              log::warning(format!("Can't generate Rust method for method:\n{}\n{}\n",
                                   method.short_text(),
                                   msg));
              fail = true;
              break;
            }
          }
        }
        if return_type_info.is_none() {
          match self.cpp_type_to_complete_type(&method.c_signature.return_type) {
            Ok(r) => {
              return_type_info = Some((r, None));
            }
            Err(msg) => {
              log::warning(format!("Can't generate Rust method for method:\n{}\n{}\n",
                                   method.short_text(),
                                   msg));
              fail = true;
              break;
            }
          }
        } else {
          assert!(method.c_signature.return_type == CppFfiType::void());
        }
        if fail {
          continue;
        }
        let return_type_info1 = return_type_info.unwrap();
        r.push(RustMethod {
          name: sanitize_rust_var_name(&method.cpp_method.name.to_snake_case()),
          scope: scope.clone(),
          return_type: return_type_info1.0,
          return_type_ffi_index: return_type_info1.1,
          arguments: RustMethodArguments::SingleVariant(RustMethodArgumentsVariant {
            arguments: arguments,
            cpp_method: method.clone(),
          }),
        });
      } else {
        // TODO: generate overloaded functions
      }
    }
    return r;
  }

  pub fn generate_ffi(&mut self) {
    log::info("Generating Rust FFI functions.");
    let mut ffi_functions = HashMap::new();

    for header in &self.input_data.cpp_ffi_headers.clone() {
      let module_name = include_file_to_module_name(&header.include_file);
      let mut functions = Vec::new();
      for method in &header.methods {
        match self.generate_rust_ffi_function(method, &module_name) {
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
      ffi_functions.insert(header.include_file.clone(), functions);
    }
    self.code_generator.generate_ffi_file(&ffi_functions);
  }
}
