use structs::*;
use std::fs::File;
extern crate serde;
extern crate serde_json;
use self::serde::de::{Deserialize, Deserializer};
use self::serde::de::impls::BTreeMapVisitor;
use std;


impl CppType {
  fn from_json(value: &serde_json::Value) -> Self {
    let value = value.as_object().unwrap();
    CppType {
      is_template: match value.get("template") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      is_const: match value.get("const") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      is_reference: match value.get("reference") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      is_pointer: match value.get("pointer") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      base: value.get("base").unwrap().as_string().unwrap().to_string(),
    }
  }
}

impl CppFunctionArgument {
  fn from_json(value: &serde_json::Value) -> Self {
    let value = value.as_object().unwrap();
    CppFunctionArgument {
      name: value.get("name").unwrap().as_string().unwrap().to_string(),
      argument_type: CppType::from_json(value.get("type").unwrap()),
      default_value: match value.get("default_value") {
        Some(v) => Some(v.as_string().unwrap().to_string()),
        None => None,
      },
    }
  }
}

impl CppMethod {
  fn from_json(value: &serde_json::Value, class_name: &Option<String>) -> Self {
    //println!("{:?} {:?}", value, class_name);
    let value = value.as_object().unwrap();
    CppMethod {
      name: value.get("name").unwrap().as_string().unwrap().to_string(),
      scope: match value.get("scope").unwrap().as_string().unwrap() {
        "global" => CppMethodScope::Global,
        "class" => {
          match class_name {
            &Some(ref class_name) => CppMethodScope::Class(class_name.clone()),
            &None => panic!("invalid scope for global functions file"),
          }
        }
        _ => panic!("invalid scope"),
      },
      is_virtual: match value.get("virtual") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      is_const: match value.get("const") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      return_type: match value.get("return_type") {
        Some(v) => Some(CppType::from_json(v)),
        None => None,
      },
      is_constructor: match value.get("constructor") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      is_destructor: match value.get("destructor") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      operator: match value.get("operator") {
        Some(v) => Some(v.as_string().unwrap().to_string()),
        None => None,
      },
      is_variable: match value.get("variable") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      arguments: match value.get("arguments") {
        Some(v) => {
          v.as_array()
           .unwrap()
           .into_iter()
           .map(|x| CppFunctionArgument::from_json(x))
           .collect()
        }
        None => vec![],
      },
      allows_variable_arguments: match value.get("variable_arguments") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
    }
  }
}



impl CppHeaderData {
  fn from_json(value: &serde_json::Value) -> Self {

    let value = value.as_object().unwrap();
    let class_name = match value.get("class") {
      Some(s) => Some(s.as_string().unwrap().to_string()),
      None => None,
    };
    let methods = value.get("methods")
                       .unwrap()
                       .as_array()
                       .unwrap()
                       .into_iter()
                       .map(|x| CppMethod::from_json(x, &class_name))
                       .collect();
    CppHeaderData {
      include_file: value.get("include_file").unwrap().as_string().unwrap().to_string(),
      class_name: class_name,
      macros: match value.get("macros") {
        Some(data) => {
          data.as_array()
              .unwrap()
              .into_iter()
              .map(|x| x.as_string().unwrap().to_string())
              .collect()
        }
        None => vec![],
      },
      methods: methods,
    }
  }
}


pub fn do_it(file_name: &std::path::PathBuf) -> Vec<CppHeaderData> {
  let mut f = File::open(file_name).unwrap();
  let data: serde_json::Value = serde_json::from_reader(f).unwrap();
  data.as_array().unwrap().into_iter().map(|x| CppHeaderData::from_json(x)).collect()
  // for header in data.as_array().unwrap() {
  //  let header = header.as_object().unwrap();
  //  println!("test: {}", header.get("include_file").unwrap().as_string().unwrap());
  // }

}
