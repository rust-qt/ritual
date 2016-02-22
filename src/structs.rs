


pub fn operator_c_name(cpp_name: &String, arguments_count: i32) -> String {
  if cpp_name == "=" && arguments_count == 2 {
    return "assign".to_string();
  } else if cpp_name == "+" && arguments_count == 2 {
    return "add".to_string();
  } else if cpp_name == "-" && arguments_count == 2 {
    return "sub".to_string();
  } else if cpp_name == "+" && arguments_count == 1 {
    return "unary_plus".to_string();
  } else if cpp_name == "-" && arguments_count == 1 {
    return "neg".to_string();
  } else if cpp_name == "*" && arguments_count == 2 {
    return "mul".to_string();
  } else if cpp_name == "/" && arguments_count == 2 {
    return "div".to_string();
  } else if cpp_name == "%" && arguments_count == 2 {
    return "rem".to_string();
  } else if cpp_name == "++" && arguments_count == 1 {
    return "inc".to_string();
  } else if cpp_name == "++" && arguments_count == 2 {
    return "inc_postfix".to_string();
  } else if cpp_name == "--" && arguments_count == 1 {
    return "dec".to_string();
  } else if cpp_name == "--" && arguments_count == 2 {
    return "dec_postfix".to_string();
  } else if cpp_name == "==" && arguments_count == 2 {
    return "eq".to_string();
  } else if cpp_name == "!=" && arguments_count == 2 {
    return "neq".to_string();
  } else if cpp_name == ">" && arguments_count == 2 {
    return "gt".to_string();
  } else if cpp_name == "<" && arguments_count == 2 {
    return "lt".to_string();
  } else if cpp_name == ">=" && arguments_count == 2 {
    return "ge".to_string();
  } else if cpp_name == "<=" && arguments_count == 2 {
    return "le".to_string();
  } else if cpp_name == "!" && arguments_count == 1 {
    return "not".to_string();
  } else if cpp_name == "&&" && arguments_count == 2 {
    return "and".to_string();
  } else if cpp_name == "||" && arguments_count == 2 {
    return "or".to_string();
  } else if cpp_name == "~" && arguments_count == 1 {
    return "bit_not".to_string();
  } else if cpp_name == "&" && arguments_count == 2 {
    return "bit_and".to_string();
  } else if cpp_name == "|" && arguments_count == 2 {
    return "bit_or".to_string();
  } else if cpp_name == "^" && arguments_count == 2 {
    return "bit_xor".to_string();
  } else if cpp_name == "<<" && arguments_count == 2 {
    return "shl".to_string();
  } else if cpp_name == ">>" && arguments_count == 2 {
    return "shr".to_string();
  } else if cpp_name == "+=" && arguments_count == 2 {
    return "add_assign".to_string();
  } else if cpp_name == "-=" && arguments_count == 2 {
    return "sub_assign".to_string();
  } else if cpp_name == "*=" && arguments_count == 2 {
    return "mul_assign".to_string();
  } else if cpp_name == "/=" && arguments_count == 2 {
    return "div_assign".to_string();
  } else if cpp_name == "%=" && arguments_count == 2 {
    return "rem_assign".to_string();
  } else if cpp_name == "&=" && arguments_count == 2 {
    return "bit_and_assign".to_string();
  } else if cpp_name == "|=" && arguments_count == 2 {
    return "bit_or_assign".to_string();
  } else if cpp_name == "^=" && arguments_count == 2 {
    return "bit_xor_assign".to_string();
  } else if cpp_name == "<<=" && arguments_count == 2 {
    return "shl_assign".to_string();
  } else if cpp_name == ">>=" && arguments_count == 2 {
    return "shr_assign".to_string();
  } else if cpp_name == "[]" && arguments_count == 2 {
    return "index".to_string();
  } else if cpp_name == "()" && arguments_count == 1 {
    return "call".to_string();
  } else if cpp_name == "," && arguments_count == 2 {
    return "comma".to_string();
  } else {
    panic!("unsupported operator");
  }
}





#[derive(Debug, Clone)]
pub struct CppType {
  pub is_template: bool,
  pub is_const: bool,
  pub is_reference: bool,
  pub is_pointer: bool,
  pub base: String,
}

#[derive(Debug, Clone)]
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
        // need to convert to pointer anyway
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

#[derive(Debug, Clone)]
pub struct CppFunctionArgument {
  pub name: String,
  pub argument_type: CppType,
  pub default_value: Option<String>,
}

#[derive(Debug, Clone)]
pub enum CppMethodScope {
  Global,
  Class(String),
}

#[derive(Debug, Clone)]
pub struct CppMethod {
  pub name: String,
  pub scope: CppMethodScope,
  pub is_virtual: bool,
  pub is_const: bool,
  pub is_static: bool,
  pub return_type: Option<CppType>,
  pub is_constructor: bool,
  pub is_destructor: bool,
  pub operator: Option<String>,
  pub is_variable: bool,
  pub arguments: Vec<CppFunctionArgument>,
  pub allows_variable_arguments: bool,
}

#[derive(Debug, Clone)]
pub enum CFunctionArgumentCppEquivalent {
  This,
  Argument(i8),
  ReturnValue,
}

#[derive(Debug, Clone)]
pub struct CFunctionArgument {
  pub name: String,
  pub argument_type: CType,
  pub cpp_equivalent: CFunctionArgumentCppEquivalent,
}

#[derive(Debug, Clone)]
pub struct CFunctionSignature {
  pub arguments: Vec<CFunctionArgument>,
  pub return_type: Option<CType>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AllocationPlace {
  Stack,
  Heap,
}

pub struct CppMethodWithCSignature {
  cpp_method: CppMethod,
  allocation_place: AllocationPlace,
  c_signature: CFunctionSignature,
}

pub struct CppAndCMethod {
  cpp_method: CppMethod,
  allocation_place: AllocationPlace,
  c_signature: CFunctionSignature,
  c_name: String,
}

impl CppMethodWithCSignature {
  fn from_cpp_method(cpp_method: &CppMethod,
                     allocation_place: AllocationPlace)
                     -> Option<CppMethodWithCSignature> {
    match cpp_method.c_signature(allocation_place.clone()) {
      Some(c_signature) => {
        Some(CppMethodWithCSignature {
          cpp_method: cpp_method.clone(),
          allocation_place: allocation_place,
          c_signature: c_signature,
        })
      }
      None => None,
    }
  }

  pub fn c_base_name(&self) -> String {
    let scope_prefix = match self.cpp_method.scope {
      CppMethodScope::Class(ref class_name) => class_name.clone() + &("_".to_string()),
      CppMethodScope::Global => "".to_string(),
    };
    let method_name = if self.cpp_method.is_constructor {
      match self.allocation_place {
        AllocationPlace::Stack => "constructor".to_string(),
        AllocationPlace::Heap => "new".to_string(),
      }
    } else if self.cpp_method.is_destructor {
      match self.allocation_place {
        AllocationPlace::Stack => "destructor".to_string(),
        AllocationPlace::Heap => "delete".to_string(),
      }
    } else if let Some(ref operator) = self.cpp_method.operator {
      operator_c_name(operator, self.c_signature.arguments.len() as i32)
    } else {
      self.cpp_method.name.clone()
    };
    scope_prefix + &method_name
  }

}

impl CppAndCMethod {
  fn new(data: CppMethodWithCSignature, c_name: String) -> CppAndCMethod {
    CppAndCMethod {
      cpp_method: data.cpp_method,
      allocation_place: data.allocation_place,
      c_signature: data.c_signature,
      c_name: c_name,
    }
  }
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
      if !self.is_static {
        r.arguments.push(CFunctionArgument {
          name: "self".to_string(),
          argument_type: CType {
            base: class_name.clone(),
            is_pointer: true,
          },
          cpp_equivalent: CFunctionArgumentCppEquivalent::This,
        });
      }
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

impl CppHeaderData {
  pub fn process_methods(&self) -> Vec<CppAndCMethod> {
    let mut r = Vec::new();



    r
  }

}
