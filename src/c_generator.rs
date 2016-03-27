use cpp_header_data::CppHeaderData;
use cpp_data::CppData;
use c_type::CTypeExtended;
use enums::{AllocationPlace, CFunctionArgumentCppEquivalent, IndirectionChange, CppMethodScope, CppTypeOrigin, CppTypeKind};
use cpp_and_c_method::CppAndCMethod;
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use utils::JoinWithString;

pub struct CGenerator {
  qtcw_path: PathBuf,
  cpp_data: CppData,
  sized_classes: Vec<String>,
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
        filled_arguments.push(result);
      } else {
        panic!("Error: no positional argument found\n{:?}", self);
      }
    }

    filled_arguments.into_iter().join(", ")
  }

  fn returned_expression(&self) -> String {
    let mut result = if self.cpp_method.is_constructor {
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
      let scope_specifier = if let CppMethodScope::Class(ref class_name) = self.cpp_method.scope {
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
    result = format!("{}({})", result, self.arguments_values());
    self.convert_return_type(result)
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
  pub fn new(cpp_data: CppData, qtcw_path: PathBuf) -> Self {
    CGenerator {
      cpp_data: cpp_data,
      qtcw_path: qtcw_path,
      sized_classes: Vec::new(),
    }
  }

  pub fn generate_all(&mut self) {
    self.sized_classes = self.generate_size_definer_class_list();
    let white_list = vec!["QPoint", "QRect", "QBitArray", "QByteArray"];

    for data in &self.cpp_data.headers {
      if white_list.iter().find(|&&x| x == data.include_file).is_none() {
        continue;
      }

      self.generate_one(data);

    }



  }

  //  pub fn generate_type_declaration(&self, cpp_type: &CppType, c_type: &CTypeExtended) -> CppAndCCode {
  //    let type_info = self.cpp_data.value(cpp_type.base).unwrap();
  //    match
  //  }




  pub fn generate_size_definer_class_list(&self) -> Vec<String> {
    let show_output = false;

    let mut sized_classes = Vec::new();
    // TODO: black magic happens here
    let blacklist = vec!["QFlags", "QWinEventNotifier", "QPair", "QGlobalStatic"];

    let mut h_path = self.qtcw_path.clone();
    h_path.push("size_definer");
    h_path.push("classes_list.h");
    println!("Generating file: {:?}", h_path);
    let mut h_file = File::create(&h_path).unwrap();
    for item in &self.cpp_data.headers {
      if item.involves_templates() {
        // TODO: support template classes!
        if show_output {
          println!("Ignoring {} because it involves templates.",
                   item.include_file);
        }
        continue;
      }
      if let Some(ref class_name) = item.class_name {
        if blacklist.iter().find(|&&x| x == class_name.as_ref() as &str).is_some() {
          if show_output {
            println!("Ignoring {} because it is blacklisted.", item.include_file);
          }
          continue;

        }
        let define_name = class_name.replace("::", "_");
        if show_output {
          println!("Requesting size definition for {}.", class_name);
        }
        write!(h_file, "ADD({}, {});\n", define_name, class_name).unwrap();
        sized_classes.push(class_name.clone());
      }
    }
    println!("Done.\n");
    sized_classes
  }

  fn struct_declaration(&self, c_struct_name: &String, full_declaration: bool) -> String {
    // write C struct definition
    let result = if full_declaration &&
                        self.sized_classes.iter().find(|x| *x == c_struct_name).is_some() {
      format!("struct QTCW_{} {{ char space[QTCW_sizeof_{}]; }};\n",
              c_struct_name,
              c_struct_name)
    } else {
      format!("struct QTCW_{};\n", c_struct_name)
    };
    format!("{}typedef struct QTCW_{} {};\n\n",
            result,
            c_struct_name,
            c_struct_name)
  }


  pub fn generate_one(&self, data: &CppHeaderData) {
    let mut cpp_path = self.qtcw_path.clone();
    cpp_path.push("src");
    cpp_path.push(format!("qtcw_{}.cpp", data.include_file));
    println!("Generating source file: {:?}", cpp_path);

    let mut h_path = self.qtcw_path.clone();
    h_path.push("include");
    h_path.push(format!("qtcw_{}.h", data.include_file));
    println!("Generating header file: {:?}", h_path);

    let mut cpp_file = File::create(&cpp_path).unwrap();
    let mut h_file = File::create(&h_path).unwrap();

    write!(cpp_file, "#include \"qtcw_{}.h\"\n\n", data.include_file).unwrap();
    let include_guard_name = format!("QTCW_{}_H", data.include_file.to_uppercase());
    write!(h_file,
           "#ifndef {}\n#define {}\n\n",
           include_guard_name,
           include_guard_name)
      .unwrap();

    write!(h_file, "#include \"qtcw_global.h\"\n\n").unwrap();


    write!(h_file, "#ifdef __cplusplus\n").unwrap();
    // write!(h_file, "#include <{}>\n", data.include_file).unwrap();
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
    let methods = data.process_methods(&self.cpp_data.types);
    {
      let mut check_type_for_declaration = |c_type_extended: &CTypeExtended| {
        let c_type = &c_type_extended.c_type;
        let cpp_type = &c_type_extended.cpp_type;
        if forward_declared_classes.iter().find(|&x| x == &c_type.base).is_some() {
          return; //already declared
        }
        let type_info = self.cpp_data.types.get_info(&cpp_type.base).unwrap();
        let needs_full_declaration;
        match &type_info.origin {
          &CppTypeOrigin::CBuiltIn => return,
          &CppTypeOrigin::Qt { ref include_file } => {
            needs_full_declaration = &data.include_file == include_file
          }
          &CppTypeOrigin::Unsupported(..) => panic!("this type should have been filtered previously"),
        }
        let declaration = match &type_info.kind {
          &CppTypeKind::CPrimitive => {
            panic!("this type should have been filtered in previous match")
          }
          &CppTypeKind::Enum { ref values } => {
            only_c_code(if needs_full_declaration {
              format!("enum {} {{\n{}}};\n",
                      c_type.base,
                      values.iter().map(|x| format!("  {} = {},", x.name, x.value)).join("\n"))
            } else {
              format!("enum {};\n", c_type.base)
            })
          }
          &CppTypeKind::Flags { .. } => format!("typedef uint {};\n", c_type.base),
          &CppTypeKind::TypeDef { .. } => panic!("get_info can't return TypeDef"),
          &CppTypeKind::Class { .. } => {
            only_c_code(self.struct_declaration(&c_type.base, needs_full_declaration))
          }
        };
        h_file.write(&only_c_code(declaration).into_bytes()).unwrap();

        forward_declared_classes.push(c_type.base.clone());
        // println!("Type {:?} is forward-declared.", t);
        if c_type_extended.conversion.renamed {
          h_file.write(&only_cpp_code(format!("typedef {} {};\n", cpp_type.base, c_type.base)).into_bytes()).unwrap();
        }
      };

      for method in &methods {
        check_type_for_declaration(&method.c_signature.return_type);
        for arg in &method.c_signature.arguments {
          check_type_for_declaration(&arg.argument_type);
        }
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
