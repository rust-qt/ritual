use cpp_method::CppMethod;
use enums::{CppMethodScope, CppTypeOrigin};
use cpp_and_c_method::CppAndCMethod;
use caption_strategy::MethodCaptionStrategy;
use cpp_type_map::CppTypeMap;
use std::collections::HashMap;
use log;

#[derive(Debug, Clone)]
pub struct CppHeaderData {
  pub include_file: String,
  pub class_name: Option<String>,
  pub methods: Vec<CppMethod>,
  pub macros: Vec<String>,
}


impl CppHeaderData {
  pub fn ensure_explicit_destructor(&mut self) {
    if let Some(ref class_name) = self.class_name {
      if class_name == "QStandardPaths" {
        // destructor is private
        return;
      }
      if self.methods.iter().find(|x| x.is_destructor).is_none() {
        self.methods.push(CppMethod {
          name: format!("~{}", class_name),
          scope: CppMethodScope::Class(class_name.clone()),
          is_virtual: false, // TODO: destructors may be virtual
          is_pure_virtual: false,
          is_const: false,
          is_static: false,
          is_protected: false,
          is_signal: false,
          return_type: None,
          is_constructor: false,
          is_destructor: true,
          operator: None,
          is_variable: false,
          arguments: vec![],
          allows_variable_arguments: false,
          original_index: 1000,
          origin: CppTypeOrigin::Qt { include_file: self.include_file.clone() },
          template_arguments: None,
        });
      }
    }
  }

  pub fn process_methods(&self, cpp_type_map: &CppTypeMap) -> Vec<CppAndCMethod> {
    log::info(format!("Converting methods from C++ to C: header <{}>",
                      self.include_file));
    let mut is_abstract_class = false;
    for ref method in &self.methods {
      if method.is_pure_virtual {
        is_abstract_class = true;
        break;
      }
    }
    if vec!["QAnimationGroup", "QAbstractListModel", "QAbstractTableModel"]
         .iter()
         .find(|&&x| x == self.include_file)
         .is_some() {
      // these class are abstract despite they don't have pure virtual methods!
      is_abstract_class = true;
    }

    let mut hash1 = HashMap::new();
    {
      let insert_into_hash = |hash: &mut HashMap<String, Vec<_>>, key: String, value| {
        if let Some(values) = hash.get_mut(&key) {
          values.push(value);
          return;
        }
        hash.insert(key, vec![value]);
      };

      for ref method in &self.methods {
        if is_abstract_class && method.is_constructor {
          log::debug(format!("Method is skipped:\n{}\nConstructors are not allowed for \
                              abstract classes.\n",
                             method.short_text()));
          continue;
        }
        if self.include_file == "QMetaType" &&
           (method.name == "qRegisterMetaType" ||
            method.name == "qRegisterMetaTypeStreamOperators" ||
            ((method.name == "hasRegisteredComparators" ||
              method.name == "hasRegisteredConverterFunction" ||
              method.name == "isRegistered" || method.name == "registerComparators" ||
              method.name == "registerConverter" ||
              method.name == "registerDebugStreamOperator" ||
              method.name == "registerEqualsComparator" ||
              method.name == "qMetaTypeId" ||
              method.name == "hasRegisteredDebugStreamOperator") &&
             method.arguments.len() == 0)) {
          log::warning(format!("Method is skipped:\n{}\nThis method is blacklisted because it \
                                is a template method.\n",
                               method.short_text()));
          continue;
        }
        if self.include_file == "QMetaEnum" && method.name == "fromType" {
          log::warning(format!("Method is skipped:\n{}\nThis method is blacklisted because it \
                                is a template method.\n",
                               method.short_text()));
          continue;
        }
        if self.include_file == "QRectF" && method.scope == CppMethodScope::Global &&
           (method.name == "marginsAdded" || method.name == "marginsRemoved") {
          log::debug(format!("Method is skipped:\n{}\nThis method is blacklisted because it \
                              does not really exist.\n",
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
        }

        match method.add_c_signatures(cpp_type_map) {
          Err(msg) => {
            log::warning(format!("Unable to produce C function for method:\n{}\nError:{}\n",
                                 method.short_text(),
                                 msg));
          }
          Ok((result_heap, result_stack)) => {
            match result_heap.c_base_name() {
              Err(msg) => {
                log::warning(format!("Unable to produce C function for method:\n{}\nError:{}\n",
                                     method.short_text(),
                                     msg));
              }
              Ok(mut heap_name) => {
                if let Some(result_stack) = result_stack {
                  let mut stack_name = result_stack.c_base_name().unwrap();
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
    let name_prefix = match self.class_name {
      Some(ref class_name) => class_name.replace("::", "_"),
      None => self.include_file.clone(),
    };
    for (key, mut values) in hash1.into_iter() {
      if values.len() == 1 {
        r.push(CppAndCMethod::new(values.remove(0), format!("{}_{}", name_prefix, key)));
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
          r.push(CppAndCMethod::new(x,
                                    format!("{}_{}{}{}",
                                            name_prefix,
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
    r.sort_by(|a, b| a.cpp_method.original_index.cmp(&b.cpp_method.original_index));
    r
  }
}
