use std::collections::HashMap;

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
    panic!("unsupported operator: {}, {}", cpp_name, arguments_count);
  }
}





#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppType {
  pub is_template: bool,
  pub is_const: bool,
  pub is_reference: bool,
  pub is_pointer: bool,
  pub base: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CType {
  pub is_pointer: bool,
  pub base: String,
}

impl CppType {
  fn to_c_type(&self) -> Option<CType> {
    if self.is_template {
      return None;
    }
    if self.is_pointer || self.is_reference {
      return Some(CType::new(self.base.clone(), true));
    } else {
      if self.base == "void" || self.base == "int" || self.base == "float" ||
         self.base == "double" || self.base == "bool" {
        return Some(CType::new(self.base.clone(), false));
      } else if self.base == "quint8" {
        return Some(CType::new("int8_t".to_string(), false));
        // TODO: more type conversions
      } else {
        // need to convert to pointer anyway
        return Some(CType::new(self.base.clone(), true));
      }
    }
  }

  fn is_stack_allocated_struct(&self) -> bool {
    !self.is_pointer && !self.is_reference && self.base.starts_with("Q")
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
    // if self.is_pointer {
    //  r = r + &("_ptr".to_string());
    // }
    r
  }

  pub fn to_c_code(&self) -> String {
    let mut r = self.base.clone();
    if self.is_pointer {
      r = r + &("*".to_string());
    }
    r
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppFunctionArgument {
  pub name: String,
  pub argument_type: CppType,
  pub default_value: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppMethodScope {
  Global,
  Class(String),
}

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CFunctionArgumentCppEquivalent {
  This,
  Argument(i8),
  ReturnValue,
}

impl CFunctionArgumentCppEquivalent {
  fn is_argument(&self) -> bool {
    match self {
      &CFunctionArgumentCppEquivalent::Argument(..) => true,
      _ => false,
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CFunctionArgument {
  pub name: String,
  pub argument_type: CType,
  pub cpp_equivalent: CFunctionArgumentCppEquivalent,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum ArgumentCaptionStrategy {
  NameOnly,
  TypeOnly,
  TypeAndName,
}

impl CFunctionArgument {
  fn caption(&self, strategy: ArgumentCaptionStrategy) -> String {
    match strategy {
      ArgumentCaptionStrategy::NameOnly => self.name.clone(),
      ArgumentCaptionStrategy::TypeOnly => self.argument_type.caption(),
      ArgumentCaptionStrategy::TypeAndName => {
        self.argument_type.caption() + &("_".to_string()) + &self.name
      }
    }
  }

  pub fn to_c_code(&self) -> String {
    self.argument_type.to_c_code() + &(" ".to_string()) + &self.name
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CFunctionSignature {
  pub arguments: Vec<CFunctionArgument>,
  pub return_type: CType,
}

impl CFunctionSignature {
  fn caption(&self, strategy: ArgumentCaptionStrategy) -> String {
    let r = self.arguments
                .iter()
                .filter(|x| x.cpp_equivalent.is_argument())
                .map(|x| x.caption(strategy.clone()))
                .fold("".to_string(), |a, b| {
                  let m = if a.len() > 0 {
                    a + "_"
                  } else {
                    a
                  };
                  m + &b
                });
    if r.len() == 0 {
      "no_args".to_string()
    } else {
      r
    }

  }

  pub fn arguments_to_c_code(&self) -> String {
    self.arguments
        .iter()
        .map(|x| x.to_c_code())
        .fold("".to_string(), |a, b| {
          let m = if a.len() > 0 {
            a + ", "
          } else {
            a
          };
          m + &b
        })
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AllocationPlace {
  Stack,
  Heap,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppMethodWithCSignature {
  pub cpp_method: CppMethod,
  pub allocation_place: AllocationPlace,
  pub c_signature: CFunctionSignature,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppAndCMethod {
  pub cpp_method: CppMethod,
  pub allocation_place: AllocationPlace,
  pub c_signature: CFunctionSignature,
  pub c_name: String,
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
      CppMethodScope::Class(..) => "".to_string(),
      CppMethodScope::Global => "G_".to_string(),
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
      "OP_".to_string() + &operator_c_name(operator, self.cpp_method.real_arguments_count())
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

  fn real_arguments_count(&self) -> i32 {
    let mut result = self.arguments.len() as i32;
    if let CppMethodScope::Class(..) = self.scope {
      if !self.is_static {
        result += 1;
      }
    }
    result
  }


  pub fn c_signature(&self, allocation_place: AllocationPlace) -> Option<CFunctionSignature> {
    if self.is_variable || self.allows_variable_arguments {
      // no complicated cases support for now
      return None;
    }
    let mut r = CFunctionSignature {
      arguments: Vec::new(),
      return_type: CType { base: "void".to_string(), is_pointer: false },
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
          if allocation_place == AllocationPlace::Stack && return_type.is_stack_allocated_struct() {
            r.arguments.push(CFunctionArgument {
              name: "output".to_string(),
              argument_type: c_type,
              cpp_equivalent: CFunctionArgumentCppEquivalent::ReturnValue,
            });
          } else {
            r.return_type = c_type;
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
  pub fn involves_templates(&self) -> bool {
    for method in &self.methods {
      if let Some(ref t) = method.return_type {
        if t.is_template {
          return true;
        }
      }
      for arg in &method.arguments {
        if arg.argument_type.is_template {
          return true;
        }
      }
    }
    false
  }


  pub fn process_methods(&self) -> Vec<CppAndCMethod> {
    println!("Processing header <{}>", self.include_file);
    let mut hash1 = HashMap::new();
    {
      let insert_into_hash = |hash: &mut HashMap<String, Vec<_>>, key: String, value| {
        if let Some(values) = hash.get_mut(&key) {
          values.push(value);
          return;
        }
        hash.insert(key, vec![value]);
      };

      for ref method in &self.methods {
        if let Some(result_stack) =
               CppMethodWithCSignature::from_cpp_method(&method, AllocationPlace::Stack) {
          if let Some(result_heap) =
                 CppMethodWithCSignature::from_cpp_method(&method, AllocationPlace::Heap) {
            if result_stack.c_signature == result_heap.c_signature {
              let c_base_name = result_stack.c_base_name();
              insert_into_hash(&mut hash1, c_base_name, result_stack);
            } else {
              let mut stack_name = result_stack.c_base_name();
              let mut heap_name = result_heap.c_base_name();
              if stack_name == heap_name {
                stack_name = "SA_".to_string() + &stack_name;
                heap_name = "HA_".to_string() + &heap_name;
              }
              insert_into_hash(&mut hash1, stack_name, result_stack);
              insert_into_hash(&mut hash1, heap_name, result_heap);
            }
          } else {
            panic!("unexpected error: stack strategy success but heap strategy fail");
          }
        } else {
          println!("Unable to produce C function for method: {:?}", method);
        }
      }
    }
    let mut r = Vec::new();
    for (key, mut values) in hash1.into_iter() {
      if values.len() == 1 {
        r.push(CppAndCMethod::new(values.remove(0),
                                  self.include_file.clone() + &("_".to_string()) + &key));
        continue;
      }
      let mut found_strategy = None;
      for strategy in vec![ArgumentCaptionStrategy::NameOnly,
                           ArgumentCaptionStrategy::TypeOnly,
                           ArgumentCaptionStrategy::TypeAndName] {
        let mut type_captions: Vec<_> = values.iter()
                                              .map(|x| x.c_signature.caption(strategy.clone()))
                                              .collect();
        type_captions.sort();
        type_captions.dedup();
        if type_captions.len() == values.len() {
          found_strategy = Some(strategy);
          break;
        }
      }
      if let Some(strategy) = found_strategy {
        for x in values {
          let caption = x.c_signature.caption(strategy.clone());
          r.push(CppAndCMethod::new(x,
                                    self.include_file.clone() + &("_".to_string()) + &key +
                                    &("_".to_string()) +
                                    &caption));
        }
      } else {
        panic!("all type caption strategies have failed!");
      }
    }

    for x in &r {
      println!("{}", x.c_name);
    }

    r
  }
}
