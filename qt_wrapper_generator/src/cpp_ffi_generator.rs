use cpp_and_ffi_method::CppAndFfiMethod;
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use utils::JoinWithString;
use std::collections::HashMap;
use log;
use cpp_data::{CppData, CppTypeKind, CppVisibility};
use caption_strategy::MethodCaptionStrategy;
use cpp_method::{CppMethod, AllocationPlace, CppMethodScope};
use cpp_ffi_type::{IndirectionChange};
use cpp_ffi_function_argument::CppFfiArgumentMeaning;
use cpp_type::CppTypeBase;

pub struct CGenerator {
  qtcw_path: PathBuf,
  cpp_data: CppData,
  template_classes: Vec<String>,
  abstract_classes: Vec<String>,
}

impl CppAndFfiMethod {
  fn header_code(&self) -> String {
    format!("{} QTCW_EXPORT {}({});\n",
            self.c_signature.return_type.ffi_type.to_cpp_code().unwrap(),
            self.c_name,
            self.c_signature.arguments_to_cpp_code().unwrap())
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
                result = format!("new {}({})",
                                 return_type.base.to_cpp_code().unwrap(),
                                 result)
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
    if self.c_signature.return_type.conversion.qflags_to_uint {
      result = format!("uint({})", result);
    }

    if self.allocation_place == AllocationPlace::Stack && !self.cpp_method.is_constructor {
      if let Some(arg) = self.c_signature
                             .arguments
                             .iter()
                             .find(|x| x.meaning == CppFfiArgumentMeaning::ReturnValue) {
        if let Some(ref return_type) = self.cpp_method.return_type {
          result = format!("new({}) {}({})",
                           arg.name,
                           return_type.base.to_cpp_code().unwrap(),
                           result);
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
        x.meaning == CppFfiArgumentMeaning::Argument(i as i8)
      }) {
        let mut result = c_argument.name.clone();
        match c_argument.argument_type
                        .conversion
                        .indirection_change {
          IndirectionChange::ValueToPointer |
          IndirectionChange::ReferenceToPointer => result = format!("*{}", result),
          IndirectionChange::NoChange => {}
        }
        if c_argument.argument_type.conversion.qflags_to_uint {
          result = format!("{}({})",
                           cpp_argument.argument_type.to_cpp_code().unwrap(),
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
    self.convert_return_type(if self.cpp_method.is_destructor {
      if let Some(arg) = self.c_signature
                             .arguments
                             .iter()
                             .find(|x| x.meaning == CppFfiArgumentMeaning::This) {
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
                x.meaning == CppFfiArgumentMeaning::ReturnValue
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
            if let Some(arg) = self.c_signature
                                   .arguments
                                   .iter()
                                   .find(|x| x.meaning == CppFfiArgumentMeaning::This) {
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
                             .find(|x| x.meaning == CppFfiArgumentMeaning::This) {
        format!("delete {};\n", arg.name)
      } else {
        panic!("Error: no this argument found\n{:?}", self);
      }
    } else {
      format!("{}{};\n",
              if self.c_signature.return_type.ffi_type.is_void() {
                ""
              } else {
                "return "
              },
              self.returned_expression())
    }

  }

  fn source_code(&self) -> String {
    format!("{} {}({}) {{\n  {}}}\n\n",
            self.c_signature.return_type.ffi_type.to_cpp_code().unwrap(),
            self.c_name,
            self.c_signature.arguments_to_cpp_code().unwrap(),
            self.source_body())
  }
}

#[derive(Debug, Clone)]
pub struct CppFfiHeaderData {
  pub include_file: String,
  pub methods: Vec<CppAndFfiMethod>,
}

pub struct CppAndFfiData {
  pub cpp_data: CppData,
  pub cpp_data_by_headers: HashMap<String, CppData>,
  pub cpp_ffi_headers: Vec<CppFfiHeaderData>,
}

impl CGenerator {
  pub fn new(cpp_data: CppData, qtcw_path: PathBuf) -> Self {
    CGenerator {
      qtcw_path: qtcw_path,
      template_classes: cpp_data.types
                                .iter()
                                .filter_map(|t| {
                                  if let CppTypeKind::Class { ref template_arguments, .. } =
                                         t.kind {
                                    if template_arguments.is_some() {
                                      Some(t.name.clone())
                                    } else {
                                      None
                                    }
                                  } else {
                                    None
                                  }
                                })
                                .collect(),
      cpp_data: cpp_data,
      abstract_classes: Vec::new(),
    }
  }

  pub fn generate_all(mut self) -> CppAndFfiData {
    self.abstract_classes = self.cpp_data
                                .types
                                .iter()
                                .filter_map(|t| {
                                  if let CppTypeKind::Class { .. } = t.kind {
                                    if self.get_pure_virtual_methods(&t.name).len() > 0 {
                                      Some(t.name.clone())
                                    } else {
                                      None
                                    }
                                  } else {
                                    None
                                  }
                                })
                                .collect();
    log::info(format!("Abstract classes: {:?}", self.abstract_classes));
    let mut h_path = self.qtcw_path.clone();
    h_path.push("include");
    h_path.push("qtcw.h");
    let mut all_header_file = File::create(&h_path).unwrap();
    write!(all_header_file, "#ifndef QTCW_H\n#define QTCW_H\n\n").unwrap();

    let mut c_headers = Vec::new();
    let cpp_data_by_headers = self.cpp_data.split_by_headers();

    for (ref include_file, ref data) in &cpp_data_by_headers {
      c_headers.push(self.generate_one(include_file, data));
      write!(all_header_file, "#include \"qtcw_{}.h\"\n", include_file).unwrap();
    }

    write!(all_header_file, "#endif // QTCW_H\n").unwrap();
    CppAndFfiData {
      cpp_data: self.cpp_data,
      cpp_data_by_headers: cpp_data_by_headers,
      cpp_ffi_headers: c_headers,
    }

  }

  fn generate_one(&self, include_file: &String, data: &CppData) -> CppFfiHeaderData {
    log::info(format!("Generating C++ FFI methods for header: <{}>", include_file));
    let mut include_file_base_name = include_file.clone();
    if include_file_base_name.ends_with(".h") {
      include_file_base_name = include_file_base_name[0..include_file_base_name.len() - 2]
                                 .to_string();
    }
    let ffi_include_file = format!("qtcw_{}.h", include_file_base_name);

    let mut cpp_path = self.qtcw_path.clone();
    cpp_path.push("src");
    cpp_path.push(format!("qtcw_{}.cpp", include_file_base_name));
    log::info(format!("Generating source file: {:?}", cpp_path));

    let mut h_path = self.qtcw_path.clone();
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
    // write!(h_file, "#include <{}>\n", include_file).unwrap();
    write!(h_file, "#include <QtCore>\n").unwrap();
    write!(h_file, "#endif\n\n").unwrap();

    write!(h_file, "QTCW_EXTERN_C_BEGIN\n\n").unwrap();

    let methods: Vec<CppAndFfiMethod> = self.process_methods(&include_file_base_name,
                                                             &data.methods);
    for method in &methods {
      h_file.write(&method.header_code().into_bytes()).unwrap();
      cpp_file.write(&method.source_code().into_bytes()).unwrap();
    }

    write!(h_file, "\nQTCW_EXTERN_C_END\n\n").unwrap();

    write!(h_file, "#endif // {}\n", include_guard_name).unwrap();
    CppFfiHeaderData {
      include_file: include_file.clone(),
      methods: methods,
    }
  }

  #[allow(dead_code)]
  fn get_all_methods(&self, class_name: &String) -> Vec<CppMethod> {
    let own_methods: Vec<_> = self.cpp_data
                                  .methods
                                  .iter()
                                  .filter(|m| m.scope.class_name() == Some(class_name))
                                  .collect();
    let mut inherited_methods = Vec::new();
    if let Some(type_info) = self.cpp_data.types.iter().find(|t| &t.name == class_name) {
      if let CppTypeKind::Class { ref bases, .. } = type_info.kind {
        for base in bases {
          if let CppTypeBase::Class { ref name, .. } = base.base {
            for method in self.get_all_methods(name) {
              if own_methods.iter()
                            .find(|m| m.name == method.name && m.argument_types_equal(&method))
                            .is_none() {
                inherited_methods.push(method.clone());
              }
            }
          }
        }
      } else {
        panic!("get_all_methods: not a class");
      }
    } else {
      log::warning(format!("get_all_methods: no type info for {:?}", class_name));
    }
    for m in own_methods {
      inherited_methods.push((*m).clone());
    }
    inherited_methods
  }

  fn get_pure_virtual_methods(&self, class_name: &String) -> Vec<CppMethod> {

    let own_methods: Vec<_> = self.cpp_data
                                  .methods
                                  .iter()
                                  .filter(|m| m.scope.class_name() == Some(class_name))
                                  .collect();
    let own_pure_virtual_methods: Vec<_> = own_methods.iter()
                                                      .filter(|m| m.is_pure_virtual)
                                                      .collect();
    if class_name == "QStringListModel" {
      println!("OWN: {:?}", own_methods);
    }
    let mut inherited_methods = Vec::new();
    if let Some(type_info) = self.cpp_data.types.iter().find(|t| &t.name == class_name) {
      if let CppTypeKind::Class { ref bases, .. } = type_info.kind {
        for base in bases {
          if let CppTypeBase::Class { ref name, .. } = base.base {
            for method in self.get_pure_virtual_methods(name) {
              if class_name == "QStringListModel" {
                println!("INHERITED: {:?}", method);
              }
              if own_methods.iter()
                            .find(|m| m.name == method.name && m.argument_types_equal(&method))
                            .is_none() {
                if class_name == "QStringListModel" {
                  println!("not overriden!");
                }
                inherited_methods.push(method.clone());
              }
            }
          }
        }
      } else {
        panic!("get_pure_virtual_methods: not a class");
      }
    } else {
      log::warning(format!("get_pure_virtual_methods: no type info for {:?}",
                           class_name));
    }
    for m in own_pure_virtual_methods {
      inherited_methods.push((*m).clone());
    }
    inherited_methods
  }

  pub fn process_methods(&self,
                         include_file_base_name: &String,
                         methods: &Vec<CppMethod>)
                         -> Vec<CppAndFfiMethod> {

    //  if vec!["QAnimationGroup", "QAbstractListModel", "QAbstractTableModel"]
    //       .iter()
    //       .find(|&&x| x == self.include_file)
    //       .is_some() {
    //    // these class are abstract despite they don't have pure virtual methods!
    //    is_abstract_class = true;
    //  }

    let mut hash1 = HashMap::new();
    {
      let insert_into_hash = |hash: &mut HashMap<String, Vec<_>>, key: String, value| {
        if let Some(values) = hash.get_mut(&key) {
          values.push(value);
          return;
        }
        hash.insert(key, vec![value]);
      };

      for ref method in methods {
//        if include_file == "QRect" { println!("process_methods test1 {:?}", method); }
        if method.is_constructor {
          if let CppMethodScope::Class(ref class_name) = method.scope {
            if self.abstract_classes.iter().find(|x| x == &class_name).is_some() {
              log::debug(format!("Method is skipped:\n{}\nConstructors are not allowed for abstract \
                            classes.\n",
              method.short_text()));
              continue;
            }
          }
        }
        if method.visibility == CppVisibility::Private {
          continue;
        }
        if method.visibility == CppVisibility::Protected {
          log::debug(format!("Skipping protected method: \n{}\n",
          method.short_text()));
          continue;
        }
        if method.is_signal {
          log::warning(format!("Skipping signal: \n{}\n",
          method.short_text()));
          continue;
        }
        if method.template_arguments.is_some() {
          log::warning(format!("Skipping template method: \n{}\n",
          method.short_text()));
          continue;
        }
        if let CppMethodScope::Class(ref class_name) = method.scope {
          if self.template_classes
          .iter()
          .find(|x| x == &class_name || class_name.starts_with(&format!("{}::", x)))
          .is_some() {
            log::warning(format!("Skipping method of template class: \n{}\n",
            method.short_text()));
            continue;
          }
        }

        /*
        if self.include_file == "QMetaType" &&
           (method.name == "qRegisterMetaType" || method.name == "qRegisterMetaTypeStreamOperators" ||
            ((method.name == "hasRegisteredComparators" ||
              method.name == "hasRegisteredConverterFunction" ||
              method.name == "isRegistered" || method.name == "registerComparators" ||
              method.name == "registerConverter" ||
              method.name == "registerDebugStreamOperator" ||
              method.name == "registerEqualsComparator" || method.name == "qMetaTypeId" ||
              method.name == "hasRegisteredDebugStreamOperator") &&
             method.arguments.len() == 0)) {
          log::warning(format!("Method is skipped:\n{}\nThis method is blacklisted because it is \
                                a template method.\n",
                               method.short_text()));
          continue;
        }
        if self.include_file == "QMetaEnum" && method.name == "fromType" {
          log::warning(format!("Method is skipped:\n{}\nThis method is blacklisted because it is \
                                a template method.\n",
                               method.short_text()));
          continue;
        }
        if self.include_file == "QRectF" && method.scope == CppMethodScope::Global &&
           (method.name == "marginsAdded" || method.name == "marginsRemoved") {
          log::debug(format!("Method is skipped:\n{}\nThis method is blacklisted because it does \
                              not really exist.\n",
                             method.short_text()));
          continue;
        }
        // TODO: unblock on Windows
        if self.include_file == "QProcess" &&
           (method.name == "nativeArguments" || method.name == "setNativeArguments") {
          log::warning(format!("Method is skipped:\n{}\nThis method is Windows-only.\n",
                               method.short_text()));
          continue;
        }
        if self.include_file == "QAbstractEventDispatcher" &&
           (method.name == "registerEventNotifier" || method.name == "unregisterEventNotifier") {
          log::warning(format!("Method is skipped:\n{}\nThis method is Windows-only.\n",
                               method.short_text()));
          continue;
        }*/

        match method.add_c_signatures() {
          Err(msg) => {
            log::warning(format!("Unable to produce C function for method:\n{}\nError:{}\n",
            method.short_text(),
            msg));
          }
          Ok((result_heap, result_stack)) => {
            match result_heap.c_base_name(include_file_base_name) {
              Err(msg) => {
                log::warning(format!("Unable to produce C function for method:\n{}\nError:{}\n",
                method.short_text(),
                msg));
              }
              Ok(mut heap_name) => {
                if let Some(result_stack) = result_stack {
                  let mut stack_name = result_stack.c_base_name(include_file_base_name).unwrap();
                  if stack_name == heap_name {
                    stack_name = "SA_".to_string() + &stack_name;
                    heap_name = "HA_".to_string() + &heap_name;
                  }
                  insert_into_hash(&mut hash1, stack_name, result_stack);
                  insert_into_hash(&mut hash1, heap_name, result_heap);
                } else {
                  insert_into_hash(&mut hash1, heap_name, result_heap);
                }
              }
            }
          }
        }
      }
    }
    let mut r = Vec::new();
    for (key, mut values) in hash1.into_iter() {
      if values.len() == 1 {
        r.push(CppAndFfiMethod::new(values.remove(0), key.clone()));
        continue;
      }
      let mut found_strategy = None;
      for strategy in MethodCaptionStrategy::all() {
        let mut type_captions: Vec<_> = values.iter()
                                              .map(|x| x.caption(strategy.clone()))
                                              .collect();
        // println!("test1 {:?}", type_captions);
        type_captions.sort();
        type_captions.dedup();
        if type_captions.len() == values.len() {
          found_strategy = Some(strategy);
          break;
        }
      }
      if let Some(strategy) = found_strategy {
        for x in values {
          let caption = x.caption(strategy.clone());
          r.push(CppAndFfiMethod::new(x,
                                      format!("{}{}{}",
                                              key,
                                              if caption.is_empty() {
                                                ""
                                              } else {
                                                "_"
                                              },
                                              caption)));
        }
      } else {
        panic!("all type caption strategies have failed! Involved functions: \n{:?}",
               values);
      }
    }
    //TODO: make sorting
    //r.sort_by(|a, b| a.cpp_method.original_index.cmp(&b.cpp_method.original_index));
    // if include_file == "QRect" { println!("process_methods test2 {:?}", r); }
    r
  }
}
