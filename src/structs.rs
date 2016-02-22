

#[derive(Debug)]
pub struct CppType {
  pub is_template: bool,
  pub is_const: bool,
  pub is_reference: bool,
  pub is_pointer: bool,
  pub base: String,
}

#[derive(Debug)]
pub struct CppFunctionArgument {
  pub name: String,
  pub argument_type: CppType,
  pub default_value: Option<String>,
}

  #[derive(Debug)]
pub enum CppMethodScope {
  Global,
  Class
}

#[derive(Debug)]
pub struct CppMethod {
  pub name: String,
  pub scope: CppMethodScope,
  pub is_virtual: bool,
  pub is_const: bool,
  pub return_type: Option<CppType>,
  pub is_constructor: bool,
  pub is_destructor: bool,
  pub operator: Option<String>,
  pub is_variable: bool,
  pub arguments: Vec<CppFunctionArgument>,
  pub allows_variable_arguments: bool,
}

#[derive(Debug)]
pub struct CppHeaderData {
  pub include_file: String,
  pub class_name: Option<String>,
  pub methods: Vec<CppMethod>,
  pub macros: Vec<String>,
}
