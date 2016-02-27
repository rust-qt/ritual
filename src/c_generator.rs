use std::path::PathBuf;
use structs::*;
use std::fs::File;
use std::io::Write;

pub struct CGenerator {
  qtcw_path: PathBuf,
  all_data: Vec<CppHeaderData>,
  sized_classes: Vec<String>,
}



impl CGenerator {
  pub fn new(all_data: Vec<CppHeaderData>, qtcw_path: PathBuf) -> Self {
    CGenerator {
      all_data: all_data,
      qtcw_path: qtcw_path,
      sized_classes: Vec::new(),
    }
  }

  pub fn generate_all(&mut self) {
    self.sized_classes = self.generate_size_definer_class_list();
    let white_list = vec!["QPoint", "QRect", "QBitArray"];

    for data in &self.all_data {
      if white_list.iter().find(|&&x| x == data.include_file).is_none() {
        continue;
      }

      self.generate_one(data);

    }



  }




  pub fn generate_size_definer_class_list(&self) -> Vec<String> {
    let mut sized_classes = Vec::new();
    // TODO: black magic happens here
    let blacklist = vec!["QFlags", "QWinEventNotifier", "QPair", "QGlobalStatic"];

    let mut h_path = self.qtcw_path.clone();
    h_path.push("size_definer");
    h_path.push("classes_list.h");
    println!("Generating file: {:?}", h_path);
    let mut h_file = File::create(&h_path).unwrap();
    for item in &self.all_data {
      if item.involves_templates() {
        // TODO: support template classes!
        println!("Ignoring {} because it involves templates.",
                 item.include_file);
        continue;
      }
      if let Some(ref class_name) = item.class_name {
        if class_name.contains("::") {
          // TODO: support nested classes!
          println!("Ignoring {} because it is a nested class.",
                   item.include_file);
          continue;
        }
        if blacklist.iter().find(|&&x| x == class_name.as_ref() as &str).is_some() {
          println!("Ignoring {} because it is blacklisted.", item.include_file);
          continue;

        }
        println!("Requesting size definition for {}.", class_name);
        write!(h_file, "ADD({});\n", class_name).unwrap();
        sized_classes.push(class_name.clone());
      }
    }
    println!("Done.\n");
    sized_classes
  }

  fn write_struct_declaration(&self,
                              h_file: &mut File,
                              class_name: &String,
                              full_declaration: bool) {
    // write C struct definition
    write!(h_file, "#ifndef __cplusplus // if C\n").unwrap();
    if full_declaration && self.sized_classes.iter().find(|x| *x == class_name).is_some() {
      write!(h_file,
             "struct QTCW_{} {{ char space[QTCW_sizeof_{}]; }};\n",
             class_name,
             class_name)
        .unwrap();
    } else {
      write!(h_file, "struct QTCW_{};\n", class_name).unwrap();
    }
    write!(h_file,
           "typedef struct QTCW_{} {};\n",
           class_name,
           class_name)
      .unwrap();
    write!(h_file, "#endif\n\n").unwrap();
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
    write!(h_file, "#include <{}>\n", data.include_file).unwrap();
    write!(h_file, "#endif\n\n").unwrap();

    if let Some(ref class_name) = data.class_name {
      self.write_struct_declaration(&mut h_file, class_name, true);

    } else {
      println!("Not a class header. Wrapper struct is not generated.");
    }

    write!(h_file, "QTCW_EXTERN_C_BEGIN\n\n").unwrap();
    let methods = data.process_methods();
    {
      let mut forward_declared_classes = vec![];
      let mut check_type_for_forward_declaration = |t: &CTypeExtended| {
        if t.is_primitive {
          return; //it's built-in type
        }
        if let Some(ref class_name) = data.class_name {
          if &t.c_type.base == class_name {
            return; //it's main type for this header
          }
        }
        if forward_declared_classes.iter().find(|&x| x == &t.c_type.base).is_some() {
          return; //already declared
        }
        if !t.c_type.is_pointer {
          println!("Warning: value of non-primitive type encountered ({:?})",
                   t.c_type);
        }
        self.write_struct_declaration(&mut h_file, &t.c_type.base, false);
        forward_declared_classes.push(t.c_type.base.clone());
      };

      for method in &methods {
        check_type_for_forward_declaration(&method.c_signature.return_type);
        for arg in &method.c_signature.arguments {
          check_type_for_forward_declaration(&arg.argument_type);
        }
      }
    }


    for method in &methods {
      write!(h_file,
             "{} QTCW_EXPORT {}({});\n",
             method.c_signature.return_type.c_type.to_c_code(),
             method.c_name,
             method.c_signature.arguments_to_c_code())
        .unwrap();

      write!(cpp_file,
             "{} {}({}) {{\n  ",
             method.c_signature.return_type.c_type.to_c_code(),
             method.c_name,
             method.c_signature.arguments_to_c_code())
        .unwrap();

      if method.cpp_method.is_destructor && method.allocation_place == AllocationPlace::Heap {
        if let Some(arg) = method.c_signature.arguments.iter().find(|x| {
          x.cpp_equivalent == CFunctionArgumentCppEquivalent::This
        }) {
          write!(cpp_file, "delete {};\n", arg.name);
        } else {
          panic!("Error: no this argument found\n{:?}", method);
        }
      } else {

        if method.c_signature.return_type != CTypeExtended::void() {
          write!(cpp_file, "return ");
        }
        let mut return_type_conversion_prefix = "".to_string();
        let mut return_type_conversion_suffix = "".to_string();
        match method.c_signature.return_type.conversion {
          CppToCTypeConversion::NoConversion => {}
          CppToCTypeConversion::ValueToPointer => {
            match method.allocation_place {
              AllocationPlace::Stack => {
                panic!("stack allocated wrappers are expected to return void!")
              }
              AllocationPlace::Heap => {
                // constructors are said to return values in parse result,
                // but in reality we use `new` which returns a pointer,
                // so no conversion is necessary for constructors.
                if !method.cpp_method.is_constructor {
                  return_type_conversion_prefix = format!("new {}(",
                                                          method.c_signature
                                                                .return_type
                                                                .c_type
                                                                .base);
                  return_type_conversion_suffix = ")".to_string();
                }
              }
            }
          }
          CppToCTypeConversion::ReferenceToPointer => {
            return_type_conversion_prefix = "&".to_string();
          }
        }
        if method.allocation_place == AllocationPlace::Stack && !method.cpp_method.is_constructor {
          if let Some(arg) = method.c_signature.arguments.iter().find(|x| {
            x.cpp_equivalent == CFunctionArgumentCppEquivalent::ReturnValue
          }) {
            return_type_conversion_prefix = format!("new({}) {}(",
                                                    arg.name,
                                                    arg.argument_type.c_type.base);
            return_type_conversion_suffix = ")".to_string();

          }
        }

        write!(cpp_file, "{}", return_type_conversion_prefix);

        if method.cpp_method.is_constructor {
          if let CppMethodScope::Class(ref class_name) = method.cpp_method.scope {
            match method.allocation_place {
              AllocationPlace::Stack => {
                if let Some(arg) = method.c_signature.arguments.iter().find(|x| {
                  x.cpp_equivalent == CFunctionArgumentCppEquivalent::ReturnValue
                }) {
                  write!(cpp_file, "new({}) {}", arg.name, class_name);
                } else {
                  panic!("no return value equivalent argument found");
                }
              }
              AllocationPlace::Heap => {
                write!(cpp_file, "new {}", class_name);
              }
            }
          } else {
            panic!("constructor not in class scope");
          }


        } else {
          if let CppMethodScope::Class(ref class_name) = method.cpp_method.scope {
            if method.cpp_method.is_static {
              write!(cpp_file, "{}::", class_name);
            } else {
              if let Some(arg) = method.c_signature.arguments.iter().find(|x| {
                x.cpp_equivalent == CFunctionArgumentCppEquivalent::This
              }) {
                write!(cpp_file, "{}->", arg.name);
              } else {
                panic!("Error: no this argument found\n{:?}", method);
              }
            }
          }
          write!(cpp_file, "{}", method.cpp_method.name);
        }

        let mut filled_arguments = vec![];
        for i in 0..method.cpp_method.arguments.len() as i8 {
          if let Some(c_argument) = method.c_signature.arguments.iter().find(|x| {
            x.cpp_equivalent == CFunctionArgumentCppEquivalent::Argument(i)
          }) {
            let conversion_prefix = match c_argument.argument_type.conversion {
              CppToCTypeConversion::ValueToPointer | CppToCTypeConversion::ReferenceToPointer => {
                "*"
              }
              CppToCTypeConversion::NoConversion => "",
            };
            filled_arguments.push(format!("{}{}", conversion_prefix, c_argument.name));
          } else {
            panic!("Error: no positional argument found\n{:?}", method);
          }
        }


        write!(cpp_file,
               "({}){};\n",
               filled_arguments.into_iter().join(", "),
               return_type_conversion_suffix);
      }

      write!(cpp_file, "}}\n\n"); // method end





    }

    write!(h_file, "\nQTCW_EXTERN_C_END\n\n").unwrap();




    write!(h_file, "#endif // {}\n", include_guard_name).unwrap();
    println!("Done.\n")
  }
}
