use cpp_type::CppType;
use cpp_type_map::{EnumValue, CppTypeInfo, CppTypeMap};
use cpp_method::{CppFunctionArgument, CppMethod};
use cpp_header_data::CppHeaderData;
use cpp_data::CppData;
use enums::{CppMethodScope, CppTypeOrigin, CppTypeKind, CppTypeIndirection};

use std::fs::File;
extern crate serde;
extern crate serde_json;
use std;


impl CppType {
  fn from_json(value: &serde_json::Value) -> Self {
    let value = value.as_object().unwrap();
    CppType {
      template_arguments: match value.get("template_arguments") {
        Some(v) => {
          Some(v.as_array()
                .unwrap()
                .into_iter()
                .map(|x| CppType::from_json(x))
                .collect())
        }
        None => None,
      },
      is_const: match value.get("const") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      indirection: match value.get("indirection") {
        Some(v) => match v.as_string().unwrap() {
          "*" => CppTypeIndirection::Ptr,
          "&" => CppTypeIndirection::Ref,
          "&&" => CppTypeIndirection::RefRef,
          "*&" => CppTypeIndirection::PtrRef,
          "**" => CppTypeIndirection::PtrPtr,
          _ => panic!("unknown indirection string")
        },
        None => CppTypeIndirection::None,
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
  fn from_json(value: &serde_json::Value, class_name: &Option<String>, index: i32) -> Self {
    // println!("{:?} {:?}", value, class_name);
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
      is_static: match value.get("static") {
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
      original_index: index,
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
                       .enumerate()
                       .map(|(index, x)| CppMethod::from_json(x, &class_name, index as i32))
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

impl EnumValue {
  fn from_json(value: &serde_json::Value) -> Self {
    let value = value.as_object().unwrap();
    EnumValue {
      name: value.get("name").unwrap().as_string().unwrap().to_string(),
      value: value.get("value").unwrap().as_string().unwrap().to_string(),
      description: value.get("description").unwrap().as_string().unwrap().to_string()
    }
  }
}

impl CppTypeInfo {
  fn from_json(value: &serde_json::Value, name: String) -> Self {
    let value = value.as_object().unwrap();
    let origin = match value.get("origin").unwrap().as_string().unwrap() {
      "c_built_in" => CppTypeOrigin::CBuiltIn,
      "qt" => {
        CppTypeOrigin::Qt {
          include_file: value.get("qt_header").unwrap().as_string().unwrap().to_string(),
        }
      }
      other => CppTypeOrigin::Unsupported(other.to_string()),
    };
    CppTypeInfo {
      name: name,
      origin: origin.clone(),
      kind: if origin == CppTypeOrigin::CBuiltIn {
        CppTypeKind::CPrimitive
      } else {
        match value.get("kind").unwrap().as_string().unwrap() {
          "enum" => {
            CppTypeKind::Enum {
              values: value.get("values")
                           .unwrap()
                           .as_array()
                           .unwrap()
                           .into_iter()
                           .map(|x| EnumValue::from_json(x))
                           .collect(),
            }
          }
          "flags" => {
            CppTypeKind::Flags {
              enum_name: value.get("enum").unwrap().as_string().unwrap().to_string(),
            }
          }
          "typedef" => {
            CppTypeKind::TypeDef { meaning: CppType::from_json(value.get("meaning").unwrap()) }
          }
          "class" => {
            CppTypeKind::Class {
              inherits: match value.get("inherits") {
                Some(inherits) => Some(CppType::from_json(inherits)),
                None => None,
              },
            }
          }
          _ => panic!("invalid kind of type"),
        }
      },
    }
  }
}

impl CppTypeMap {
  fn from_json(value: &serde_json::Value) -> Self {
    let value = value.as_object().unwrap();
    CppTypeMap(value.into_iter().map(|(k, v)| (k.clone(), CppTypeInfo::from_json(v, k.clone()))).collect())
  }
}

pub fn do_it(file_name: &std::path::PathBuf) -> CppData {
  let f = File::open(file_name).unwrap();
  let data: serde_json::Value = serde_json::from_reader(f).unwrap();
  let object = data.as_object().unwrap();
  CppData {
    headers: object.get("headers_data")
                   .unwrap()
                   .as_array()
                   .unwrap()
                   .into_iter()
                   .map(|x| CppHeaderData::from_json(x))
                   .collect(),
    types: CppTypeMap::from_json(object.get("type_info").unwrap()),
  }
}
