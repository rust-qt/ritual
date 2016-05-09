
// use cpp_type_map::CppTypeInfo;
use cpp_method::CppMethod;
use cpp_type::CppType;
use cpp_type_map::EnumValue;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CLangClassField {
  pub name: String,
  pub field_type: CppType,
  pub is_protected: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CLangCppTypeKind {
  Enum {
    values: Vec<EnumValue>,
  },
  Class {
    bases: Vec<CppType>,
    fields: Vec<CLangClassField>,
    template_arguments: Option<Vec<String>>,
  },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CLangCppTypeData {
  pub name: String,
  pub header: String,
  pub kind: CLangCppTypeKind,
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CLangCppData {
  pub types: Vec<CLangCppTypeData>,
  pub methods: Vec<CppMethod>,
}
