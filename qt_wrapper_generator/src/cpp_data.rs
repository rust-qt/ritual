use cpp_header_data::CppHeaderData;
use cpp_type_map::CppTypeMap;
use cpp_type::{CppType, CppTypeBase};
use enums::{CppTypeOrigin, CppMethodScope};

#[derive(Debug, Clone)]
pub struct CppData {
  pub headers: Vec<CppHeaderData>,
  pub types: CppTypeMap,
  pub classes_blacklist: Vec<String>,
}

impl CppData {
  fn type_contains_template_arguments(&self, cpp_type: &CppType) -> Result<bool, String> {
    match cpp_type.base {
      CppTypeBase::Unspecified { ref name, ref template_arguments } => {
        match self.types.get_info(name) {
          Ok(ref info) => {
            if let CppTypeOrigin::Unsupported(ref v) = info.origin {
              if v == "template_argument" {
                return Ok(true);
              }
            }
          }
          Err(msg) => return Err(msg),
        }
        if let &Some(ref args) = template_arguments {
          for arg in args {
            match self.type_contains_template_arguments(&arg) {
              Ok(r) => {
                if r {
                  return Ok(true);
                }
              }
              Err(msg) => return Err(msg),
            }
          }
        }
        Ok(false)

      }
      _ => panic!("new cpp types are not supported here yet"),
    }
  }

  pub fn is_template_class(&self, class_name: &String) -> Result<bool, String> {
    if class_name == "QGlobalStatic" || class_name == "QFlags" {
      return Ok(true);
    }
    if class_name == "QVariant" || class_name == "QObject" {
      return Ok(false);
    }
    for item in &self.headers {
      if let Some(ref item_class_name) = item.class_name {
        if item_class_name == class_name {
          for method in &item.methods {
            if let CppMethodScope::Class(..) = method.scope {
              if let Some(ref return_type) = method.return_type {
                match self.type_contains_template_arguments(return_type) {
                  Ok(r) => {
                    if r {
                      return Ok(true);
                    }
                  }
                  Err(msg) => return Err(msg),
                }
              }
              for arg in &method.arguments {
                match self.type_contains_template_arguments(&arg.argument_type) {
                  Ok(r) => {
                    if r {
                      return Ok(true);
                    }
                  }
                  Err(msg) => return Err(msg),
                }
              }
            }
          }
          if let Some(index) = class_name.rfind("::") {
            match self.is_template_class(&class_name[0..index].to_string()) {
              Ok(r) => {
                if r {
                  return Ok(true);
                }
              }
              Err(msg) => return Err(msg),
            }
          }
          return Ok(false);
        }
      }
    }
    Err("Corresponding header not found".to_string())
  }
}
