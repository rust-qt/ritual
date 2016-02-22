

#[derive(Debug, Clone)]
pub struct CppType {
  pub is_template: bool,
  pub is_const: bool,
  pub is_reference: bool,
  pub is_pointer: bool,
  pub base: String,
}

#[derive(Debug)]
pub struct CType {
  pub is_pointer: bool,
  pub base: String,
}

impl CppType {
  fn to_c_type(&self) -> Option<CType> {
    if self.is_template {
      return None;
    }
    let is_pointer = self.is_pointer || self.is_reference;
    if !is_pointer {
      if CType::new(self.base.clone(), false).to_primitive_c_type().is_none() {
        //need to convert to pointer anyway
        return Some(CType::new(self.base.clone(), true));
      }
    }
    Some(CType::new(self.base.clone(), is_pointer))
  }
}

impl CType {
  fn new(base: String, is_pointer: bool) -> CType {
    CType {
      base: base,
      is_pointer: is_pointer,
    }
  }

  fn caption(&self) -> String {
    let mut r = self.base.clone();
    if self.is_pointer {
      r = r + &("_ptr".to_string());
    }
    r
  }

  fn to_primitive_c_type(&self) -> Option<CType> {
    if self.is_pointer {
      return None;
    }
    if self.base == "int" {
      Some(CType::new("int".to_string(), false))
    } else if self.base == "float" {
      Some(CType::new("float".to_string(), false))
    } else {
      None
    }
  }
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
  Class(String),
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
pub enum CFunctionArgumentCppEquivalent {
  This,
  Argument(i8),
  ReturnValue,
}

#[derive(Debug)]
pub struct CFunctionArgument {
  pub name: String,
  pub argument_type: CType,
  pub cpp_equivalent: CFunctionArgumentCppEquivalent,
}

#[derive(Debug)]
pub struct CFunctionSignature {
  pub arguments: Vec<CFunctionArgument>,
  pub return_type: Option<CType>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AllocationPlace {
  Stack,
  Heap,
}

impl CppMethod {
  fn real_return_type(&self) -> Option<CppType> {
    if self.is_constructor {
      if let CppMethodScope::Class(ref class_name) = self.scope {
        return Some(CppType {
          is_template: false, // TODO: is template if base class is template
          is_reference: false,
          is_pointer: false,
          is_const: false,
          base: class_name.clone(),
        });
      } else {
        panic!("constructor encountered with no class scope");
      }
    } else {
      return self.return_type.clone();
    }
  }


  pub fn c_signature(&self, allocation_place: AllocationPlace) -> Option<CFunctionSignature> {
    if self.is_variable || self.allows_variable_arguments {
      // no complicated cases support for now
      return None;
    }
    let mut r = CFunctionSignature {
      arguments: Vec::new(),
      return_type: None,
    };
    if let CppMethodScope::Class(ref class_name) = self.scope {
      r.arguments.push(CFunctionArgument {
        name: "self".to_string(),
        argument_type: CType {
          base: class_name.clone(),
          is_pointer: true,
        },
        cpp_equivalent: CFunctionArgumentCppEquivalent::This,
      });
    }
    for (index, arg) in self.arguments.iter().enumerate() {
      match arg.argument_type.to_c_type() {
        Some(c_type) => {
          r.arguments.push(CFunctionArgument {
            name: arg.name.clone(),
            argument_type: c_type,
            cpp_equivalent: CFunctionArgumentCppEquivalent::Argument(index as i8),
          });
        }
        None => return None,
      }
    }
    if let Some(return_type) = self.real_return_type() {
      match return_type.to_c_type() {
        Some(c_type) => {
          if allocation_place == AllocationPlace::Stack && c_type.to_primitive_c_type().is_none() {
            r.arguments.push(CFunctionArgument {
              name: "output".to_string(),
              argument_type: c_type,
              cpp_equivalent: CFunctionArgumentCppEquivalent::ReturnValue,
            });
          } else {
            r.return_type = Some(c_type);
          }
        }
        None => return None,
      }
    }
    Some(r)
  }
}







#[derive(Debug)]
pub struct CppHeaderData {
  pub include_file: String,
  pub class_name: Option<String>,
  pub methods: Vec<CppMethod>,
  pub macros: Vec<String>,
}
