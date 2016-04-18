use c_generator::{CppAndCData, CHeaderData};
use cpp_and_c_method::CppAndCMethod;
use cpp_type_map::EnumValue;
use enums::{CppTypeKind, CppTypeOrigin};
use utils::JoinWithString;
use rust_type::{RustName, RustType, CompleteType, RustTypeIndirection, RustFFIFunction,
                RustFFIArgument};
use c_type::CTypeExtended;

use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use std::collections::{HashMap, HashSet};
use log;

fn include_file_to_module_name(include_file: &String) -> String {
  let include_without_prefix = if include_file == "Qt" {
    "qt".to_string()
  } else if include_file.starts_with("Qt") {
    include_file[2..].to_string()
  } else if include_file.starts_with("Q") {
    include_file[1..].to_string()
  } else {
    include_file.clone()
  };
  include_without_prefix.to_snake_case()
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
    "typeof" | "unsafe" | "unsized" | "use" | "virtual" | "where" | "while"
    | "yield" => format!("{}_", name),
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
  input_data: CppAndCData,
  output_path: PathBuf,
  modules: Vec<String>,
  crate_name: String,
  cpp_to_rust_type_map: HashMap<String, RustName>,
  processed_cpp_types: HashSet<String>,
}

impl RustGenerator {
  pub fn new(input_data: CppAndCData, output_path: PathBuf) -> Self {
    RustGenerator {
      input_data: input_data,
      output_path: output_path,
      modules: Vec::new(),
      crate_name: "qt_core".to_string(),
      cpp_to_rust_type_map: HashMap::new(),
      processed_cpp_types: HashSet::new(),
    }
  }

  fn c_type_to_complete_type(&self, c_type_ex: &CTypeExtended) -> Result<CompleteType, String> {
    if !self.cpp_to_rust_type_map.contains_key(&c_type_ex.cpp_type.base) {
      return Err(format!("Type has no Rust equivalent: {}", c_type_ex.cpp_type.base));
    }
    if !self.is_cpp_type_processed(&c_type_ex.cpp_type.base) {
      return Err(format!("Type is not processed: {}", c_type_ex.cpp_type.base));
    }
    let rust_name = self.cpp_to_rust_type_map.get(&c_type_ex.cpp_type.base).unwrap();

    let rust_ffi_type = if c_type_ex.c_type.base == "void" {
      if c_type_ex.c_type.is_pointer {
        RustType::NonVoid {
          base: rust_name.clone(),
          is_const: c_type_ex.c_type.is_const,
          indirection: RustTypeIndirection::Ptr,
          is_option: false,
        }
      } else {
        RustType::Void
      }
    } else {
      if c_type_ex.conversion.qflags_to_uint {
        if c_type_ex.c_type.is_const || c_type_ex.c_type.is_pointer {
          panic!("unsupported const or pointer in flags type");
        }
        RustType::NonVoid {
          base: self.cpp_to_rust_type_map.get(&"unsigned int".to_string()).unwrap().clone(),
          is_const: false,
          indirection: RustTypeIndirection::None,
          is_option: false,
        }
      } else {
        RustType::NonVoid {
          base: rust_name.clone(),
          is_const: c_type_ex.c_type.is_const,
          indirection: if c_type_ex.c_type.is_pointer {
            RustTypeIndirection::Ptr
          } else {
            RustTypeIndirection::None
          },
          is_option: false,
        }
      }
    };

    Ok(CompleteType {
      c_type: c_type_ex.c_type.clone(),
      cpp_type: c_type_ex.cpp_type.clone(),
      cpp_to_c_conversion: c_type_ex.conversion.clone(),
      rust_ffi_type: rust_ffi_type, /* rust_api_type: rust_api_type,
                                     * rust_api_to_c_conversion: rust_api_to_c_conversion, */
    })

  }


  fn generate_rust_ffi_function(&self,
                                data: &CppAndCMethod,
                                module_name: &String)
                                -> Result<RustFFIFunction, String> {
    let mut args = Vec::new();
    for arg in &data.c_signature.arguments {
      let rust_type = try!(self.c_type_to_complete_type(&arg.argument_type)).rust_ffi_type;
      args.push(RustFFIArgument {
        name: sanitize_rust_var_name(&arg.name),
        argument_type: rust_type,
      });
    }
    Ok(RustFFIFunction {
      return_type: try!(self.c_type_to_complete_type(&data.c_signature.return_type)).rust_ffi_type,
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

  fn is_cpp_type_processed(&self, cpp_type: &String) -> bool {
    if let Some(rust_name) = self.cpp_to_rust_type_map.get(cpp_type) {
      if rust_name.crate_name == "qt_core" && rust_name.module_name == "types" {
        true
      } else if rust_name.crate_name == "" && rust_name.module_name == "" {
        true
      } else {
        self.processed_cpp_types.contains(cpp_type)
      }
    } else {
      false
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
    for (cpp_name, type_info) in &self.input_data.cpp_data.types.0 {
      let rust_type_name = {
        let primitive_type_name = match cpp_name.as_ref() {
          "qint8" => "i8",
          "quint8" => "u8",
          "qint16" => "i16",
          "quint16" => "u16",
          "qint32" => "i32",
          "quint32" => "u32",
          "qint64" => "i64",
          "quint64" => "u64",
          "qlonglong" => "i64",
          "qulonglong" => "u64",
          "qintptr" | "qptrdiff" | "QList_difference_type" => "isize",
          "quintptr" => "usize",
          "float" => "f32",
          "double" => "f64",
          "bool" => "bool",
          _ => "",
        };
        if !primitive_type_name.is_empty() {
          Ok(RustName {
            crate_name: String::new(),
            module_name: String::new(),
            own_name: primitive_type_name.to_string(),
          })
        } else {
          let type_name = match cpp_name.as_ref() {
            "qreal" => "qreal",
            "char" => "c_char",
            "signed char" => "c_schar",
            "unsigned char" => "c_uchar",
            "short" => "c_short",
            "unsigned short" => "c_ushort",
            "int" => "c_int",
            "unsigned int" => "c_uint",
            "long" => "c_long",
            "unsigned long" => "c_ulong",
            "long long" => "c_longlong",
            "unsigned long long" => "c_ulonglong",
            "wchar_t" => "wchar_t",
            "size_t" => "size_t",
            "void" => "c_void",
            _ => "",
          };
          if !type_name.is_empty() {
            Ok(RustName {
              crate_name: "qt_core".to_string(),
              module_name: "types".to_string(),
              own_name: type_name.to_string(),
            })
          } else {
            if let CppTypeOrigin::Qt { ref include_file } = type_info.origin {
              let eliminated_name_prefix = format!("{}::", include_file);
              let mut new_name = cpp_name.clone();
              if new_name.starts_with(&eliminated_name_prefix) {
                new_name = new_name[eliminated_name_prefix.len()..].to_string();
              }
              new_name = new_name.replace("::", "_").to_class_case1();
              Ok(RustName {
                crate_name: self.crate_name.clone(),
                module_name: include_file_to_module_name(include_file),
                own_name: new_name,
              })
            } else {
              log::warning(format!("warning: type is skipped: {:?}", type_info));
              Err(())
            }
          }
        }
      };
      if let Ok(rust_type_name) = rust_type_name {
        self.cpp_to_rust_type_map.insert(cpp_name.clone(), rust_type_name);
      }
    }
    self.cpp_to_rust_type_map.insert("QFlags".to_string(),
                                     RustName {
                                       crate_name: "qt_core".to_string(),
                                       module_name: "flags".to_string(),
                                       own_name: "QFlags".to_string(),
                                     });
  }

  pub fn generate_all(&mut self) {
    self.generate_type_map();
    for header in &self.input_data.c_headers.clone() {
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
    // TODO: remove allow directive
    // TODO: ffi should be a private mod
    write!(lib_file, "#[allow(dead_code)]\npub mod ffi;\n\n").unwrap();
    for module in &self.modules {
      write!(lib_file, "pub mod {};\n", module).unwrap();
    }

    //    let mut ffi_lib_file_path = self.output_path.clone();
    //    ffi_lib_file_path.push("qt_core");
    //    ffi_lib_file_path.push("src");
    //    ffi_lib_file_path.push("ffi");
    //    ffi_lib_file_path.push("mod.rs");
    //    let mut ffi_lib_file = File::create(&ffi_lib_file_path).unwrap();
    //    for module in &self.modules {
    //      write!(ffi_lib_file, "pub mod {};\n", module).unwrap();
    //    }

  }

  pub fn generate_types(&mut self, c_header: &CHeaderData) {
    let module_name = include_file_to_module_name(&c_header.include_file);
    log::info(format!("Generating Rust types in module {}::{}",
                      self.crate_name,
                      module_name));
    let mut file_path = self.output_path.clone();
    file_path.push("qt_core");
    file_path.push("src");
    file_path.push(format!("{}.rs", module_name));
    let mut file = File::create(&file_path).unwrap();

    for type_name in self.input_data
                         .cpp_data
                         .types
                         .get_types_from_include_file(&c_header.include_file) {
      let item = self.input_data.cpp_data.types.0.get(&type_name).unwrap();
      if let Some(rust_type_name) = self.cpp_to_rust_type_map.get(&type_name) {
        if module_name == rust_type_name.module_name {
          let code = match item.kind {
            CppTypeKind::CPrimitive => Ok(String::new()),
            CppTypeKind::Unknown => Err("unknown type".to_string()),
            CppTypeKind::Enum { ref values } => {
              let mut value_to_variant: HashMap<i32, EnumValue> = HashMap::new();
              for variant in values {
                let value = self.input_data
                                .cpp_extracted_info
                                .enum_values
                                .get(&item.name)
                                .unwrap()
                                .get(&variant.name)
                                .unwrap();
                if value_to_variant.contains_key(value) {
                  log::warning(format!("warning: {}: duplicated enum variant removed: {} \
                                        (previous variant: {})",
                                       item.name,
                                       variant.name,
                                       value_to_variant.get(value).unwrap().name));
                } else {
                  value_to_variant.insert(*value, variant.clone());
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
                                          value: format!("{}", dummy_value),
                                          description: String::new(),
                                        });
              }
              Ok(format!("#[repr(C)]\npub enum {} {{\n{}\n}}\n\n",
                         rust_type_name.own_name,
                         value_to_variant.iter()
                                         .map(|(value, variant)| {
                                           format!("  {} = {}",
                                                   variant.name.to_class_case1(),
                                                   value)
                                         })
                                         .join(", \n")))
            }
            CppTypeKind::Class { .. } => {
              match self.input_data.cpp_extracted_info.class_sizes.get(&item.name) {
                Some(size) => {
                  Ok(format!("#[repr(C)]\npub struct {} {{\n  _buffer: [{}; {}],\n}}\n\n",
                             rust_type_name.own_name,
                             self.cpp_to_rust_type_map
                                 .get(&"char".to_string())
                                 .unwrap()
                                 .full_name(&self.crate_name),
                             size))
                }
                None => Ok(format!("pub enum {} {{}}\n\n", rust_type_name.own_name)),

              }
            }
            CppTypeKind::TypeDef { .. } => Err("typedefs are not exported yet".to_string()),
            CppTypeKind::Flags { ref enum_name } => {
              let enum_rust_name = self.cpp_to_rust_type_map.get(enum_name).unwrap();
              Ok(format!("pub type {} = {}<{}>;\n\n",
                         rust_type_name.own_name,
                         self.cpp_to_rust_type_map
                             .get(&"QFlags".to_string())
                             .unwrap()
                             .full_name(&self.crate_name),
                         enum_rust_name.full_name(&self.crate_name)))
            }

          };
          match code {
            Ok(code) => {
              file.write(code.as_bytes()).unwrap();
              self.processed_cpp_types.insert(type_name);
            }
            Err(msg) => {
              log::warning(format!("Can't generate Rust type for {}: {}", type_name, msg));
            }
          }
        } else if rust_type_name.module_name != "types" && !rust_type_name.module_name.is_empty() {
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
    write!(file, "#[link(name = \"Qt5Core\")]\n").unwrap();
    write!(file, "#[link(name = \"icui18n\")]\n").unwrap();
    write!(file, "#[link(name = \"icuuc\")]\n").unwrap();
    write!(file, "#[link(name = \"icudata\")]\n").unwrap();
    write!(file, "#[link(name = \"stdc++\")]\n").unwrap();
    write!(file, "#[link(name = \"qtcw\", kind = \"static\")]\n").unwrap();
    write!(file, "extern \"C\" {{\n").unwrap();

    for header in &self.input_data.c_headers.clone() {
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

    }

    write!(file, "}}\n").unwrap();
  }
}
