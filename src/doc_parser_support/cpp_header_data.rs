use cpp_method::{CppMethod, CppMethodScope, CppMethodKind};
use cpp_data::CppVisibility;

#[derive(Debug, Clone)]
pub struct CppHeaderData {
  pub include_file: String,
  pub class_name: Option<String>,
  pub methods: Vec<CppMethod>,
  pub macros: Vec<String>,
}


impl CppHeaderData {
  #[allow(dead_code)]
  pub fn ensure_explicit_destructor(&mut self) {
    if let Some(ref class_name) = self.class_name {
      if class_name == "QStandardPaths" {
        // destructor is private
        return;
      }
      if self.methods.iter().find(|x| x.kind == CppMethodKind::Destructor).is_none() {
        self.methods.push(CppMethod {
          name: format!("~{}", class_name),
          scope: CppMethodScope::Class(class_name.clone()),
          is_virtual: false,
          is_pure_virtual: false,
          is_const: false,
          is_static: false,
          visibility: CppVisibility::Public,
          is_signal: false,
          return_type: None,
          kind: CppMethodKind::Destructor,
          arguments: vec![],
          allows_variable_arguments: false,
          include_file: self.include_file.clone(),
          origin_location: None,
          template_arguments: None,
        });
      }
    }
  }
}
