use c_generator::CppAndCData;
use cpp_and_c_method::CppAndCMethod;

use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

extern crate inflector;
use self::inflector::Inflector;

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
    write!(file, "//...").unwrap();

    for item in self.input_data.cpp_data.types.get_types_from_include_file(&include_file) {
      println!("type: {:?}", item);
    }

    //...


    self.modules.push(module_name);
  }
}
