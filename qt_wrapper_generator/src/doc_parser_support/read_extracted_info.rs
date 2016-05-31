use std::path::PathBuf;
use std::collections::HashMap;
use std::fs::File;

extern crate serde;
extern crate serde_json;

#[derive(Debug, Clone)]
pub struct CppExtractedInfo {
  pub class_sizes: HashMap<String, i32>,
  pub enum_values: HashMap<String, HashMap<String, i32>>,
}

#[allow(dead_code)]
pub fn do_it(file_name: PathBuf) -> CppExtractedInfo {
  let f = File::open(file_name).unwrap();
  let data_value: serde_json::Value = serde_json::from_reader(f).unwrap();
  let data = data_value.as_object().unwrap();
  CppExtractedInfo {
    class_sizes: data.get("class_sizes")
                     .unwrap()
                     .as_object()
                     .unwrap()
                     .into_iter()
                     .map(|(k, v)| (k.clone(), v.as_i64().unwrap() as i32))
                     .collect(),
    enum_values: data.get("enum_values")
                     .unwrap()
                     .as_object()
                     .unwrap()
                     .into_iter()
                     .map(|(k, v)| {
                       (k.clone(),
                        v.as_object()
                         .unwrap()
                         .into_iter()
                         .map(|(k, v)| (k.clone(), v.as_i64().unwrap() as i32))
                         .collect())
                     })
                     .collect(),
  }
}
