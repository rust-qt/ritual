extern crate regex;

use std;
use std::path::PathBuf;
use cpp_data::CppData;
use std::io::Read;
use std::collections::HashMap;
use log;

pub fn fix_header_names(data: &mut CppData, headers_dir: &PathBuf) {
  let re = self::regex::Regex::new(r#"^#include "([a-zA-Z._]+)"$"#).unwrap();
  let mut map = HashMap::new();
  log::info("Detecting fancy Qt header names.");
  for header in std::fs::read_dir(headers_dir).unwrap() {
    let header = header.unwrap();
    let header_path = header.path();
    if std::fs::metadata(&header_path).unwrap().len() < 100 {
      let mut file = std::fs::File::open(&header_path).unwrap();
      let mut file_content = Vec::new();
      file.read_to_end(&mut file_content).unwrap();
      let file_content_string = String::from_utf8(file_content).unwrap().trim().to_string();
      if let Some(matches) = re.captures(file_content_string.as_ref()) {
        let real_header = matches.at(1).unwrap().to_string();
        let fancy_header = header.file_name().into_string().unwrap();
        if !map.contains_key(&real_header) {
          map.insert(real_header.clone(), Vec::new());
        }
        map.get_mut(&real_header).unwrap().push(fancy_header);
      }
    }
  }
  {
    let v = map["qsharedpointer.h"].clone();
    map.insert("qsharedpointer_impl.h".to_string(), v);
  }
  let mut map2 = HashMap::new();
  for (real_header, fancy_headers) in &map {
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
    map2.insert(real_header, fancy_header);
  }
  let get_header = |real_header: &String, class_name: Option<&String>| -> String {
    if let Some(class_name) = class_name {
      if let Some(fancy_headers) = map.get(real_header) {
        if let Some(x) = fancy_headers.iter()
          .find(|&x| x == class_name || class_name.starts_with(&format!("{}::", x))) {
          return x.clone();
        }
      }
    }
    if let Some(fancy_header) = map2.get(real_header) {
      return fancy_header.clone();
    }
    return real_header.clone();
  };

  for t in &mut data.types {
    t.include_file = get_header(&t.include_file, Some(&t.name));
  }
  for m in &mut data.methods {
    let x = get_header(&m.include_file, m.scope.class_name());
    m.include_file = x;
  }
}
