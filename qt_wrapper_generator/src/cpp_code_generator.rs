use cpp_and_ffi_method::CppAndFfiMethod;
use cpp_ffi_type::IndirectionChange;
use cpp_method::{ReturnValueAllocationPlace, CppMethodScope, CppMethodKind};
use cpp_ffi_function_argument::CppFfiArgumentMeaning;
use cpp_ffi_generator::CppFfiHeaderData;
use log;
use std::fs;
use std::io::Write;
use utils::JoinWithString;
use std::path::PathBuf;
use tweaked_file::TweakedFile;
use utils::PathBufPushTweak;

pub struct CppCodeGenerator {
  lib_name: String,
  lib_name_upper: String,
  lib_path: PathBuf,
}

impl CppCodeGenerator {
  pub fn new(lib_name: String, lib_path: PathBuf) -> Self {
    CppCodeGenerator {
      lib_name: lib_name.clone(),
      lib_name_upper: lib_name.to_uppercase(),
      lib_path: lib_path,
    }
  }

  fn header_code(&self, method: &CppAndFfiMethod) -> String {
    format!("{} {}_EXPORT {}({});\n",
            method.c_signature.return_type.ffi_type.to_cpp_code().unwrap(),
            self.lib_name_upper,
            method.c_name,
            method.c_signature.arguments_to_cpp_code().unwrap())
  }

  fn convert_return_type(&self, method: &CppAndFfiMethod, expression: String) -> String {
    let mut result = expression;
    match method.c_signature.return_type.conversion.indirection_change {
      IndirectionChange::NoChange => {}
      IndirectionChange::ValueToPointer => {
        match method.allocation_place {
          ReturnValueAllocationPlace::Stack => {
            panic!("stack allocated wrappers are expected to return void!")
          }
          ReturnValueAllocationPlace::Heap |
          ReturnValueAllocationPlace::NotApplicable => {
            // constructors are said to return values in parse result,
            // but in reality we use `new` which returns a pointer,
            // so no conversion is necessary for constructors.
            if method.cpp_method.kind != CppMethodKind::Constructor {
              if let Some(ref return_type) = method.cpp_method.return_type {
                result = format!("new {}({})",
                                 return_type.base.to_cpp_code().unwrap(),
                                 result)
              } else {
                panic!("cpp method unexpectedly doesn't have return type");
              }
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
       method.cpp_method.kind != CppMethodKind::Constructor {
      if let Some(arg) = method.c_signature
                               .arguments
                               .iter()
                               .find(|x| x.meaning == CppFfiArgumentMeaning::ReturnValue) {
        if let Some(ref return_type) = method.cpp_method.return_type {
          result = format!("new({}) {}({})",
                           arg.name,
                           return_type.base.to_cpp_code().unwrap(),
                           result);
        } else {
          panic!("cpp method unexpectedly doesn't have return type");
        }
      }
    }
    result
  }

  fn arguments_values(&self, method: &CppAndFfiMethod) -> String {
    let mut filled_arguments = vec![];
    for (i, cpp_argument) in method.cpp_method.arguments.iter().enumerate() {
      if let Some(c_argument) = method.c_signature
                                      .arguments
                                      .iter()
                                      .find(|x| {
                                        x.meaning == CppFfiArgumentMeaning::Argument(i as i8)
                                      }) {
        let mut result = c_argument.name.clone();
        match c_argument.argument_type
                        .conversion
                        .indirection_change {
          IndirectionChange::ValueToPointer |
          IndirectionChange::ReferenceToPointer => result = format!("*{}", result),
          IndirectionChange::NoChange => {}
          IndirectionChange::QFlagsToUInt => {
            result = format!("{}({})",
                             cpp_argument.argument_type.to_cpp_code().unwrap(),
                             result);
          }
        }
        filled_arguments.push(result);
      } else {
        panic!("Error: no positional argument found\n{:?}", method);
      }
    }

    filled_arguments.into_iter().join(", ")
  }

  fn returned_expression(&self, method: &CppAndFfiMethod) -> String {
    self.convert_return_type(&method,
                             if method.cpp_method.kind == CppMethodKind::Destructor {
                               if let Some(arg) = method.c_signature
                                                        .arguments
                                                        .iter()
                                                        .find(|x| {
                                                          x.meaning == CppFfiArgumentMeaning::This
                                                        }) {
                                 format!("{}_call_destructor({})", self.lib_name, arg.name)
                               } else {
                                 panic!("Error: no this argument found\n{:?}", method);
                               }
                             } else {
                               let result_without_args = if method.cpp_method.kind ==
                                                            CppMethodKind::Constructor {
                                 if let CppMethodScope::Class(ref class_name) = method.cpp_method
                                                                                      .scope {
                                   match method.allocation_place {
                                     ReturnValueAllocationPlace::Stack => {
                                       if let Some(arg) = method.c_signature
                                                                .arguments
                                                                .iter()
                                                                .find(|x| {
                                                                  x.meaning ==
                                                                  CppFfiArgumentMeaning::ReturnValue
                                                                }) {
                                         format!("new({}) {}", arg.name, class_name)
                                       } else {
                                         panic!("no return value equivalent argument found");
                                       }
                                     }
                                     ReturnValueAllocationPlace::Heap => {
                                       format!("new {}", class_name)
                                     }
                                     ReturnValueAllocationPlace::NotApplicable => unreachable!(),
                                   }
                                 } else {
                                   panic!("constructor not in class scope");
                                 }
                               } else {
                                 let scope_specifier =
                                   if let CppMethodScope::Class(ref class_name) =
                                          method.cpp_method
                                                .scope {
                                     if method.cpp_method.is_static {
                                       format!("{}::", class_name)
                                     } else {
                                       if let Some(arg) = method.c_signature
                                                                .arguments
                                                                .iter()
                                                                .find(|x| {
                                                                  x.meaning ==
                                                                  CppFfiArgumentMeaning::This
                                                                }) {
                                         format!("{}->", arg.name)
                                       } else {
                                         panic!("Error: no this argument found\n{:?}", method);
                                       }
                                     }
                                   } else {
                                     "".to_string()
                                   };
                                 format!("{}{}", scope_specifier, method.cpp_method.name)
                               };
                               format!("{}({})",
                                       result_without_args,
                                       self.arguments_values(&method))
                             })
  }


  fn source_body(&self, method: &CppAndFfiMethod) -> String {
    if method.cpp_method.kind == CppMethodKind::Destructor &&
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

  fn source_code(&self, method: &CppAndFfiMethod) -> String {
    format!("{} {}({}) {{\n  {}}}\n\n",
            method.c_signature.return_type.ffi_type.to_cpp_code().unwrap(),
            method.c_name,
            method.c_signature.arguments_to_cpp_code().unwrap(),
            self.source_body(&method))
  }

  pub fn generate_template_files(&self,
                                 link_libraries: &Vec<String>,
                                 cpp_lib_include_file: &String,
                                 include_directories: &Vec<String>) {
    let name_upper = self.lib_name.to_uppercase();
    let mut cmakelists_file = TweakedFile::create(self.lib_path.with_added("CMakeLists.txt"))
                                .unwrap();
    write!(cmakelists_file,
           include_str!("../templates/c_lib/CMakeLists.txt"),
           lib_name_lowercase = &self.lib_name,
           lib_name_uppercase = name_upper,
           link_libraries = link_libraries.join(" "),
           include_directories = include_directories.into_iter()
                                                    .map(|x| format!("\"{}\"", x))
                                                    .join(" "))
      .unwrap();
    let src_dir = {
      let mut path = self.lib_path.clone();
      path.push("src");
      path
    };
    fs::create_dir_all(&src_dir).unwrap();

    let include_dir = self.lib_path.with_added("include");
    fs::create_dir_all(&include_dir).unwrap();

    let mut exports_file = TweakedFile::create({
                             let mut path = include_dir.clone();
                             path.push(format!("{}_exports.h", &self.lib_name));
                             path
                           })
                             .unwrap();
    write!(exports_file,
           include_str!("../templates/c_lib/exports.h"),
           lib_name_uppercase = name_upper)
      .unwrap();

    let mut global_file = TweakedFile::create(include_dir.with_added(format!("{}_global.h",
                                                                             &self.lib_name)))
                            .unwrap();
    write!(global_file,
           include_str!("../templates/c_lib/global.h"),
           lib_name_lowercase = &self.lib_name,
           lib_name_uppercase = name_upper,
           cpp_lib_include_file = cpp_lib_include_file)
      .unwrap();
  }

  pub fn generate_all_headers_file(&self, names: &Vec<String>) {
    let mut h_path = self.lib_path.clone();
    h_path.push("include");
    h_path.push(format!("{}.h", &self.lib_name));
    let mut all_header_file = TweakedFile::create(&h_path).unwrap();
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

  pub fn generate_one(&self, data: &CppFfiHeaderData) {
    let ffi_include_file = format!("{}_{}.h", &self.lib_name, data.include_file_base_name);

    let cpp_path = self.lib_path.with_added("src").with_added(format!("{}_{}.cpp",
                                                                      &self.lib_name,
                                                                      data.include_file_base_name));
    log::info(format!("Generating source file: {:?}", cpp_path));

    let h_path = self.lib_path.with_added("include").with_added(&ffi_include_file);
    log::info(format!("Generating header file: {:?}", h_path));

    let mut cpp_file = TweakedFile::create(&cpp_path).unwrap();
    let mut h_file = TweakedFile::create(&h_path).unwrap();

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
      h_file.write(&self.header_code(method).into_bytes()).unwrap();
      cpp_file.write(&self.source_code(method).into_bytes()).unwrap();
    }

    write!(h_file, "\n}} // extern \"C\"\n\n").unwrap();

    write!(h_file, "#endif // {}\n", include_guard_name).unwrap();
  }
}
