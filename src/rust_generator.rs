use cpp_ffi_generator::{CppAndFfiData, CppFfiHeaderData};
use cpp_and_ffi_method::CppAndFfiMethod;
use cpp_type::{CppType, CppTypeBase, CppBuiltInNumericType, CppTypeIndirection,
               CppSpecificNumericTypeKind};
use cpp_ffi_type::{CppFfiType, IndirectionChange};
use rust_type::{RustName, RustType, CompleteType, RustTypeIndirection, RustFFIFunction,
                RustFFIArgument, RustToCTypeConversion};
use cpp_data::{CppTypeKind, EnumValue, CppTypeData};
use std::path::PathBuf;
use std::collections::{HashMap, HashSet};
use log;
use rust_code_generator::RustCodeGenerator;
use rust_info::{RustTypeDeclaration, RustTypeDeclarationKind, RustTypeWrapperKind, RustModule,
                RustMethod, RustMethodScope, RustMethodArgument, RustMethodArgumentsVariant,
                RustMethodArguments, TraitImpl, TraitName, RustMethodSelfArg};
use cpp_method::{CppMethodScope, ReturnValueAllocationPlace};
use cpp_ffi_function_argument::CppFfiArgumentMeaning;
use utils::CaseOperations;

fn include_file_to_module_name(include_file: &String) -> String {
  let mut r = include_file.clone();
  if r.ends_with(".h") {
    r = r[0..r.len() - 2].to_string();
  }
  if r == "Qt" {
    r = "global".to_string();
  } else if r.starts_with("Qt") {
    r = r[2..].to_string();
  } else if r.starts_with("Q") {
    r = r[1..].to_string();
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

pub struct RustGenerator {
  input_data: CppAndFfiData,
  modules: Vec<RustModule>,
  crate_name: String,
  cpp_to_rust_type_map: HashMap<String, RustName>,
  code_generator: RustCodeGenerator,
}

impl RustGenerator {
  pub fn new(input_data: CppAndFfiData,
             output_path: PathBuf,
             template_path: PathBuf,
             c_lib_name: String,
             cpp_lib_name: String,
             c_lib_path: PathBuf)
             -> Self {
    let crate_name = "qt_core".to_string();
    RustGenerator {
      input_data: input_data,
      modules: Vec::new(),
      crate_name: crate_name.clone(),
      cpp_to_rust_type_map: HashMap::new(),
      code_generator: RustCodeGenerator::new(crate_name,
                                             output_path,
                                             template_path,
                                             c_lib_name,
                                             cpp_lib_name,
                                             c_lib_path),
    }
  }

  fn cpp_type_to_complete_type(&self,
                               cpp_ffi_type: &CppFfiType,
                               argument_meaning: &CppFfiArgumentMeaning)
                               -> Result<CompleteType, String> {
    let rust_ffi_type = try!(self.cpp_type_to_rust_ffi_type(cpp_ffi_type));
    let mut rust_api_type = rust_ffi_type.clone();
    let mut rust_api_to_c_conversion = RustToCTypeConversion::None;
    if let RustType::NonVoid { ref mut indirection, .. } = rust_api_type {
      match cpp_ffi_type.conversion.indirection_change {
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
        base: RustName::new(vec!["qt_core".to_string(), "flags".to_string(), "QFlags".to_string()]),
        generic_arguments: Some(vec![RustType::NonVoid {
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


  fn cpp_type_to_rust_ffi_type(&self, cpp_ffi_type: &CppFfiType) -> Result<RustType, String> {
    let rust_name = match cpp_ffi_type.ffi_type.base {
      CppTypeBase::Void => {
        match cpp_ffi_type.ffi_type.indirection {
          CppTypeIndirection::None => return Ok(RustType::Void),
          _ => RustName::new(vec!["libc".to_string(), "c_void".to_string()]),
        }
      }
      CppTypeBase::BuiltInNumeric(ref numeric) => {
        if numeric == &CppBuiltInNumericType::Bool {
          RustName::new(vec!["bool".to_string()])
        } else {
          let own_name = match *numeric {
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
          };
          RustName::new(vec!["libc".to_string(), own_name.to_string()])
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
        RustName::new(vec![format!("{}{}", letter, bits)])
      }
      CppTypeBase::PointerSizedInteger { ref is_signed, .. } => {
        RustName::new(vec![if *is_signed {
                               "isize"
                             } else {
                               "usize"
                             }
                             .to_string()])
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
        CppTypeIndirection::PtrPtr => RustTypeIndirection::PtrPtr,
        _ => return Err(format!("unsupported level of indirection: {:?}", cpp_ffi_type)),
      },
      generic_arguments: None,
    });
  }


  fn generate_rust_ffi_function(&self, data: &CppAndFfiMethod) -> Result<RustFFIFunction, String> {
    let mut args = Vec::new();
    for arg in &data.c_signature.arguments {
      let rust_type = try!(self.cpp_type_to_complete_type(&arg.argument_type, &arg.meaning))
        .rust_ffi_type;
      args.push(RustFFIArgument {
        name: sanitize_rust_var_name(&arg.name),
        argument_type: rust_type,
      });
    }
    Ok(RustFFIFunction {
      return_type: try!(self.cpp_type_to_complete_type(&data.c_signature.return_type,
                                                       &CppFfiArgumentMeaning::ReturnValue))
        .rust_ffi_type,
      name: data.c_name.clone(),
      arguments: args,
    })
  }




  fn generate_type_map(&mut self) {

    fn add_one_to_type_map(crate_name: &String,
                           map: &mut HashMap<String, RustName>,
                           name: &String,
                           include_file: &String,
                           is_function: bool) {
      let mut split_parts: Vec<_> = name.split("::").collect();
      let last_part = split_parts.pop().unwrap().to_string();
      let last_part_final = if is_function {
        last_part.to_snake_case()
      } else {
        last_part.to_class_case()
      };

      let mut parts = Vec::new();
      parts.push(crate_name.clone());
      parts.push(include_file_to_module_name(&include_file));
      for part in split_parts {
        parts.push(part.to_string().to_snake_case());
      }

      if parts.len() > 2 && parts[1] == parts[2] {
        // special case
        parts.remove(2);
      }
      parts.push(last_part_final);

      // TODO: this is Qt-specific
      for part in &mut parts {
        if part.starts_with("q_") {
          *part = part[2..].to_string();
        } else if part.starts_with("Q") {
          *part = part[1..].to_string();
        }
      }

      map.insert(name.clone(), RustName::new(parts));
    }
    for type_info in &self.input_data.cpp_data.types {
      if let CppTypeKind::Class { size, .. } = type_info.kind {
        if size.is_none() {
          log::warning(format!("Rust type is not generated for a struct with unknown \
                                        size: {}",
                               type_info.name));
          continue;
        }
      }

      add_one_to_type_map(&self.crate_name,
                          &mut self.cpp_to_rust_type_map,
                          &type_info.name,
                          &type_info.include_file,
                          false);
    }
    for header in &self.input_data.cpp_ffi_headers {
      for method in &header.methods {
        if method.cpp_method.scope == CppMethodScope::Global {
          add_one_to_type_map(&self.crate_name,
                              &mut self.cpp_to_rust_type_map,
                              &method.cpp_method.name,
                              &header.include_file,
                              true);
        }
      }
    }
  }

  fn process_type(&self,
                  type_info: &CppTypeData,
                  c_header: &CppFfiHeaderData)
                  -> Vec<RustTypeDeclaration> {
    let rust_name = self.cpp_to_rust_type_map.get(&type_info.name).unwrap();
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
                                      name: variant.name.to_class_case(),
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
          .map(|(_val, variant)| variant)
          .collect();
        values.sort_by(|a, b| a.value.cmp(&b.value));
        let mut is_flaggable = false;
        if let Some(instantiations) = self.input_data
          .cpp_data
          .template_instantiations
          .get(&"QFlags".to_string()) {
          let cpp_type_sample = CppType {
            is_const: false,
            indirection: CppTypeIndirection::None,
            base: CppTypeBase::Enum { name: type_info.name.clone() },
          };
          if instantiations.iter().find(|x| x.len() == 1 && &x[0] == &cpp_type_sample).is_some() {
            is_flaggable = true;
          }
        }
        vec![RustTypeDeclaration {
               name: rust_name.clone(),
               kind: RustTypeDeclarationKind::CppTypeWrapper {
                 kind: RustTypeWrapperKind::Enum {
                   values: values,
                   is_flaggable: is_flaggable,
                 },
                 cpp_type_name: type_info.name.clone(),
                 cpp_template_arguments: None,
                 methods: Vec::new(),
                 traits: Vec::new(),
               },
             }]
      }
      CppTypeKind::Class { ref size, .. } => {
        let methods_scope = RustMethodScope::Impl { type_name: rust_name.clone() };
        let (methods, traits, types) = self.generate_functions(c_header.methods
                                                                 .iter()
                                                                 .filter(|&x| {
                                                                   x.cpp_method
                                                                     .scope
                                                                     .class_name() ==
                                                                   Some(&type_info.name)
                                                                 })
                                                                 .collect(),
                                                               &methods_scope);
        let mut result = types;
        result.push(RustTypeDeclaration {
          name: rust_name.clone(),
          kind: RustTypeDeclarationKind::CppTypeWrapper {
            kind: RustTypeWrapperKind::Struct { size: size.unwrap() },
            cpp_type_name: type_info.name.clone(),
            cpp_template_arguments: None,
            methods: methods,
            traits: traits,
          },
        });
        result
      }
    }
  }

  pub fn generate_all(&mut self) {
    self.code_generator.generate_template();
    self.generate_type_map();
    for header in &self.input_data.cpp_ffi_headers.clone() {
      self.generate_modules_from_header(header);
    }
    self.generate_ffi();
    self.code_generator.generate_lib_file(&self.modules
      .iter()
      .map(|x| x.name.last_name().clone())
      .collect());
  }

  pub fn generate_modules_from_header(&mut self, c_header: &CppFfiHeaderData) {
    let module_name = include_file_to_module_name(&c_header.include_file);
    if module_name == "flags" && self.crate_name == "qt_core" {
      log::info(format!("Skipping module {}::{}", self.crate_name, module_name));
      return;
    }
    let module_name1 = RustName::new(vec![self.crate_name.clone(), module_name]);
    if let Some(module) = self.generate_module(c_header, &module_name1) {
      self.code_generator.generate_module_file(&module);
      self.modules.push(module);
    }
  }

  pub fn generate_module(&mut self,
                         c_header: &CppFfiHeaderData,
                         module_name: &RustName)
                         -> Option<RustModule> {
    log::info(format!("Generating Rust module {}", module_name.full_name(None)));

    let mut direct_submodules = HashSet::new();
    let mut rust_types = Vec::new();
    let mut good_methods = Vec::new();
    {
      let mut check_name = |name| {
        if let Some(rust_name) = self.cpp_to_rust_type_map.get(name) {
          let extra_modules_count = rust_name.parts.len() - module_name.parts.len();
          if extra_modules_count > 0 {
            if rust_name.parts[0..module_name.parts.len()] != module_name.parts[..] {
              return false; // not in this module
            }
          }
          if extra_modules_count == 2 {
            let direct_submodule = &rust_name.parts[module_name.parts.len()];
            if !direct_submodules.contains(direct_submodule) {
              direct_submodules.insert(direct_submodule.clone());
            }
          }
          if extra_modules_count == 1 {
            return true;
          }
          // this type is in nested submodule
        }
        false
      };
      for type_data in &self.input_data.cpp_data.types {
        if check_name(&type_data.name) {
          rust_types.append(&mut self.process_type(type_data, c_header));
        }
      }
      for method in &c_header.methods {
        if method.cpp_method.scope == CppMethodScope::Global {
          if check_name(&method.cpp_method.name) {
            good_methods.push(method);
          }
        }
      }
    }
    let mut submodules = Vec::new();
    for name in direct_submodules {
      let mut new_name = module_name.clone();
      new_name.parts.push(name);
      if let Some(m) = self.generate_module(c_header, &new_name) {
        submodules.push(m);
      }
    }

    let (free_methods, free_traits, mut free_types) =
      self.generate_functions(good_methods, &RustMethodScope::Free);
    assert!(free_traits.is_empty());
    rust_types.append(&mut free_types);

    let module = RustModule {
      name: module_name.clone(),
      types: rust_types,
      functions: free_methods,
      submodules: submodules,
    };
    return Some(module);
  }

  fn generate_function(&self,
                       method: &CppAndFfiMethod,
                       scope: &RustMethodScope,
                       use_args_caption: bool)
                       -> Result<RustMethod, String> {
    if method.cpp_method.kind.is_operator() {
      // TODO: implement operator traits
      return Err(format!("operators are not supported yet"));
    }
    let mut arguments = Vec::new();
    let mut return_type_info = None;
    for (arg_index, arg) in method.c_signature.arguments.iter().enumerate() {
      match self.cpp_type_to_complete_type(&arg.argument_type, &arg.meaning) {
        Ok(mut complete_type) => {
          if arg.meaning == CppFfiArgumentMeaning::ReturnValue {
            assert!(return_type_info.is_none());
            return_type_info = Some((complete_type, Some(arg_index as i32)));
          } else {
            if method.allocation_place == ReturnValueAllocationPlace::Heap &&
               method.cpp_method.kind.is_destructor() {
              if let RustType::NonVoid { ref mut indirection, .. } = complete_type.rust_api_type {
                assert!(*indirection == RustTypeIndirection::Ref { lifetime: None });
                *indirection = RustTypeIndirection::None;
              } else {
                panic!("unexpected void type");
              }
              assert!(complete_type.rust_api_to_c_conversion == RustToCTypeConversion::RefToPtr);
              complete_type.rust_api_to_c_conversion = RustToCTypeConversion::ValueToPtr;
            }

            arguments.push(RustMethodArgument {
              ffi_index: arg_index as i32,
              argument_type: complete_type,
              name: if arg.meaning == CppFfiArgumentMeaning::This {
                "self".to_string()
              } else {
                sanitize_rust_var_name(&arg.name.to_snake_case())
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
      match self.cpp_type_to_complete_type(&method.c_signature.return_type,
                                           &CppFfiArgumentMeaning::ReturnValue) {
        Ok(mut r) => {
          if method.allocation_place == ReturnValueAllocationPlace::Heap &&
             !method.cpp_method.kind.is_destructor() {
            if let RustType::NonVoid { ref mut indirection, .. } = r.rust_api_type {
              assert!(*indirection == RustTypeIndirection::None);
              *indirection = RustTypeIndirection::Ptr;
            } else {
              panic!("unexpected void type");
            }
            assert!(r.cpp_type.indirection == CppTypeIndirection::None);
            assert!(r.cpp_to_ffi_conversion.indirection_change ==
                    IndirectionChange::ValueToPointer);
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
      assert!(method.c_signature.return_type == CppFfiType::void());
    }
    let return_type_info1 = return_type_info.unwrap();

    Ok(RustMethod {
      name: self.method_rust_name(method, use_args_caption),
      scope: scope.clone(),
      return_type: return_type_info1.0,
      arguments: RustMethodArguments::SingleVariant(RustMethodArgumentsVariant {
        arguments: arguments,
        cpp_method: method.clone(),
        return_type_ffi_index: return_type_info1.1,
      }),
    })
  }

  fn method_rust_name(&self, method: &CppAndFfiMethod, use_args_caption: bool) -> RustName {
    let mut name = if method.cpp_method.scope == CppMethodScope::Global {
      self.cpp_to_rust_type_map.get(&method.cpp_method.name).unwrap().clone()
    } else {
      let x = if method.cpp_method.kind.is_constructor() {
        "new".to_string()
      } else if method.cpp_method.kind.is_destructor() {
        "delete".to_string()
      } else {
        method.cpp_method.name.to_snake_case()
      };
      RustName::new(vec![x])
    };
    if use_args_caption {
      if let Some(ref args_caption) = method.args_caption {
        if !args_caption.is_empty() {
          let x = name.parts.pop().unwrap();
          name.parts.push(format!("{}_args_{}", x, args_caption.to_snake_case()));
        }
      } else {
        panic!("unexpected lack of args_caption: {:?}", method);

      }
    }
    match method.allocation_place {
      ReturnValueAllocationPlace::Heap => {
        let x = name.parts.pop().unwrap();
        name.parts.push(format!("{}_as_ptr", x));
      }
      ReturnValueAllocationPlace::Stack |
      ReturnValueAllocationPlace::NotApplicable => {}
    }
    let sanitized = sanitize_rust_var_name(name.last_name());
    if &sanitized != name.last_name() {
      name.parts.pop().unwrap();
      name.parts.push(sanitized);
    }
    name
  }

  fn generate_functions(&self,
                        methods: Vec<&CppAndFfiMethod>,
                        scope: &RustMethodScope)
                        -> (Vec<RustMethod>, Vec<TraitImpl>, Vec<RustTypeDeclaration>) {
    let mut single_rust_methods = Vec::new();
    let mut rust_methods = Vec::new();
    let mut traits = Vec::new();
    let mut types = Vec::new();
    let mut method_names = HashSet::new();
    for method in &methods {
      if method.cpp_method.kind.is_destructor() {
        if let &RustMethodScope::Impl { ref type_name } = scope {
          if method.allocation_place == ReturnValueAllocationPlace::Stack {
            match self.generate_function(method, scope, false) {
              Ok(mut method) => {
                method.name = RustName::new(vec!["drop".to_string()]);
                method.scope = RustMethodScope::TraitImpl {
                  type_name: type_name.clone(),
                  trait_name: TraitName::Drop,
                };
                traits.push(TraitImpl {
                  target_type: type_name.clone(),
                  trait_name: TraitName::Drop,
                  methods: vec![method],
                });
              }
              Err(msg) => {
                log::warning(format!("Failed to generate destructor: {}\n{:?}\n", msg, method))
              }
            }
            continue;
          }
        } else {
          panic!("destructor must be in class scope");
        }
      }

      match self.generate_function(method, scope, false) {
        Ok(rust_method) => {
          if !method_names.contains(rust_method.name.last_name()) {
            method_names.insert(rust_method.name.last_name().clone());
          }
          single_rust_methods.push(rust_method);
        }
        Err(msg) => log::warning(msg),
      }
    }
    // let mut name_counters = HashMap::new();
    for method_name in method_names {
      let current_methods: Vec<_> = single_rust_methods.clone()
        .into_iter()
        .filter(|m| m.name.last_name() == &method_name)
        .collect();
      let mut return_type_to_methods = HashMap::new();
      assert!(!current_methods.is_empty());
      for method in current_methods {
        let t = (method.return_type.rust_api_type.clone(), method.self_arg());
        if !return_type_to_methods.contains_key(&t) {
          return_type_to_methods.insert(t.clone(), Vec::new());
        }
        return_type_to_methods.get_mut(&t).unwrap().push(method);
      }
      let use_additional_caption = return_type_to_methods.len() > 1;
      #[derive(Clone)]
      enum CaptionStrategy {
        ReturnType,
        SelfArg,
        Both,
        None,
      }
      let generate_caption = |a: &RustType, b: &RustMethodSelfArg, s| {
        match s {
          CaptionStrategy::ReturnType => a.caption(),
          CaptionStrategy::SelfArg => b.caption().to_string(),
          CaptionStrategy::Both => format!("{}_{}", a.caption(), b.caption()),
          CaptionStrategy::None => String::new(),
        }
      };
      let caption_strategy = if use_additional_caption {
        let mut maybe_caption_strategy = None;
        for s in vec![CaptionStrategy::ReturnType,
                      CaptionStrategy::SelfArg,
                      CaptionStrategy::Both] {
          let mut set = HashSet::new();
          let mut ok = true;
          for (&(ref return_type, ref self_arg), _methods) in &return_type_to_methods {
            let caption = generate_caption(return_type, self_arg, s.clone());
            if set.contains(&caption) {
              ok = false;
              break;
            }
            set.insert(caption);
          }
          if ok {
            maybe_caption_strategy = Some(s);
            break;
          }
        }
        if maybe_caption_strategy.is_none() {
          println!("failed on methods: {:?}", return_type_to_methods);
          panic!("all caption strategies have failed!");
        }
        maybe_caption_strategy.unwrap()
      } else {
        CaptionStrategy::None // unused
      };

      for ((return_type, self_arg), overloaded_methods) in return_type_to_methods {
        let additional_caption =
          generate_caption(&return_type, &self_arg, caption_strategy.clone());

        let mut enum_name_base = method_name.to_class_case();
        if let &RustMethodScope::Impl { ref type_name } = scope {
          enum_name_base = format!("{}{}", type_name.last_name(), enum_name_base);
        }
        if use_additional_caption {
          enum_name_base = format!("{}As{}", enum_name_base, additional_caption.to_class_case());
        }
        assert!(!overloaded_methods.is_empty());

        let mut all_real_args = HashSet::new();
        let mut filtered_methods = Vec::new();
        for method in overloaded_methods {
          let ok = if let RustMethodArguments::SingleVariant(ref args) = method.arguments {
            let real_args: Vec<_> =
              args.arguments.iter().map(|x| x.argument_type.rust_api_type.dealias_libc()).collect();
            if all_real_args.contains(&real_args) {
              log::warning(format!("Removing method because another method with the same \
                                    argument types exists:\n{:?}",
                                   method));
              false
            } else {
              all_real_args.insert(real_args);
              true
            }
          } else {
            unreachable!()
          };
          if ok {
            filtered_methods.push(method);
          }
        }

        let methods_count = filtered_methods.len();
        let mut method = if methods_count > 1 {
          let enum_name = RustName::new(vec![format!("{}ParamsVariants", enum_name_base)]);
          let trait_name = RustName::new(vec![format!("{}Params", enum_name_base)]);
          let first_method = filtered_methods[0].clone();
          let self_argument = if let RustMethodArguments::SingleVariant(ref args) =
                                     first_method.arguments {
            if args.arguments.len() > 0 && args.arguments[0].name == "self" {
              Some(args.arguments[0].clone())
            } else {
              None
            }
          } else {
            unreachable!()
          };
          let mut args_variants = Vec::new();
          let mut enum_variants = Vec::new();
          for method in filtered_methods {
            assert!(method.name == first_method.name);
            assert!(method.scope == first_method.scope);
            assert!(method.return_type == first_method.return_type);
            let dbg_method = method.clone();
            if let RustMethodArguments::SingleVariant(mut args) = method.arguments {
              if let Some(ref self_argument) = self_argument {
                if !(args.arguments.len() > 0 && &args.arguments[0] == self_argument) {
                  println!("FAIL! TEST1: {:?}", &first_method);
                  println!("TEST2: {:?}", &dbg_method);
                }
                assert!(args.arguments.len() > 0 && &args.arguments[0] == self_argument);
                args.arguments.remove(0);
              }
              enum_variants.push(args.arguments
                .iter()
                .map(|x| x.argument_type.rust_api_type.clone())
                .collect());
              args_variants.push(args);
            } else {
              unreachable!()
            }
          }

          // overloaded methods
          types.push(RustTypeDeclaration {
            name: enum_name.clone(),
            kind: RustTypeDeclarationKind::MethodParametersEnum {
              variants: enum_variants,
              trait_name: trait_name.clone(),
            },
          });
          types.push(RustTypeDeclaration {
            name: trait_name.clone(),
            kind: RustTypeDeclarationKind::MethodParametersTrait { enum_name: enum_name.clone() },
          });
          RustMethod {
            name: first_method.name,
            scope: first_method.scope,
            return_type: first_method.return_type,
            arguments: RustMethodArguments::MultipleVariants {
              params_enum_name: enum_name.last_name().clone(),
              params_trait_name: trait_name.last_name().clone(),
              shared_arguments: match self_argument {
                None => Vec::new(),
                Some(arg) => vec![arg],
              },
              variant_argument_name: "params".to_string(),
              variants: args_variants,
            },
          }
        } else {
          filtered_methods.pop().unwrap()
        };
        if use_additional_caption {
          let name = method.name.parts.pop().unwrap();
          method.name.parts.push(format!("{}_as_{}", name, additional_caption.to_snake_case()));
        }
        rust_methods.push(method);
      }
    }
    return (rust_methods, traits, types);
  }

  pub fn generate_ffi(&mut self) {
    log::info("Generating Rust FFI functions.");
    let mut ffi_functions = HashMap::new();

    for header in &self.input_data.cpp_ffi_headers.clone() {
      let mut functions = Vec::new();
      for method in &header.methods {
        match self.generate_rust_ffi_function(method) {
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
