use c_generator::CppAndCData;
use cpp_and_c_method::CppAndCMethod;
use cpp_type_map::EnumValue;
use enums::CppTypeKind;
use utils::JoinWithString;

use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;

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
// use self::inflector::cases::snakecase::to_snake_case;

// fn to_class_case(s: &String) -> String {
//  return s.clone();
//
// }

pub struct RustGenerator {
  input_data: CppAndCData,
  output_path: PathBuf,
  modules: Vec<String>,
}

impl RustGenerator {
  pub fn new(input_data: CppAndCData, output_path: PathBuf) -> Self {
    RustGenerator {
      input_data: input_data,
      output_path: output_path,
      modules: Vec::new(),
    }

  }

  pub fn generate_all(&mut self) {
    for header in self.input_data.c_headers.clone() {
      self.generate_one(header.include_file, header.methods);
    }

    let mut lib_file_path = self.output_path.clone();
    lib_file_path.push("qt_core");
    lib_file_path.push("src");
    lib_file_path.push("lib.rs");
    let mut lib_file = File::create(&lib_file_path).unwrap();
    write!(lib_file, "mod lib_root; pub use lib_root::*;").unwrap();
    for module in &self.modules {
      write!(lib_file, "pub mod {};\n", module).unwrap();
    }
  }

  pub fn generate_one(&mut self, include_file: String, methods: Vec<CppAndCMethod>) {
    let include_without_prefix = if include_file == "Qt" {
      "general".to_string()
    } else if include_file.starts_with("Qt") {
      include_file[2..].to_string()
    } else if include_file.starts_with("Q") {
      include_file[1..].to_string()
    } else {
      include_file.clone()
    };
    let module_name = include_without_prefix.to_snake_case();
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
      let mut new_name = item.name.clone();
      if new_name.starts_with(&eliminated_name_prefix) {
        new_name = new_name[eliminated_name_prefix.len()..].to_string();
      }
      new_name = new_name.replace("::", "_").to_class_case1();

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
              println!("warning: {}: duplicated enum variant removed: {} (previous variant: {})",
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
                 new_name,
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
                     "#[repr(C)]\npub struct {} {{\n  _buffer: [::c_char; {}],\n}}\n\n",
                     new_name,
                     size)
                .unwrap();
            }
            None => {
              write!(file, "pub enum {} {{}}\n\n", new_name).unwrap();
            }

          }
        }
        CppTypeKind::TypeDef { .. } => {
          println!("warning: typedefs are not exported yet: {}", item.name);
        }
        CppTypeKind::Flags { ref enum_name } => {
          write!(file,
                 "pub type {} = ::QFlags; //enum={}\n\n",
                 new_name,
                 enum_name)
            .unwrap();
        }

      }
    }

    // ...


    self.modules.push(module_name);
  }
}
