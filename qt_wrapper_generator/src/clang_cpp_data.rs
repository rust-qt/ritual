
use cpp_type_map::CppTypeInfo;
use cpp_method::CppMethod;

pub struct CLangCppData {
  pub types: Vec<CppTypeInfo>,
  pub methods: Vec<CppMethod>
}
