use std::collections::{HashSet, HashMap};
use log;
use cpp_data::{CppData, CppTypeKind, CppVisibility};
use caption_strategy::MethodCaptionStrategy;
use cpp_method::{CppMethod, CppMethodKind};
use cpp_ffi_data::{CppAndFfiMethod, c_base_name};
use utils::add_to_multihash;
use serializable::CppLibSpec;

struct CGenerator<'a> {
  abstract_classes: Vec<String>,
  cpp_data: &'a CppData,
  cpp_lib_spec: CppLibSpec,
}

#[derive(Debug, Clone)]
pub struct CppFfiHeaderData {
  pub include_file: String,
  pub include_file_base_name: String,
  pub methods: Vec<CppAndFfiMethod>,
}

pub struct CppAndFfiData {
  pub cpp_data: CppData,
  pub cpp_ffi_headers: Vec<CppFfiHeaderData>,
}

/// Runs FFI generator
pub fn run(cpp_data: &CppData, cpp_lib_spec: CppLibSpec) -> Vec<CppFfiHeaderData> {
  let abstract_classes = cpp_data.types
    .iter()
    .filter_map(|t| {
      if let CppTypeKind::Class { .. } = t.kind {
        if cpp_data.get_pure_virtual_methods(&t.name).len() > 0 {
          Some(t.name.clone())
        } else {
          None
        }
      } else {
        None
      }
    })
    .collect();
  log::info(format!("Abstract classes: {:?}", abstract_classes));
  let generator = CGenerator {
    cpp_data: cpp_data,
    abstract_classes: abstract_classes,
    cpp_lib_spec: cpp_lib_spec,
  };

  let mut c_headers = Vec::new();
  let mut include_name_list: Vec<_> = generator.cpp_data.all_include_files().into_iter().collect();
  include_name_list.sort();

  for include_file in &include_name_list {
    if let Some(ref include_file_blacklist) = generator.cpp_lib_spec.include_file_blacklist {
      if include_file_blacklist.iter().find(|x| x == &include_file).is_some() {
        log::info(format!("Skipping include file {}", include_file));
        continue;
      }
    }
    let mut include_file_base_name = include_file.clone();

    if let Some(index) = include_file_base_name.find(".") {
      include_file_base_name = include_file_base_name[0..index].to_string();
    }
    let methods = generator.process_methods(&include_file,
                                            &include_file_base_name,
                                            generator.cpp_data
                                              .methods
                                              .iter()
                                              .filter(|x| &x.include_file == include_file));
    c_headers.push(CppFfiHeaderData {
      include_file: include_file.clone(),
      include_file_base_name: include_file_base_name,
      methods: methods,
    });
  }
  c_headers
}

impl<'a> CGenerator<'a> {
  /// Returns false if the method is excluded from processing
  /// for some reason
  fn should_process_method(&self, method: &CppMethod) -> bool {
    let full_name = method.full_name();
    let short_text = method.short_text();
    let class_name = method.class_name().unwrap_or(&String::new()).clone();
    if let Some(ref ffi_methods_blacklist) = self.cpp_lib_spec.ffi_methods_blacklist {
      if ffi_methods_blacklist.iter()
        .find(|&x| x == &full_name || x == &short_text || x == &class_name)
        .is_some() {
        log::noisy(format!("Skipping blacklisted method: \n{}\n", method.short_text()));
        return false;
      }
    }
    if let Some(ref membership) = method.class_membership {
      if membership.kind == CppMethodKind::Constructor {
        let class_name = &membership.class_type.name;
        if self.abstract_classes.iter().find(|&x| x == class_name).is_some() {
          log::noisy(format!("Method is skipped:\n{}\nConstructors are not allowed for abstract \
                              classes.\n",
                             method.short_text()));
          return false;
        }
      }
      if membership.visibility == CppVisibility::Private {
        return false;
      }
      if membership.visibility == CppVisibility::Protected {
        log::noisy(format!("Skipping protected method: \n{}\n", method.short_text()));
        return false;
      }
      if membership.is_signal {
        log::warning(format!("Skipping signal: \n{}\n", method.short_text()));
        return false;
      }
    }
    if method.template_arguments.is_some() {
      // TODO: check findChildren and similar instantiated methods
      log::noisy(format!("Skipping template method: \n{}\n", method.short_text()));
      return false;
    }
    if method.all_involved_types()
      .iter()
      .find(|x| x.base.is_or_contains_template_parameter())
      .is_some() {
      log::noisy(format!("Skipping method containing template parameters: \n{}\n",
                         method.short_text()));
      return false;
    }
    true
  }

  /// Generates FFI wrappers for all specified methods,
  /// resolving all name conflicts using additional method captions.
  fn process_methods<'b, I: Iterator<Item = &'b CppMethod>>(&self,
                                                            include_file: &String,
                                                            include_file_base_name: &String,
                                                            methods: I)
                                                            -> Vec<CppAndFfiMethod> {
    log::info(format!("Generating C++ FFI methods for header: {}", include_file));
    let mut hash_name_to_methods: HashMap<String, Vec<_>> = HashMap::new();
    for ref method in methods {
      if !self.should_process_method(method) {
        continue;
      }
      match method.to_ffi_signatures() {
        Err(msg) => {
          log::warning(format!("Unable to produce C function for method:\n{}\nError:{}\n",
                               method.short_text(),
                               msg));
        }
        Ok(results) => {
          for result in results {
            match c_base_name(&result.cpp_method,
                              &result.allocation_place,
                              include_file_base_name) {
              Err(msg) => {
                log::warning(format!("Unable to produce C function for method:\n{}\nError:{}\n",
                                     method.short_text(),
                                     msg));
              }
              Ok(name) => {
                add_to_multihash(&mut hash_name_to_methods, &name, result);
              }
            }
          }
        }
      }
    }

    let mut processed_methods = Vec::new();
    for (key, mut values) in hash_name_to_methods.into_iter() {
      if values.len() == 1 {
        processed_methods.push(CppAndFfiMethod::new(values.remove(0), key.clone()));
        continue;
      }
      let mut found_strategy = None;
      for strategy in MethodCaptionStrategy::all() {
        let mut type_captions: HashSet<_> = HashSet::new();
        let mut ok = true;
        for value in &values {
          let caption = value.c_signature.caption(strategy.clone());
          if type_captions.contains(&caption) {
            ok = false;
            break;
          }
          type_captions.insert(caption);
        }
        if ok {
          found_strategy = Some(strategy);
          break;
        }
      }
      if let Some(strategy) = found_strategy {
        for x in values {
          let caption = x.c_signature.caption(strategy.clone());
          let final_name = if caption.is_empty() {
            key.clone()
          } else {
            format!("{}_{}", key, caption)
          };
          processed_methods.push(CppAndFfiMethod::new(x, final_name));
        }
      } else {
        log::error(format!("values dump: {:?}\n", values));
        log::error(format!("All type caption strategies have failed! Involved functions:"));
        for value in values {
          log::error(format!("  {}", value.cpp_method.short_text()));
        }
        panic!("all type caption strategies have failed");
      }
    }
    processed_methods.sort_by(|a, b| a.c_name.cmp(&b.c_name));
    processed_methods
  }
}
