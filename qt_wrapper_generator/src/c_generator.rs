use cpp_header_data::CppHeaderData;
use cpp_data::CppData;
use c_type::CTypeExtended;
use cpp_type::CppType;
use enums::{AllocationPlace, CFunctionArgumentCppEquivalent, IndirectionChange, CppMethodScope,
            CppTypeOrigin, CppTypeKind, CppTypeIndirection};
use cpp_and_c_method::CppAndCMethod;
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use utils::JoinWithString;
use std::collections::HashMap;
use read_extracted_info::CppExtractedInfo;

pub struct CGenerator {
  qtcw_path: PathBuf,
  cpp_data: CppData,
  cpp_extracted_info: CppExtractedInfo,
}

fn only_c_code(code: String) -> String {
  format!("#ifndef __cplusplus // if C\n{}#endif // if C\n\n", code)
}
fn only_cpp_code(code: String) -> String {
  format!("#ifdef __cplusplus // if C++\n{}#endif // if C++\n\n", code)
}


impl CppAndCMethod {
  fn header_code(&self) -> String {
    format!("{} QTCW_EXPORT {}({});\n",
            self.c_signature.return_type.c_type.to_c_code(),
            self.c_name,
            self.c_signature.arguments_to_c_code())
  }

  fn convert_return_type(&self, expression: String) -> String {
    let mut result = expression;
    match self.c_signature.return_type.conversion.indirection_change {
      IndirectionChange::NoChange => {}
      IndirectionChange::ValueToPointer => {
        match self.allocation_place {
          AllocationPlace::Stack => panic!("stack allocated wrappers are expected to return void!"),
          AllocationPlace::Heap => {
            // constructors are said to return values in parse result,
            // but in reality we use `new` which returns a pointer,
            // so no conversion is necessary for constructors.
            if !self.cpp_method.is_constructor {
              if let Some(ref return_type) = self.cpp_method.return_type {
                result = format!("new {}({})", return_type.base, result);
              } else {
                panic!("cpp self unexpectedly doesn't have return type");
              }
            }
          }
        }
      }
      IndirectionChange::ReferenceToPointer => {
        result = format!("&{}", result);
      }
    }
    if self.c_signature.return_type.conversion.renamed {
      result = format!("reinterpret_cast<{}>({})",
                       self.c_signature
                           .return_type
                           .c_type
                           .to_c_code(),
                       result);
    }
    if self.c_signature.return_type.conversion.qflags_to_uint {
      result = format!("uint({})", result);
    }

    if self.allocation_place == AllocationPlace::Stack && !self.cpp_method.is_constructor {
      if let Some(arg) = self.c_signature.arguments.iter().find(|x| {
        x.cpp_equivalent == CFunctionArgumentCppEquivalent::ReturnValue
      }) {
        if let Some(ref return_type) = self.cpp_method.return_type {
          result = format!("new({}) {}({})", arg.name, return_type.base, result);
        } else {
          panic!("cpp self unexpectedly doesn't have return type");
        }
      }
    }
    result
  }

  fn arguments_values(&self) -> String {
    let mut filled_arguments = vec![];
    for (i, cpp_argument) in self.cpp_method.arguments.iter().enumerate() {
      if let Some(c_argument) = self.c_signature.arguments.iter().find(|x| {
        x.cpp_equivalent == CFunctionArgumentCppEquivalent::Argument(i as i8)
      }) {
        let mut result = c_argument.name.clone();
        match c_argument.argument_type
                        .conversion
                        .indirection_change {
          IndirectionChange::ValueToPointer |
          IndirectionChange::ReferenceToPointer => result = format!("*{}", result),
          IndirectionChange::NoChange => {}
        }
        if c_argument.argument_type.conversion.renamed {
          result = format!("reinterpret_cast<{}>({})",
                           cpp_argument.argument_type.to_cpp_code(),
                           result);
        }
        if c_argument.argument_type.conversion.qflags_to_uint {
          result = format!("{}({})", cpp_argument.argument_type.base, result);
        }
        filled_arguments.push(result);
      } else {
        panic!("Error: no positional argument found\n{:?}", self);
      }
    }

    filled_arguments.into_iter().join(", ")
  }

  fn returned_expression(&self) -> String {
    self.convert_return_type(if self.cpp_method.is_destructor {
      if let Some(arg) = self.c_signature
                             .arguments
                             .iter()
                             .find(|x| x.cpp_equivalent == CFunctionArgumentCppEquivalent::This) {
        format!("qtcw_call_destructor({})", arg.name)
      } else {
        panic!("Error: no this argument found\n{:?}", self);
      }
    } else {
      let result_without_args = if self.cpp_method.is_constructor {
        if let CppMethodScope::Class(ref class_name) = self.cpp_method.scope {
          match self.allocation_place {
            AllocationPlace::Stack => {
              if let Some(arg) = self.c_signature.arguments.iter().find(|x| {
                x.cpp_equivalent == CFunctionArgumentCppEquivalent::ReturnValue
              }) {
                format!("new({}) {}", arg.name, class_name)
              } else {
                panic!("no return value equivalent argument found");
              }
            }
            AllocationPlace::Heap => format!("new {}", class_name),
          }
        } else {
          panic!("constructor not in class scope");
        }
      } else {
        let scope_specifier = if let CppMethodScope::Class(ref class_name) = self.cpp_method
                                                                                 .scope {
          if self.cpp_method.is_static {
            format!("{}::", class_name)
          } else {
            if let Some(arg) = self.c_signature.arguments.iter().find(|x| {
              x.cpp_equivalent == CFunctionArgumentCppEquivalent::This
            }) {
              format!("{}->", arg.name)
            } else {
              panic!("Error: no this argument found\n{:?}", self);
            }
          }
        } else {
          "".to_string()
        };
        format!("{}{}", scope_specifier, self.cpp_method.name)
      };
      format!("{}({})", result_without_args, self.arguments_values())
    })
  }


  fn source_body(&self) -> String {
    if self.cpp_method.is_destructor && self.allocation_place == AllocationPlace::Heap {
      if let Some(arg) = self.c_signature
                             .arguments
                             .iter()
                             .find(|x| x.cpp_equivalent == CFunctionArgumentCppEquivalent::This) {
        format!("delete {};\n", arg.name)
      } else {
        panic!("Error: no this argument found\n{:?}", self);
      }
    } else {
      format!("{}{};\n",
              if self.c_signature.return_type == CTypeExtended::void() {
                ""
              } else {
                "return "
              },
              self.returned_expression())
    }

  }

  fn source_code(&self) -> String {
    format!("{} {}({}) {{\n  {}}}\n\n",
            self.c_signature.return_type.c_type.to_c_code(),
            self.c_name,
            self.c_signature.arguments_to_c_code(),
            self.source_body())
  }
}

// struct CppAndCCode {
//  cpp_code: String,
//  c_code: String,
// }


impl CGenerator {
  pub fn new(cpp_data: CppData, cpp_extracted_info: CppExtractedInfo, qtcw_path: PathBuf) -> Self {
    CGenerator {
      cpp_data: cpp_data,
      cpp_extracted_info: cpp_extracted_info,
      qtcw_path: qtcw_path,
    }
  }

  pub fn generate_all(&mut self) {
    let mut h_path = self.qtcw_path.clone();
    h_path.push("include");
    h_path.push("qtcw.h");
    let mut all_header_file = File::create(&h_path).unwrap();
    write!(all_header_file, "#ifndef QTCW_H\n#define QTCW_H\n\n").unwrap();

    let mut include_file_to_header_data: HashMap<String, Vec<CppHeaderData>> = HashMap::new();

    for data in &self.cpp_data.headers {
      // if white_list.iter().find(|&&x| x == data.include_file).is_none() {
      //  continue;
      // }
      if let Some(ref class_name) = data.class_name {
        if self.cpp_data
               .classes_blacklist
               .iter()
               .find(|&x| x == class_name.as_ref() as &str)
               .is_some() {
          println!("Ignoring {} because it is blacklisted.", data.include_file);
          continue;
        }
        match self.cpp_data.is_template_class(class_name) {
          Ok(is_template_class) => {
            if is_template_class {
              println!("Skipping code generation for header {} because it contains a template \
                        class.\n",
                       data.include_file);
              continue;
            }
          }
          Err(msg) => {
            println!("Skipping code generation for header {} because of error: {}.\n",
                     data.include_file,
                     msg);
            continue;
          }
        }
      }
      if !match include_file_to_header_data.get_mut(&data.include_file) {
        Some(mut vec) => {
          vec.push(data.clone());
          true
        }
        None => false,
      } {
        include_file_to_header_data.insert(data.include_file.clone(), vec![data.clone()]);
      }
    }

    for (include_file, data) in &include_file_to_header_data {
      self.generate_one(include_file, data);
      write!(all_header_file, "#include \"qtcw_{}.h\"\n", include_file).unwrap();

    }

    write!(all_header_file, "#endif // QTCW_H\n").unwrap();




  }

  //  pub fn generate_type_declaration(&self, cpp_type: &CppType, c_type: &CTypeExtended) -> CppAndCCode {
  //    let type_info = self.cpp_data.value(cpp_type.base).unwrap();
  //    match
  //  }



  fn struct_declaration(&self, c_struct_name: &String, full_declaration: bool) -> String {
    if c_struct_name.find("::").is_some() {
      panic!("struct_declaration called for invalid struct name {}",
             c_struct_name);
    }
    // write C struct definition
    let result = if full_declaration &&
                    self.cpp_extracted_info.class_sizes.contains_key(c_struct_name) {
      format!("struct QTCW_{} {{ char space[{}]; }};\n",
              c_struct_name,
              self.cpp_extracted_info.class_sizes.get(c_struct_name).unwrap())
    } else {
      format!("struct QTCW_{};\n", c_struct_name)
    };
    format!("{}typedef struct QTCW_{} {};\n\n",
            result,
            c_struct_name,
            c_struct_name)
  }

  fn generate_type_declaration(&self,
                               c_type_extended: &CTypeExtended,
                               current_include_file: &String,
                               already_declared: &mut Vec<String>)
                               -> String {
    // println!("check_type_for_declaration {:?}", c_type_extended);
    let c_type = &c_type_extended.c_type;
    let cpp_type = &c_type_extended.cpp_type;
    if already_declared.iter().find(|&x| x == &c_type.base).is_some() {
      // println!("already declared");
      return String::new(); //already declared
    }
    if c_type.base == "wchar_t" {
      return only_c_code("#include <wchar.h>\n".to_string());
    }

    let type_info = self.cpp_data.types.0.get(&cpp_type.base).unwrap();
    // println!("type info: {:?}", type_info);
    let mut result = match &type_info.origin {
      &CppTypeOrigin::CBuiltIn => {
        // println!("CBuiltIn");
        String::new()
      }
      &CppTypeOrigin::Unsupported(..) => panic!("this type should have been filtered previously"),
      &CppTypeOrigin::Qt { ref include_file } => {
        let needs_full_declaration = current_include_file == include_file;

        let declaration = match &type_info.kind {
          &CppTypeKind::Unknown => panic!("this type should have been filtered previously"),
          &CppTypeKind::CPrimitive => "".to_string(),
          &CppTypeKind::Enum { ref values } => {
            only_c_code(if needs_full_declaration {
              format!("typedef enum QTCW_{0} {{\n{1}\n}} {0};\n",
                      c_type.base,
                      values.iter()
                            .map(|x| {
                              format!("  {}_{} = {}",
                                      c_type.base,
                                      x.name,
                                      self.cpp_extracted_info
                                          .enum_values
                                          .get(&cpp_type.base) 
                                          .unwrap()
                                          .get(&x.name)
                                          .unwrap())
                            })
                            .join(", \n"))
            } else {
              format!("typedef enum QTCW_{0} {0};\n", c_type.base)
            })
          }
          &CppTypeKind::Flags { .. } => format!("typedef unsigned int {};\n", c_type.base),
          &CppTypeKind::TypeDef { ref meaning } => {
            let c_meaning = meaning.to_c_type(&self.cpp_data.types).unwrap();
            // println!("typedef meaning: {:?}", c_meaning.c_type);
            self.generate_type_declaration(&c_meaning, current_include_file, already_declared) +
            &only_c_code(format!("typedef {} {};\n",
                                 c_meaning.c_type.to_c_code(),
                                 c_type.base))
          }
          &CppTypeKind::Class { .. } => {
            only_c_code(self.struct_declaration(&c_type.base, needs_full_declaration))
          }
        };
        already_declared.push(c_type.base.clone());
        // println!("declaration: {}", declaration);
        // println!("Type {:?} is forward-declared.", c_type.base);
        declaration
      }
    };
    if c_type_extended.conversion.renamed {
      //      println!("write renaming typedef cpp={} c={}",
      //               cpp_type.base,
      //               c_type.base);
      result = result + &only_cpp_code(format!("typedef {} {};\n", cpp_type.base, c_type.base));
    }
    result
  }

  pub fn generate_one(&self, include_file: &String, data_vec: &Vec<CppHeaderData>) {
    let mut cpp_path = self.qtcw_path.clone();
    cpp_path.push("src");
    cpp_path.push(format!("qtcw_{}.cpp", include_file));
    println!("Generating source file: {:?}", cpp_path);

    let mut h_path = self.qtcw_path.clone();
    h_path.push("include");
    h_path.push(format!("qtcw_{}.h", include_file));
    println!("Generating header file: {:?}", h_path);

    let mut cpp_file = File::create(&cpp_path).unwrap();
    let mut h_file = File::create(&h_path).unwrap();

    write!(cpp_file, "#include \"qtcw_{}.h\"\n\n", include_file).unwrap();
    let include_guard_name = format!("QTCW_{}_H", include_file.to_uppercase());
    write!(h_file,
           "#ifndef {}\n#define {}\n\n",
           include_guard_name,
           include_guard_name)
      .unwrap();

    write!(h_file, "#include \"qtcw_global.h\"\n\n").unwrap();


    write!(h_file, "#ifdef __cplusplus\n").unwrap();
    // write!(h_file, "#include <{}>\n", include_file).unwrap();
    write!(h_file, "#include <QtCore>\n").unwrap();
    write!(h_file, "#endif\n\n").unwrap();

    let mut forward_declared_classes = vec![];
    //    if let Some(ref class_name) = data.class_name {
    //      self.write_struct_declaration(&mut h_file, class_name, true, true);
    //      forward_declared_classes.push(class_name.clone());
    //    } else {
    //      println!("Not a class header. Wrapper struct is not generated.");
    //    }

    write!(h_file, "QTCW_EXTERN_C_BEGIN\n\n").unwrap();
    for name in self.cpp_data.types.get_types_from_include_file(&include_file) {
      let cpp_type = CppType {
        is_const: false,
        indirection: CppTypeIndirection::None,
        base: name,
        template_arguments: None,
      };
      if let Ok(c_type_ex) = cpp_type.to_c_type(&self.cpp_data.types) {
        h_file.write(&self.generate_type_declaration(&c_type_ex,
                                                     &include_file,
                                                     &mut forward_declared_classes)
                          .into_bytes())
              .unwrap();
      }
    }

    let mut methods: Vec<CppAndCMethod> = vec![];
    for data in data_vec {
      methods.append(&mut data.process_methods(&self.cpp_data.types)
                              .into_iter()
                              .filter(|method| {
                                if method.cpp_method.is_protected {
                                  println!("Skipping protected method: \n{:?}\n", method);
                                  return false;
                                }
                                if method.cpp_method.is_signal {
                                  println!("Skipping signal: \n{:?}\n", method);
                                  return false;
                                }
                                true
                              })
                              .collect());
    }
    for method in &methods {

      // println!("Generating code for method: {:?}", method);
      h_file.write(&self.generate_type_declaration(&method.c_signature.return_type,
                                                   &include_file,
                                                   &mut forward_declared_classes)
                        .into_bytes())
            .unwrap();
      for arg in &method.c_signature.arguments {
        h_file.write(&self.generate_type_declaration(&arg.argument_type,
                                                     &include_file,
                                                     &mut forward_declared_classes)
                          .into_bytes())
              .unwrap();
      }
    }


    for method in &methods {
      h_file.write(&method.header_code().into_bytes()).unwrap();
      cpp_file.write(&method.source_code().into_bytes()).unwrap();
    }

    write!(h_file, "\nQTCW_EXTERN_C_END\n\n").unwrap();

    write!(h_file, "#endif // {}\n", include_guard_name).unwrap();
    println!("Done.\n")
  }
}
