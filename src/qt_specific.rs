extern crate regex;

use std;
use std::path::PathBuf;
use cpp_data::CppData;
use std::io::Read;
use std::collections::HashMap;
use log;
use utils::add_to_multihash;

pub fn fix_header_names(data: &mut CppData, headers_dir: &PathBuf) {
  let re = self::regex::Regex::new(r#"^#include "([a-zA-Z._]+)"$"#).unwrap();
  let mut map_real_to_all_fancy: HashMap<_, Vec<_>> = HashMap::new();
  log::info("Detecting fancy Qt header names.");
  for header in std::fs::read_dir(headers_dir).unwrap() {
    let header = header.unwrap();
    let header_path = header.path();
    if !header_path.is_file() {
      continue;
    }
    if std::fs::metadata(&header_path).unwrap().len() < 100 {
      let mut file = std::fs::File::open(&header_path).unwrap();
      let mut file_content = Vec::new();
      file.read_to_end(&mut file_content).unwrap();
      let file_content_string = String::from_utf8(file_content).unwrap().trim().to_string();
      if let Some(matches) = re.captures(file_content_string.as_ref()) {
        let real_header = matches.at(1).unwrap().to_string();
        let fancy_header = header.file_name().into_string().unwrap();
        add_to_multihash(&mut map_real_to_all_fancy, &real_header, fancy_header);
      }
    }
  }
  if map_real_to_all_fancy.contains_key("qsharedpointer.h") {
    let v = map_real_to_all_fancy["qsharedpointer.h"].clone();
    map_real_to_all_fancy.insert("qsharedpointer_impl.h".to_string(), v);
  }
  let mut map_real_to_fancy = HashMap::new();
  for (real_header, fancy_headers) in &map_real_to_all_fancy {
    let fancy_header = if fancy_headers.len() == 1 {
      fancy_headers[0].clone()
    } else {
      let mut result = fancy_headers[0].clone();
      let mut ok = false;
      for h in fancy_headers {
        if format!("{}.h", h.to_lowercase()) == *real_header {
          result = h.clone();
          ok = true;
          break;
        }
      }
      if !ok {
        log::noisy(format!("{} -> {:?} (detect failed)", real_header, fancy_headers));
      }
      result
    };
    log::noisy(format!("{} -> {}", real_header, fancy_header));
    map_real_to_fancy.insert(real_header, fancy_header);
  }
  let get_header = |real_header: &String, class_name: Option<&String>| -> String {
    if let Some(class_name) = class_name {
      if let Some(fancy_headers) = map_real_to_all_fancy.get(real_header) {
        if let Some(x) = fancy_headers.iter()
          .find(|&x| x == class_name || class_name.starts_with(&format!("{}::", x))) {
          return x.clone();
        }
      }
    }
    if let Some(fancy_header) = map_real_to_fancy.get(real_header) {
      return fancy_header.clone();
    }
    return real_header.clone();
  };

  for t in &mut data.types {
    t.include_file = get_header(&t.include_file, Some(&t.name));
  }
  for m in &mut data.methods {
    let x = get_header(&m.include_file, m.class_name());
    m.include_file = x;
  }
  for t in &mut data.template_instantiations {
    t.include_file = get_header(&t.include_file, Some(&t.class_name));
  }
}

// TODO: save header mapping and use in dependencies
// (e.g. qflags.h instead of QFlags in QtGui is not good)
