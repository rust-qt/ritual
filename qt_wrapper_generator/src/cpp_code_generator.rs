use cpp_and_ffi_method::CppAndFfiMethod;
use cpp_ffi_type::IndirectionChange;
use cpp_method::{ReturnValueAllocationPlace, CppMethodScope, CppMethodKind};
use cpp_ffi_function_argument::CppFfiArgumentMeaning;
use cpp_ffi_generator::CppFfiHeaderData;
use log;
use std::fs::File;
use std::io::Write;
use utils::JoinWithString;
use std::path::PathBuf;

fn header_code(method: &CppAndFfiMethod) -> String {
  format!("{} QTCW_EXPORT {}({});\n",
          method.c_signature.return_type.ffi_type.to_cpp_code().unwrap(),
          method.c_name,
          method.c_signature.arguments_to_cpp_code().unwrap())
}

fn convert_return_type(method: &CppAndFfiMethod, expression: String) -> String {
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

fn arguments_values(method: &CppAndFfiMethod) -> String {
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

fn returned_expression(method: &CppAndFfiMethod) -> String {
  convert_return_type(&method,
                      if method.cpp_method.kind == CppMethodKind::Destructor {
                        if let Some(arg) = method.c_signature
                                                 .arguments
                                                 .iter()
                                                 .find(|x| {
                                                   x.meaning == CppFfiArgumentMeaning::This
                                                 }) {
                          format!("qtcw_call_destructor({})", arg.name)
                        } else {
                          panic!("Error: no this argument found\n{:?}", method);
                        }
                      } else {
                        let result_without_args = if method.cpp_method.kind ==
                                                     CppMethodKind::Constructor {
                          if let CppMethodScope::Class(ref class_name) = method.cpp_method.scope {
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
                              ReturnValueAllocationPlace::Heap => format!("new {}", class_name),
                              ReturnValueAllocationPlace::NotApplicable => unreachable!(),
                            }
                          } else {
                            panic!("constructor not in class scope");
                          }
                        } else {
                          let scope_specifier = if let CppMethodScope::Class(ref class_name) =
                                                       method.cpp_method
                                                             .scope {
                            if method.cpp_method.is_static {
                              format!("{}::", class_name)
                            } else {
                              if let Some(arg) = method.c_signature
                                                       .arguments
                                                       .iter()
                                                       .find(|x| {
                                                         x.meaning == CppFfiArgumentMeaning::This
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
                        format!("{}({})", result_without_args, arguments_values(&method))
                      })
}


fn source_body(method: &CppAndFfiMethod) -> String {
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
            returned_expression(&method))
  }

}

fn source_code(method: &CppAndFfiMethod) -> String {
  format!("{} {}({}) {{\n  {}}}\n\n",
          method.c_signature.return_type.ffi_type.to_cpp_code().unwrap(),
          method.c_name,
          method.c_signature.arguments_to_cpp_code().unwrap(),
          source_body(&method))
}

pub fn generate_all_headers_file(qtcw_path: &PathBuf, names: &Vec<String>) {
  let mut h_path = qtcw_path.clone();
  h_path.push("include");
  h_path.push("qtcw.h");
  let mut all_header_file = File::create(&h_path).unwrap();
  write!(all_header_file, "#ifndef QTCW_H\n#define QTCW_H\n\n").unwrap();
  for name in names {
    write!(all_header_file, "#include \"qtcw_{}.h\"\n", name).unwrap();
  }
  write!(all_header_file, "#endif // QTCW_H\n").unwrap();
}

pub fn generate_one(qtcw_path: &PathBuf, data: &CppFfiHeaderData) {
  let ffi_include_file = format!("qtcw_{}.h", data.include_file_base_name);

  let mut cpp_path = qtcw_path.clone();
  cpp_path.push("src");
  cpp_path.push(format!("qtcw_{}.cpp", data.include_file_base_name));
  log::info(format!("Generating source file: {:?}", cpp_path));

  let mut h_path = qtcw_path.clone();
  h_path.push("include");
  h_path.push(&ffi_include_file);
  log::info(format!("Generating header file: {:?}", h_path));

  let mut cpp_file = File::create(&cpp_path).unwrap();
  let mut h_file = File::create(&h_path).unwrap();

  write!(cpp_file, "#include \"{}\"\n\n", ffi_include_file).unwrap();
  let include_guard_name = ffi_include_file.replace(".", "_").to_uppercase();
  write!(h_file,
         "#ifndef {}\n#define {}\n\n",
         include_guard_name,
         include_guard_name)
    .unwrap();

  write!(h_file, "#include \"qtcw_global.h\"\n\n").unwrap();


  write!(h_file, "#ifdef __cplusplus\n").unwrap();
  write!(h_file, "#include <QtCore>\n").unwrap();
  write!(h_file, "#endif\n\n").unwrap();

  write!(h_file, "QTCW_EXTERN_C_BEGIN\n\n").unwrap();

  for method in &data.methods {
    h_file.write(&header_code(method).into_bytes()).unwrap();
    cpp_file.write(&source_code(method).into_bytes()).unwrap();
  }

  write!(h_file, "\nQTCW_EXTERN_C_END\n\n").unwrap();

  write!(h_file, "#endif // {}\n", include_guard_name).unwrap();
}
