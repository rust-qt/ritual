use c_generator::CppAndCData;
use cpp_and_c_method::CppAndCMethod;
use cpp_type_map::EnumValue;
use enums::{CppTypeKind, CppTypeOrigin};
use utils::JoinWithString;
use rust_type::{RustTypeName, RustType, CompleteType, RustTypeIndirection};
use c_type::CTypeExtended;

use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;

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
  cpp_to_rust_type_map: HashMap<String, RustTypeName>,
}

impl RustGenerator {
  pub fn new(input_data: CppAndCData, output_path: PathBuf) -> Self {
    RustGenerator {
      input_data: input_data,
      output_path: output_path,
      modules: Vec::new(),
      crate_name: "qt_core".to_string(),
      cpp_to_rust_type_map: HashMap::new(),
    }
  }

  fn c_type_to_complete_type(&self, c_type_ex: &CTypeExtended) -> Result<CompleteType, String> {
    if !self.cpp_to_rust_type_map.contains_key(&c_type_ex.cpp_type.base) {
      return Err(format!("Type has no Rust equivalent: {}", c_type_ex.cpp_type.base));
    }
    let rust_name = self.cpp_to_rust_type_map.get(&c_type_ex.cpp_type.base).unwrap();

    let rust_ffi_type = if c_type_ex.c_type.base == "void" {
      if c_type_ex.c_type.is_pointer {
        RustType::NonVoid {
          base: rust_name.clone(),
          is_const: c_type_ex.c_type.is_const,
          indirection: RustTypeIndirection::Ptr,
          is_option: true,
        }
      } else {
        RustType::Void
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
        is_option: c_type_ex.c_type.is_pointer,
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
          "qintptr" | "qptrdiff" | "QList_difference_type" => "isize",
          "quintptr" => "usize",
          "float" => "f32",
          "double" => "f64",
          "bool" => "bool",
          _ => "",
        };
        if !primitive_type_name.is_empty() {
          Ok(RustTypeName {
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
            Ok(RustTypeName {
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
              Ok(RustTypeName {
                crate_name: self.crate_name.clone(),
                module_name: include_file_to_module_name(include_file),
                own_name: new_name,
              })
            } else {
              println!("warning: type is skipped: {:?}", type_info);
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
                                     RustTypeName {
                                       crate_name: "qt_core".to_string(),
                                       module_name: "flags".to_string(),
                                       own_name: "QFlags".to_string(),
                                     });
  }

  pub fn generate_all(&mut self) {
    self.generate_type_map();
    for header in self.input_data.c_headers.clone() {
      self.generate_one(header.include_file, header.methods);
    }

    let mut lib_file_path = self.output_path.clone();
    lib_file_path.push("qt_core");
    lib_file_path.push("src");
    lib_file_path.push("lib.rs");
    let mut lib_file = File::create(&lib_file_path).unwrap();
    write!(lib_file, "pub mod types;\npub mod flags;\n").unwrap();
    for module in &self.modules {
      write!(lib_file, "pub mod {};\n", module).unwrap();
    }
  }

  pub fn generate_one(&mut self, include_file: String, methods: Vec<CppAndCMethod>) {
    let module_name = include_file_to_module_name(&include_file);
    println!("MODULE: {}", module_name);
    let mut file_path = self.output_path.clone();
    file_path.push("qt_core");
    file_path.push("src");
    file_path.push(format!("{}.rs", module_name));
    let mut file = File::create(&file_path).unwrap();

    let eliminated_name_prefix = format!("{}::", include_file);
    for type_name in self.input_data.cpp_data.types.get_types_from_include_file(&include_file) {
      let item = self.input_data.cpp_data.types.0.get(&type_name).unwrap();
      println!("type: {:?}", item);
      if let Some(rust_type_name) = self.cpp_to_rust_type_map.get(&type_name) {
        if module_name == rust_type_name.module_name {
          match item.kind {
            CppTypeKind::CPrimitive | CppTypeKind::Unknown => {}
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
                  println!("warning: {}: duplicated enum variant removed: {} (previous variant: \
                            {})",
                           item.name,
                           variant.name,
                           value_to_variant.get(value).unwrap().name);
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
              write!(file,
                     "#[repr(C)]\npub enum {} {{\n{}\n}}\n\n",
                     rust_type_name.own_name,
                     value_to_variant.iter()
                                     .map(|(value, variant)| {
                                       format!("  {} = {}", variant.name.to_class_case1(), value)
                                     })
                                     .join(", \n"))
                .unwrap();
            }
            CppTypeKind::Class { .. } => {
              match self.input_data.cpp_extracted_info.class_sizes.get(&item.name) {
                Some(size) => {
                  write!(file,
                         "#[repr(C)]\npub struct {} {{\n  _buffer: [{}; {}],\n}}\n\n",
                         rust_type_name.own_name,
                         self.cpp_to_rust_type_map
                             .get(&"char".to_string())
                             .unwrap()
                             .full_name(&self.crate_name),
                         size)
                    .unwrap();
                }
                None => {
                  write!(file, "pub enum {} {{}}\n\n", rust_type_name.own_name).unwrap();
                }

              }
            }
            CppTypeKind::TypeDef { .. } => {
              println!("warning: typedefs are not exported yet: {}", item.name);
            }
            CppTypeKind::Flags { ref enum_name } => {
              let enum_rust_name = self.cpp_to_rust_type_map.get(enum_name).unwrap();
              write!(file,
                     "pub type {} = {}<{}>;\n\n",
                     rust_type_name.own_name,
                     self.cpp_to_rust_type_map
                         .get(&"QFlags".to_string())
                         .unwrap()
                         .full_name(&self.crate_name),
                     enum_rust_name.full_name(&self.crate_name))
                .unwrap();
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

    // ...

    self.modules.push(module_name);
  }
}
