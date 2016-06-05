use cpp_ffi_generator::{CppAndFfiData, CppFfiHeaderData};
use cpp_and_ffi_method::CppAndFfiMethod;
use cpp_type::{CppTypeBase, CppBuiltInNumericType, CppTypeIndirection, CppSpecificNumericTypeKind};
use cpp_ffi_type::CppFfiType;
use utils::JoinWithString;
use rust_type::{RustName, RustType, CompleteType, RustTypeIndirection, RustFFIFunction,
                RustFFIArgument};
use cpp_data::{CppTypeKind, EnumValue};
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use std::collections::{HashMap, HashSet};
use log;

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
  modules: Vec<String>,
  crate_name: String,
  cpp_to_rust_type_map: HashMap<String, RustName>,
  processed_cpp_types: HashSet<String>,
}

impl RustGenerator {
  pub fn new(input_data: CppAndFfiData, output_path: PathBuf) -> Self {
    RustGenerator {
      input_data: input_data,
      output_path: output_path,
      modules: Vec::new(),
      crate_name: "qt_core".to_string(),
      cpp_to_rust_type_map: HashMap::new(),
      processed_cpp_types: HashSet::new(),
    }
  }

  fn cpp_type_to_complete_type(&self, cpp_ffi_type: &CppFfiType) -> Result<CompleteType, String> {
    Ok(CompleteType {
      cpp_ffi_type: cpp_ffi_type.ffi_type.clone(),
      cpp_type: cpp_ffi_type.original_type.clone(),
      cpp_to_ffi_conversion: cpp_ffi_type.conversion.clone(),
      rust_ffi_type: try!(self.cpp_type_to_rust_ffi_type(cpp_ffi_type)), /* rust_api_type: rust_api_type,
                                                                          * rust_api_to_c_conversion: rust_api_to_c_conversion, */
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

  fn rust_type_to_code(&self, rust_type: &RustType) -> String {
    match rust_type {
      &RustType::Void => panic!("rust void can't be converted to code"),
      &RustType::NonVoid { ref base, ref is_const, ref indirection, ref is_option } => {
        let base_s = base.full_name(&self.crate_name);
        let s = match indirection {
          &RustTypeIndirection::None => base_s,
          &RustTypeIndirection::Ref => {
            if *is_const {
              format!("&{}", base_s)
            } else {
              format!("&mut {}", base_s)
            }
          }
          &RustTypeIndirection::Ptr => {
            if *is_const {
              format!("*const {}", base_s)
            } else {
              format!("*mut {}", base_s)
            }
          }
        };
        if *is_option {
          format!("Option<{}>", s)
        } else {
          s
        }
      }
    }
  }

  fn rust_ffi_function_to_code(&self, func: &RustFFIFunction) -> String {
    let args = func.arguments.iter().map(|arg| {
      format!("{}: {}",
              arg.name,
              self.rust_type_to_code(&arg.argument_type))
    });
    format!("  pub fn {}({}){};\n",
            func.name.own_name,
            args.join(", "),
            match func.return_type {
              RustType::Void => String::new(),
              RustType::NonVoid { .. } => {
                format!(" -> {}", self.rust_type_to_code(&func.return_type))
              }
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
      self.cpp_to_rust_type_map.insert(type_info.name.clone(),
                                       RustName {
                                         crate_name: self.crate_name.clone(),
                                         module_name:
                                           include_file_to_module_name(&type_info.include_file),
                                         own_name: new_name,
                                       });
    }
  }

  pub fn generate_all(&mut self) {
    self.generate_type_map();
    for header in &self.input_data.cpp_ffi_headers.clone() {
      self.generate_types(header);
    }
    self.generate_ffi();

    let mut lib_file_path = self.output_path.clone();
    lib_file_path.push("qt_core");
    lib_file_path.push("src");
    lib_file_path.push("lib.rs");
    let mut lib_file = File::create(&lib_file_path).unwrap();
    write!(lib_file, "pub mod types;\n\n").unwrap();
    write!(lib_file, "pub mod flags;\n\n").unwrap();
    write!(lib_file, "pub mod extra;\n\n").unwrap();
    // TODO: remove allow directive
    // TODO: ffi should be a private mod
    write!(lib_file, "#[allow(dead_code)]\npub mod ffi;\n\n").unwrap();
    for module in &self.modules {
      write!(lib_file, "pub mod {};\n", module).unwrap();
    }
  }

  pub fn generate_types(&mut self, c_header: &CppFfiHeaderData) {
    let module_name = include_file_to_module_name(&c_header.include_file);
    if module_name == "flags" && self.crate_name == "qt_core" {
      log::info(format!("Skipping module {}::{}", self.crate_name, module_name));
      return;
    }
    log::info(format!("Generating Rust types in module {}::{}",
                      self.crate_name,
                      module_name));
    let mut file_path = self.output_path.clone();
    file_path.push("qt_core");
    file_path.push("src");
    file_path.push(format!("{}.rs", module_name));
    let mut file = File::create(&file_path).unwrap();

    for type_data in &self.input_data
                          .cpp_data_by_headers
                          .get(&c_header.include_file)
                          .unwrap()
                          .types {
      if let Some(rust_type_name) = self.cpp_to_rust_type_map.get(&type_data.name) {
        if module_name == rust_type_name.module_name {
          let code = match type_data.kind {
            CppTypeKind::Enum { ref values } => {
              let mut value_to_variant: HashMap<i64, EnumValue> = HashMap::new();
              for variant in values {
                let value = variant.value;
                if value_to_variant.contains_key(&value) {
                  log::warning(format!("warning: {}: duplicated enum variant removed: {} \
                                      (previous variant: {})",
                                       type_data.name,
                                       variant.name,
                                       value_to_variant.get(&value).unwrap().name));
                } else {
                  value_to_variant.insert(value, variant.clone());
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
              format!("#[repr(C)]\npub enum {} {{\n{}\n}}\n\n",
                      rust_type_name.own_name,
                      value_to_variant.iter()
                                      .map(|(value, variant)| {
                                        format!("  {} = {}", variant.name.to_class_case1(), value)
                                      })
                                      .join(", \n"))
            }
            CppTypeKind::Class { ref size, .. } => {
              match *size {
                Some(ref size) => {
                  format!("#[repr(C)]\npub struct {} {{\n  _buffer: [u8; {}],\n}}\n\n",
                          rust_type_name.own_name,
                          size)
                }
                None => format!("pub enum {} {{}}\n\n", rust_type_name.own_name),

              }
            }
          };
          file.write(code.as_bytes()).unwrap();
          self.processed_cpp_types.insert(type_data.name.clone());
        } else {
          panic!("unexpected module name mismatch: {}, {:?}",
                 module_name,
                 rust_type_name);
        }
      } else {
        // type is skipped: no rust name
      }
    }
    self.modules.push(module_name);
  }

  pub fn generate_ffi(&mut self) {
    log::info("Generating Rust FFI functions.");
    let mut file_path = self.output_path.clone();
    file_path.push("qt_core");
    file_path.push("src");
    file_path.push("ffi.rs");
    let mut file = File::create(&file_path).unwrap();
    write!(file, "extern crate libc;\n\n").unwrap();
    write!(file, "#[link(name = \"Qt5Core\")]\n").unwrap();
    write!(file, "#[link(name = \"icui18n\")]\n").unwrap();
    write!(file, "#[link(name = \"icuuc\")]\n").unwrap();
    write!(file, "#[link(name = \"icudata\")]\n").unwrap();
    write!(file, "#[link(name = \"stdc++\")]\n").unwrap();
    write!(file, "#[link(name = \"qtcw\", kind = \"static\")]\n").unwrap();
    write!(file, "extern \"C\" {{\n").unwrap();

    for header in &self.input_data.cpp_ffi_headers.clone() {
      write!(file, "  // Header: {}\n", header.include_file).unwrap();
      let module_name = include_file_to_module_name(&header.include_file);
      for method in &header.methods {
        match self.generate_rust_ffi_function(method, &module_name) {
          Ok(function) => {
            file.write(self.rust_ffi_function_to_code(&function).as_bytes()).unwrap();
          }
          Err(msg) => {
            log::warning(format!("Can't generate Rust FFI function for method:\n{}\n{}\n",
                                 method.short_text(),
                                 msg));
          }
        }
      }
      write!(file, "\n").unwrap();
    }

    write!(file, "}}\n").unwrap();
  }
}
