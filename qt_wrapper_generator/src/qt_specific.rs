extern crate regex;

use std;
use std::path::PathBuf;
use clang_cpp_data::CLangCppData;
use std::io::Read;
use std::collections::HashMap;
use log;

pub fn fix_header_names(data: &mut CLangCppData, headers_dir: &PathBuf) {
  let re = self::regex::Regex::new(r#"^#include "([a-zA-Z._]+)"$"#).unwrap();
  let mut map = HashMap::new();
  log::info("Detecting fancy Qt header names.");
  for header in std::fs::read_dir(headers_dir).unwrap() {
    // println!("Name: {}", header.unwrap().path().display());
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
        // if format!("{}.h", fancy_header.to_lowercase()) != map[&real_header] {
        map.get_mut(&real_header).unwrap().push(fancy_header);
      }
    }
  }
  {
    let v = map["qsharedpointer.h"].clone();
    map.insert("qsharedpointer_impl.h".to_string(), v);
  }
  let mut map2 = HashMap::new();
  for (real_header, mut fancy_headers) in &map {
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
        log::info(format!("{} -> {:?} (detect failed)", real_header, fancy_headers));
      }
      result
    };
    log::info(format!("{} -> {}", real_header, fancy_header));
    map2.insert(real_header, fancy_header);
  }
  for t in &mut data.types {
    if let Some(fancy_headers) = map.get(&t.header) {
      if let Some(x) = fancy_headers.iter().find(|&x| {
        x == &t.name || t.name.starts_with(&format!("{}::", x))
      }) {
        t.header = x.clone();
        continue;
      }
    }
    if let Some(fancy_header) = map2.get(&t.header) {
      t.header = fancy_header.clone();
    }
  }
}
