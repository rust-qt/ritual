use enums::CppTypeIndirection;
use c_type::CTypeExtended;
use enums::{IndirectionChange, CppTypeKind};
use cpp_type_map::CppTypeMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppType {
  pub is_const: bool,
  pub indirection: CppTypeIndirection,
  pub base: String,
  pub template_arguments: Option<Vec<CppType>>,
}

impl CppType {
  pub fn void() -> Self {
    CppType {
      is_const: false,
      indirection: CppTypeIndirection::None,
      base: "void".to_string(),
      template_arguments: None,
    }
  }

  pub fn is_template(&self) -> bool {
    self.template_arguments.is_some()
  }

  pub fn to_cpp_code(&self) -> String {
    if self.is_template() {
      panic!("template types are not supported yet")
    }
    format!("{}{}{}",
            if self.is_const {
              "const "
            } else {
              ""
            },
            self.base,
            match self.indirection {
              CppTypeIndirection::None => "",
              CppTypeIndirection::Ptr => "*",
              CppTypeIndirection::Ref => "&",
              CppTypeIndirection::PtrRef => "*&",
              CppTypeIndirection::PtrPtr => "**",
              CppTypeIndirection::RefRef => "&&",
            })
  }

  pub fn to_c_type(&self, cpp_type_map: &CppTypeMap) -> Result<CTypeExtended, String> {
    if self.is_template() {
      return Err("Template types are not supported yet".to_string());
    }
    // todo: refactor this (it's easy to forgot initialization of result fields)
    let mut result = CTypeExtended::void();
    result.c_type.is_const = self.is_const;
    result.cpp_type = self.clone();
    match self.indirection {
      CppTypeIndirection::None => {
        // "const Rect" return type should not be translated to const pointer
        result.c_type.is_const = false;
      }
      CppTypeIndirection::Ptr => {
        result.c_type.is_pointer = true;

      }
      CppTypeIndirection::Ref => {
        result.c_type.is_pointer = true;
        result.conversion.indirection_change = IndirectionChange::ReferenceToPointer;
      }
      _ => return Err("Unsupported level of indirection".to_string()),
    }

    match cpp_type_map.get_info(&self.base) {
      Ok(info) => {
        match info.kind {
          CppTypeKind::TypeDef { .. } => panic!("cpp_type_map.get_info should not return typedef"),
          CppTypeKind::CPrimitive | CppTypeKind::Enum { .. } => {
            result.c_type.base = self.base.clone();
          }
          CppTypeKind::Flags { .. } => {
            result.c_type.base = format!("QTCW_{}", self.base.replace("::", "_"));
            result.conversion.qflags_to_uint = true;
          }
          CppTypeKind::Class { .. } => {
            result.c_type.base = self.base.clone();
            result.c_type.is_pointer = true;
            if self.indirection == CppTypeIndirection::None {
              result.conversion.indirection_change = IndirectionChange::ValueToPointer;
            }
          }
        }
      }
      Err(msg) => return Err(format!("Type info error for {:?}: {}", self, msg)),
    }
    if result.c_type.base.find("::").is_some() {
      result.c_type.base = result.c_type.base.replace("::", "_");
      result.conversion.renamed = true;
    }
    Ok(result)
  }
}
