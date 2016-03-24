use enums::CppTypeIndirection;
use c_type::CTypeExtended;
use enums::IndirectionChange;
use cpp_type_map::CppTypeMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppType {
  pub is_const: bool,
  pub indirection: CppTypeIndirection,
  pub base: String,
  pub template_arguments: Option<Vec<CppType>>,
}

impl CppType {
  pub fn is_template(&self) {
    self.template_arguments.is_some()
  }

  pub fn to_cpp_code(&self) -> String {
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
      CppTypeIndirection::Ptr_ref => "*&",
      CppTypeIndirection::Ptr_ptr => "**",
      CppTypeIndirection::Ref_ref => "&&",
    })
  }

  fn to_c_type(&self, cpp_type_map: &CppTypeMap) -> Option<CTypeExtended> {
    if self.is_template {
      return None;
    }
    let mut result = CTypeExtended::void();
    result.c_type.is_const = self.is_const;
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
    }


    // let mut aliased_primitive_types = HashMap::new();
    // aliased_primitive_types.insert("qint8", "int8_t");

    if let Some(info) = cpp_type_map.get_info(&self.base) {

    }

    if good_primitive_types.iter().find(|&x| x == &self.base).is_some() {
      result.is_primitive = true;
      result.c_type.base = self.base.clone();
    } else {
      result.c_type.base = self.base.clone();
      if result.c_type.base.find("::").is_some() {
        result.c_type.base = result.c_type.base.replace("::", "_");
        result.conversion.renamed = true;
      }
      result.c_type.is_pointer = true;
      if !self.is_pointer && !self.is_reference {
        result.conversion.indirection_change = IndirectionChange::ValueToPointer;
      }
    }
    Some(result)
  }

  fn is_stack_allocated_struct(&self) -> bool {
    !self.is_pointer && !self.is_reference && self.base.starts_with("Q")
  }
}
