use cpp_ffi_data::CppAndFfiMethod;
use cpp_ffi_data::IndirectionChange;
use cpp_method::ReturnValueAllocationPlace;
use cpp_ffi_data::CppFfiArgumentMeaning;
use cpp_ffi_generator::CppFfiHeaderData;
use log;
use std::fs;
use std::fs::File;
use std::io::Write;
use utils::JoinWithString;
use std::path::PathBuf;
use utils::PathBufPushTweak;
use utils::is_msvc;
use cpp_type::{CppTypeIndirection, CppTypeBase};

/// Generates C++ code for the C wrapper library.
pub struct CppCodeGenerator {
  /// Library name
  lib_name: String,
  /// Uppercase library name (for optimization)
  lib_name_upper: String,
  /// Path to the directory where the library is generated
  lib_path: PathBuf,

  is_shared: bool,
  cpp_libs: Vec<String>,
}

impl CppCodeGenerator {
  /// Creates a generator for a library.
  /// lib_name: library name
  /// lib_path: path to the directory where the library is generated
  pub fn new(lib_name: String, lib_path: PathBuf, is_shared: bool, cpp_libs: Vec<String>) -> Self {
    CppCodeGenerator {
      lib_name: lib_name.clone(),
      lib_name_upper: lib_name.to_uppercase(),
      lib_path: lib_path,
      is_shared: is_shared,
      cpp_libs: cpp_libs,
    }
  }

  /// Generates function name, return type and arguments list
  /// as it appears in both function declaration and implementation.
  fn function_signature(&self, method: &CppAndFfiMethod) -> String {
    let name_with_args = format!("{}({})",
                                 method.c_name,
                                 method.c_signature.arguments_to_cpp_code().unwrap());
    match method.c_signature.return_type.ffi_type.base {
      CppTypeBase::FunctionPointer { .. } => {
        method.c_signature.return_type.ffi_type.to_cpp_code(Some(&name_with_args)).unwrap()
      }
      _ => {
        format!("{} {}",
                method.c_signature.return_type.ffi_type.to_cpp_code(None).unwrap(),
                name_with_args)
      }
    }
  }

  /// Generates method declaration for the header.
  fn function_declaration(&self, method: &CppAndFfiMethod) -> String {
    format!("{}_EXPORT {};\n",
            self.lib_name_upper,
            self.function_signature(method))
  }

  /// Wraps expression returned by the original method to
  /// convert it to return type of the FFI method.
  fn convert_return_type(&self, method: &CppAndFfiMethod, expression: String) -> String {
    let mut result = expression;
    match method.c_signature.return_type.conversion {
      IndirectionChange::NoChange => {}
      IndirectionChange::ValueToPointer => {
        match method.allocation_place {
          ReturnValueAllocationPlace::Stack => {
            panic!("stack allocated wrappers are expected to return void!")
          }
          ReturnValueAllocationPlace::NotApplicable => {
            panic!("ValueToPointer conflicts with NotApplicable");
          }
          ReturnValueAllocationPlace::Heap => {
            // constructors are said to return values in parse result,
            // but in reality we use `new` which returns a pointer,
            // so no conversion is necessary for constructors.
            if !method.cpp_method.is_constructor() {
              result = format!("new {}({})",
                               method.cpp_method.return_type.base.to_cpp_code(None).unwrap(),
                               result);
            }
          }
        }
      }
      IndirectionChange::ReferenceToPointer => {
        result = format!("&{}", result);
      }
      IndirectionChange::QFlagsToUInt => {
        result = format!("uint({})", result);
      }
    }

    if method.allocation_place == ReturnValueAllocationPlace::Stack &&
       !method.cpp_method.is_constructor() {
      if let Some(arg) = method.c_signature
        .arguments
        .iter()
        .find(|x| x.meaning == CppFfiArgumentMeaning::ReturnValue) {
        result = format!("new({}) {}({})",
                         arg.name,
                         method.cpp_method.return_type.base.to_cpp_code(None).unwrap(),
                         result);
      }
    }
    result
  }

  /// Generates code for values passed to the original C++ method.
  fn arguments_values(&self, method: &CppAndFfiMethod) -> String {
    let mut filled_arguments = vec![];
    for (i, cpp_argument) in method.cpp_method.arguments.iter().enumerate() {
      if let Some(c_argument) = method.c_signature
        .arguments
        .iter()
        .find(|x| x.meaning == CppFfiArgumentMeaning::Argument(i as i8)) {
        let mut result = c_argument.name.clone();
        match c_argument.argument_type
          .conversion {
          IndirectionChange::ValueToPointer |
          IndirectionChange::ReferenceToPointer => result = format!("*{}", result),
          IndirectionChange::NoChange => {}
          IndirectionChange::QFlagsToUInt => {
            let type_text = if cpp_argument.argument_type.indirection == CppTypeIndirection::Ref &&
                               cpp_argument.argument_type.is_const {
              let mut fake_type = cpp_argument.argument_type.clone();
              fake_type.is_const = false;
              fake_type.indirection = CppTypeIndirection::None;
              fake_type.to_cpp_code(None).unwrap()
            } else {
              cpp_argument.argument_type.to_cpp_code(None).unwrap()
            };
            result = format!("{}({})", type_text, result);
          }
        }
        filled_arguments.push(result);
      } else {
        panic!("Error: no positional argument found\n{:?}", method);
      }
    }

    filled_arguments.into_iter().join(", ")
  }

  /// Generates code for the value returned by the FFI method.
  fn returned_expression(&self, method: &CppAndFfiMethod) -> String {
    let result = if method.cpp_method.is_destructor() {
      if let Some(arg) = method.c_signature
        .arguments
        .iter()
        .find(|x| x.meaning == CppFfiArgumentMeaning::This) {
        format!("{}_call_destructor({})", self.lib_name, arg.name)
      } else {
        panic!("Error: no this argument found\n{:?}", method);
      }
    } else {
      let result_without_args = if method.cpp_method.is_constructor() {
        let class_type = &method.cpp_method.class_membership.as_ref().unwrap().class_type;
        match method.allocation_place {
          ReturnValueAllocationPlace::Stack => {
            if let Some(arg) = method.c_signature
              .arguments
              .iter()
              .find(|x| x.meaning == CppFfiArgumentMeaning::ReturnValue) {
              format!("new({}) {}", arg.name, class_type.to_cpp_code().unwrap())
            } else {
              panic!("no return value equivalent argument found");
            }
          }
          ReturnValueAllocationPlace::Heap => format!("new {}", class_type.to_cpp_code().unwrap()),
          ReturnValueAllocationPlace::NotApplicable => unreachable!(),
        }
      } else {
        let scope_specifier = if let Some(ref class_membership) = method.cpp_method
          .class_membership {
          if class_membership.is_static {
            format!("{}::", class_membership.class_type.to_cpp_code().unwrap())
          } else {
            if let Some(arg) = method.c_signature
              .arguments
              .iter()
              .find(|x| x.meaning == CppFfiArgumentMeaning::This) {
              format!("{}->", arg.name)
            } else {
              panic!("Error: no this argument found\n{:?}", method);
            }
          }
        } else {
          "".to_string()
        };
        let template_args = match method.cpp_method.template_arguments_values {
          Some(ref args) => {
            format!("<{}>",
                    args.iter().map(|x| x.to_cpp_code(None).unwrap()).join(", "))
          }
          None => String::new(),
        };
        format!("{}{}{}",
                scope_specifier,
                method.cpp_method.name,
                template_args)
      };
      format!("{}({})", result_without_args, self.arguments_values(method))
    };
    self.convert_return_type(method, result)
  }

  /// Generates body of the FFI method implementation.
  fn source_body(&self, method: &CppAndFfiMethod) -> String {
    if method.cpp_method.is_destructor() &&
       method.allocation_place == ReturnValueAllocationPlace::Heap {
      if let Some(arg) = method.c_signature
        .arguments
        .iter()
        .find(|x| x.meaning == CppFfiArgumentMeaning::This) {
        format!("delete {};\n", arg.name)
      } else {
        panic!("Error: no this argument found\n{:?}", method);
      }
    } else {
      format!("{}{};\n",
              if method.c_signature.return_type.ffi_type.is_void() {
                ""
              } else {
                "return "
              },
              self.returned_expression(&method))
    }
  }

  /// Generates implementation of the FFI method for the source file.
  fn function_implementation(&self, method: &CppAndFfiMethod) -> String {
    format!("{} {{\n  {}}}\n\n",
            self.function_signature(method),
            self.source_body(&method))
  }

  /// Generates main files and directories of the library.
  pub fn generate_template_files(&self,
                                 cpp_lib_include_file: &str,
                                 include_directories: &[String],
                                 framework_directories: &[String]) {
    let name_upper = self.lib_name.to_uppercase();
    let mut cmakelists_file = File::create(self.lib_path.with_added("CMakeLists.txt")).unwrap();
    let mut cxx_flags = String::new();
    if !is_msvc() {
      cxx_flags.push_str("-fPIC -std=gnu++11");
    }
    for dir in framework_directories {
      cxx_flags.push_str(&format!(" -F\\\"{}\\\"", dir));
    }
    write!(cmakelists_file,
           include_str!("../templates/c_lib/CMakeLists.txt"),
           lib_name_lowercase = &self.lib_name,
           lib_name_uppercase = name_upper,
           include_directories = include_directories.into_iter()
             .map(|x| format!("\"{}\"", x.replace(r"\", r"\\")))
             .join(" "),
           library_type = if self.is_shared { "SHARED" } else { "STATIC" },
           target_link_libraries = if self.is_shared {
             format!("target_link_libraries({} {})",
                     &self.lib_name,
                     self.cpp_libs.join(" "))
           } else {
             String::new()
           },
           cxx_flags = cxx_flags)
      .unwrap();
    let src_dir = self.lib_path.with_added("src");
    fs::create_dir_all(&src_dir).unwrap();

    let include_dir = self.lib_path.with_added("include");
    fs::create_dir_all(&include_dir).unwrap();

    let mut exports_file = File::create({
        let mut path = include_dir.clone();
        path.push(format!("{}_exports.h", &self.lib_name));
        path
      })
      .unwrap();
    write!(exports_file,
           include_str!("../templates/c_lib/exports.h"),
           lib_name_uppercase = name_upper)
      .unwrap();

    let mut global_file =
      File::create(include_dir.with_added(format!("{}_global.h", &self.lib_name))).unwrap();
    write!(global_file,
           include_str!("../templates/c_lib/global.h"),
           lib_name_lowercase = &self.lib_name,
           lib_name_uppercase = name_upper,
           cpp_lib_include_file = cpp_lib_include_file)
      .unwrap();
  }

  pub fn generate_files(&self, data: &[CppFfiHeaderData]) {
    self.generate_all_headers_file(data.iter().map(|x| &x.include_file));
    for item in data {
      self.generate_one(item);
    }
  }

  /// Generates the header file that includes all other headers of the library.
  fn generate_all_headers_file<'a, I: Iterator<Item = &'a String>>(&self, names: I) {
    let mut h_path = self.lib_path.clone();
    h_path.push("include");
    h_path.push(format!("{}.h", &self.lib_name));
    let mut all_header_file = File::create(&h_path).unwrap();
    write!(all_header_file,
           "#ifndef {0}_H\n#define {0}_H\n\n",
           &self.lib_name_upper)
      .unwrap();
    for name in names {
      write!(all_header_file,
             "#include \"{}_{}.h\"\n",
             &self.lib_name,
             name)
        .unwrap();
    }
    write!(all_header_file, "#endif // {}_H\n", &self.lib_name_upper).unwrap();
  }

  /// Generates a header file and a source file for a portion of data
  /// corresponding to a header file of original C++ library.
  fn generate_one(&self, data: &CppFfiHeaderData) {
    let ffi_include_file = format!("{}_{}.h", &self.lib_name, data.include_file_base_name);

    let cpp_path = self.lib_path
      .with_added("src")
      .with_added(format!("{}_{}.cpp", &self.lib_name, data.include_file_base_name));
    log::noisy(format!("Generating source file: {:?}", cpp_path));

    let h_path = self.lib_path.with_added("include").with_added(&ffi_include_file);
    log::noisy(format!("Generating header file: {:?}", h_path));

    let mut cpp_file = File::create(&cpp_path).unwrap();
    let mut h_file = File::create(&h_path).unwrap();

    write!(cpp_file, "#include \"{}\"\n\n", ffi_include_file).unwrap();
    let include_guard_name = ffi_include_file.replace(".", "_").to_uppercase();
    write!(h_file,
           "#ifndef {}\n#define {}\n\n",
           include_guard_name,
           include_guard_name)
      .unwrap();

    write!(h_file, "#include \"{}_global.h\"\n\n", &self.lib_name).unwrap();

    write!(h_file, "extern \"C\" {{\n\n").unwrap();

    for method in &data.methods {
      h_file.write(&self.function_declaration(method).into_bytes()).unwrap();
      cpp_file.write(&self.function_implementation(method).into_bytes()).unwrap();
    }

    write!(h_file, "\n}} // extern \"C\"\n\n").unwrap();

    write!(h_file, "#endif // {}\n", include_guard_name).unwrap();
  }
}
