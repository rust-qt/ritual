use cpp_type::{CppType, CppTypeBase, CppTypeIndirection};
use doc_parser_support::cpp_type_map::DocCppTypeOrigin;
use doc_parser_support::cpp_type_map::{CppTypeInfo, CppTypeMap};
use cpp_data::{EnumValue, CppVisibility};
use cpp_method::{CppFunctionArgument, CppMethod, CppMethodScope, CppMethodKind};
use doc_parser_support::cpp_header_data::CppHeaderData;
use doc_parser_support::cpp_data::DocCppData;
use doc_parser_support::enums::DocCppTypeKind;


use std::fs::File;
extern crate serde;
extern crate serde_json;
use std;


impl CppType {
  fn from_json(value: &serde_json::Value) -> Self {
    let value = value.as_object().unwrap();
    CppType {
      is_const: match value.get("is_const") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      indirection: match value.get("indirection") {
        Some(v) => {
          match v.as_string().unwrap() {
            "*" => CppTypeIndirection::Ptr,
            "&" => CppTypeIndirection::Ref,
            "&&" => CppTypeIndirection::RValueRef,
            "*&" => CppTypeIndirection::PtrRef,
            "**" => CppTypeIndirection::PtrPtr,
            _ => panic!("unknown indirection string"),
          }
        }
        None => CppTypeIndirection::None,
      },
      base: CppTypeBase::Unspecified {
        name: value.get("base").unwrap().as_string().unwrap().to_string(),
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
      },
    }
  }
}

impl CppFunctionArgument {
  fn from_json(value: &serde_json::Value) -> Self {
    let value = value.as_object().unwrap();
    CppFunctionArgument {
      name: value.get("name").unwrap().as_string().unwrap().to_string(),
      argument_type: CppType::from_json(value.get("type").unwrap()),
      has_default_value: value.get("default_value").is_some(),
    }
  }
}

impl CppMethod {
  #[allow(unused_variables)]
  fn from_json(value: &serde_json::Value,
               include_file: &String,
               class_name: &Option<String>,
               index: i32)
               -> Self {
    let value = value.as_object().unwrap();
    CppMethod {
      include_file: include_file.clone(),
      origin_location: None,
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
      is_pure_virtual: match value.get("pure_virtual") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      visibility: match value.get("protected") {
        Some(v) => {
          if v.as_boolean().unwrap() {
            CppVisibility::Protected
          } else {
            CppVisibility::Public
          }
        }
        None => CppVisibility::Public,
      },
      is_signal: match value.get("signal") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      is_const: match value.get("is_const") {
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
      kind: CppMethodKind::Regular,
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
      allows_variadic_arguments: match value.get("variable_arguments") {
        Some(v) => v.as_boolean().unwrap(),
        None => false,
      },
      template_arguments: None,
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
    let include_file = value.get("include_file").unwrap().as_string().unwrap().to_string();
    let methods = value.get("methods")
      .unwrap()
      .as_array()
      .unwrap()
      .into_iter()
      .enumerate()
      .map(|(index, x)| CppMethod::from_json(x, &include_file, &class_name, index as i32))
      .collect();
    CppHeaderData {
      include_file: include_file,
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
      value: 0,
    }
  }
}

impl CppTypeInfo {
  fn from_json(value: &serde_json::Value, name: String) -> Self {
    let value = value.as_object().unwrap();
    let origin = match value.get("origin").unwrap().as_string().unwrap() {
      "c_built_in" => DocCppTypeOrigin::CBuiltIn,
      "qt" => {
        DocCppTypeOrigin::IncludeFile {
          include_file: value.get("qt_header").unwrap().as_string().unwrap().to_string(),
        }
      }
      _ => DocCppTypeOrigin::Unknown,
    };
    CppTypeInfo {
      name: name,
      origin: origin.clone(),
      kind: if origin == DocCppTypeOrigin::CBuiltIn {
        DocCppTypeKind::CPrimitive
      } else {
        match value.get("kind") {
          Some(v) => {
            match v.as_string().unwrap() {
              "enum" => {
                DocCppTypeKind::Enum {
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
                DocCppTypeKind::Flags {
                  enum_name: value.get("enum").unwrap().as_string().unwrap().to_string(),
                }
              }
              "typedef" => {
                match value.get("meaning") {
                  Some(v) => DocCppTypeKind::TypeDef { meaning: CppType::from_json(v) },
                  None => DocCppTypeKind::Unknown,
                }
              }
              "class" => {
                DocCppTypeKind::Class {
                  inherits: match value.get("inherits") {
                    Some(inherits) => Some(CppType::from_json(inherits)),
                    None => None,
                  },
                }
              }
              "template_type" => DocCppTypeKind::Unknown,
              _ => panic!("invalid kind of type"),
            }
          }
          None => DocCppTypeKind::Unknown,
        }
      },
    }
  }
}

impl CppTypeMap {
  fn from_json(value: &serde_json::Value) -> Self {
    let value = value.as_object().unwrap();
    CppTypeMap(value.into_iter()
      .map(|(k, v)| (k.clone(), CppTypeInfo::from_json(v, k.clone())))
      .collect())
  }
}

pub fn do_it(file_name: &std::path::PathBuf) -> DocCppData {
  let f = File::open(file_name).unwrap();
  let data: serde_json::Value = serde_json::from_reader(f).unwrap();
  let object = data.as_object().unwrap();
  DocCppData {
    headers: object.get("headers_data")
      .unwrap()
      .as_array()
      .unwrap()
      .into_iter()
      .map(|x| CppHeaderData::from_json(x))
      .collect(),
    types: CppTypeMap::from_json(object.get("type_info").unwrap()),
    classes_blacklist: vec![],
  }
}
