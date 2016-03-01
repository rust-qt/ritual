use std::collections::HashMap;


pub trait JoinWithString {
  fn join(self, separator: &'static str) -> String;
}

impl<X> JoinWithString for X
  where X: Iterator<Item = String>
{
  fn join(self, separator: &'static str) -> String {
    self.fold("".to_string(), |a, b| {
      let m = if a.len() > 0 {
        a + separator
      } else {
        a
      };
      m + &b
    })
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
  pub is_const: bool,
  pub base: String,
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IndirectionChange {
  NoChange,
  ValueToPointer,
  ReferenceToPointer,
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppToCTypeConversion {
  pub indirection_change: IndirectionChange,
  pub renamed: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CTypeExtended {
  pub c_type: CType,
  pub is_primitive: bool,
  pub conversion: CppToCTypeConversion,
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
  pub original_index: i32,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CFunctionArgumentCppEquivalent {
  This,
  Argument(i8),
  ReturnValue,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CFunctionArgument {
  pub name: String,
  pub argument_type: CTypeExtended,
  pub cpp_equivalent: CFunctionArgumentCppEquivalent,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum ArgumentCaptionStrategy {
  NameOnly,
  TypeOnly,
  TypeAndName,
}

impl ArgumentCaptionStrategy {
  fn all() -> Vec<Self> {
    vec![ArgumentCaptionStrategy::NameOnly,
         ArgumentCaptionStrategy::TypeOnly,
         ArgumentCaptionStrategy::TypeAndName]
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum MethodCaptionStrategy {
  ArgumentsOnly(ArgumentCaptionStrategy),
  ConstOnly,
  ConstAndArguments(ArgumentCaptionStrategy),
}

impl MethodCaptionStrategy {
  fn all() -> Vec<Self> {
    let mut r = vec![];
    for i in ArgumentCaptionStrategy::all() {
      r.push(MethodCaptionStrategy::ArgumentsOnly(i));
    }
    r.push(MethodCaptionStrategy::ConstOnly);
    for i in ArgumentCaptionStrategy::all() {
      r.push(MethodCaptionStrategy::ConstAndArguments(i));
    }
    r
  }
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CFunctionSignature {
  pub arguments: Vec<CFunctionArgument>,
  pub return_type: CTypeExtended,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AllocationPlace {
  Stack,
  Heap,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AllocationPlaceImportance {
  Important,
  NotImportant,
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



#[derive(Debug)]
pub struct CppHeaderData {
  pub include_file: String,
  pub class_name: Option<String>,
  pub methods: Vec<CppMethod>,
  pub macros: Vec<String>,
}


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






impl CTypeExtended {
  pub fn void() -> Self {
    CTypeExtended {
      c_type: CType::void(),
      is_primitive: true,
      conversion: CppToCTypeConversion {
        indirection_change: IndirectionChange::NoChange,
        renamed: false,
      },
    }
  }
}

impl CppType {
  pub fn to_cpp_code(&self) -> String {
    let mut r = self.base.clone();
    if self.is_pointer {
      r = r + &("*".to_string());
    }
    if self.is_reference {
      r = r + &("&".to_string());
    }
    if self.is_const {
      r = "const ".to_string() + &r;
    }
    r
  }

  fn to_c_type(&self) -> Option<CTypeExtended> {
    if self.is_template {
      return None;
    }
    let mut result = CTypeExtended::void();
    result.c_type.is_const = self.is_const;
    if !self.is_pointer && !self.is_reference {
      // "const Rect" return type should not be translated to const pointer
      result.c_type.is_const = false;
    }
    if self.is_pointer {
      result.c_type.is_pointer = true;
    }
    if self.is_reference {
      result.c_type.is_pointer = true;
      result.conversion.indirection_change = IndirectionChange::ReferenceToPointer;
    }

    let good_primitive_types = vec![
      "void", "float", "double", "bool", "char",
      "qint8", "quint8", "qint16", "quint16", "qint32", "quint32", "qint64", "quint64",
      "qlonglong","qulonglong",
      "signed char", "unsigned char", "uchar",
      "short", "unsigned short", "ushort",
      "int", "unsigned int", "uint",
      "long", "unsigned long", "ulong"
    ];

    //let mut aliased_primitive_types = HashMap::new();
    //aliased_primitive_types.insert("qint8", "int8_t");

    if good_primitive_types.iter().find(|&x| x == &self.base).is_some() {
      result.is_primitive = true;
      result.c_type.base = self.base.clone();
    //} else if let Some(found) = aliased_primitive_types.get(self.base.as_ref() as &str) {
    //  result.is_primitive = true;
    //  result.c_type.base = found.to_string();
    } else {
      result.is_primitive = false;
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

impl CType {
  pub fn void() -> Self {
    CType {
      base: "void".to_string(),
      is_pointer: false,
      is_const: false,
    }
  }
  fn new(base: String, is_pointer: bool, is_const: bool) -> CType {
    CType {
      base: base,
      is_pointer: is_pointer,
      is_const: is_const,
    }
  }

  fn caption(&self) -> String {
    let mut r = self.base.clone();
    if self.is_pointer {
      r = r + &("_ptr".to_string());
    }
    if self.is_const {
      r = "const_".to_string() + &r;
    }
    r
  }

  pub fn to_c_code(&self) -> String {
    let mut r = self.base.clone();
    if self.is_pointer {
      r = r + &("*".to_string());
    }
    if self.is_const {
      r = "const ".to_string() + &r;
    }
    r
  }
}

impl CFunctionArgumentCppEquivalent {
  fn is_argument(&self) -> bool {
    match self {
      &CFunctionArgumentCppEquivalent::Argument(..) => true,
      _ => false,
    }
  }
}

impl CFunctionArgument {
  fn caption(&self, strategy: ArgumentCaptionStrategy) -> String {
    match strategy {
      ArgumentCaptionStrategy::NameOnly => self.name.clone(),
      ArgumentCaptionStrategy::TypeOnly => self.argument_type.c_type.caption(),
      ArgumentCaptionStrategy::TypeAndName => {
        self.argument_type.c_type.caption() + &("_".to_string()) + &self.name
      }
    }
  }

  pub fn to_c_code(&self) -> String {
    self.argument_type.c_type.to_c_code() + &(" ".to_string()) + &self.name
  }
}

impl CFunctionSignature {
  fn caption(&self, strategy: ArgumentCaptionStrategy) -> String {
    let r = self.arguments
                .iter()
                .filter(|x| x.cpp_equivalent.is_argument())
                .map(|x| x.caption(strategy.clone()))
                .join("_");
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
        .join(", ")
  }
}


impl CppMethodWithCSignature {
  fn from_cpp_method(cpp_method: &CppMethod)
                     -> (Option<CppMethodWithCSignature>,
                         Option<CppMethodWithCSignature>) {
    match cpp_method.c_signature(AllocationPlace::Heap) {
      Some((c_signature, importance)) => {
        let result1 = Some(CppMethodWithCSignature {
          cpp_method: cpp_method.clone(),
          allocation_place: AllocationPlace::Heap,
          c_signature: c_signature,
        });
        match importance {
          AllocationPlaceImportance::Important => {
            let result2 = match cpp_method.c_signature(AllocationPlace::Stack) {
              Some((c_signature2, _)) => {
                Some(CppMethodWithCSignature {
                  cpp_method: cpp_method.clone(),
                  allocation_place: AllocationPlace::Stack,
                  c_signature: c_signature2,
                })
              }
              None => None,
            };
            (result1, result2)
          }
          AllocationPlaceImportance::NotImportant => (result1, None),
        }
      }
      None => (None, None),
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

  fn caption(&self, strategy: MethodCaptionStrategy) -> String {
    match strategy {
      MethodCaptionStrategy::ArgumentsOnly(s) => self.c_signature.caption(s),
      MethodCaptionStrategy::ConstOnly => {
        if self.cpp_method.is_const {
          "const".to_string()
        } else {
          "".to_string()
        }
      }
      MethodCaptionStrategy::ConstAndArguments(s) => {
        let r = if self.cpp_method.is_const {
          "const_".to_string()
        } else {
          "".to_string()
        };
        r + &self.c_signature.caption(s)
      }
    }



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

  pub fn c_signature(&self,
                     allocation_place: AllocationPlace)
                     -> Option<(CFunctionSignature, AllocationPlaceImportance)> {
    if self.is_variable || self.allows_variable_arguments {
      // no complicated cases support for now
      // TODO: return Err
      println!("Variable arguments are not supported");
      return None;
    }
    let mut allocation_place_importance = AllocationPlaceImportance::NotImportant;
    let mut r = CFunctionSignature {
      arguments: Vec::new(),
      return_type: CTypeExtended::void(),
    };
    if let CppMethodScope::Class(ref class_name) = self.scope {
      if !self.is_static && !self.is_constructor {
        r.arguments.push(CFunctionArgument {
          name: "self".to_string(),
          argument_type: CTypeExtended {
            c_type: CType {
              base: class_name.clone(),
              is_pointer: true,
              is_const: self.is_const,
            },
            is_primitive: false,
            conversion: CppToCTypeConversion {
              indirection_change: IndirectionChange::NoChange,
              renamed: false,
            },
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
        None => {
          println!("Can't convert type to C: {:?}", arg.argument_type);
          return None;
        }
      }
    }
    if let Some(return_type) = self.real_return_type() {
      match return_type.to_c_type() {
        Some(c_type) => {
          if return_type.is_stack_allocated_struct() {
            allocation_place_importance = AllocationPlaceImportance::Important;
            if allocation_place == AllocationPlace::Stack {
              r.arguments.push(CFunctionArgument {
                name: "output".to_string(),
                argument_type: c_type,
                cpp_equivalent: CFunctionArgumentCppEquivalent::ReturnValue,
              });
            } else {
              r.return_type = c_type;
            }
          } else {
            r.return_type = c_type;
          }
        }
        None => return None,
      }
    }
    if self.is_destructor {
      allocation_place_importance = AllocationPlaceImportance::Important;
    }
    Some((r, allocation_place_importance))
  }
}

impl CppHeaderData {
  pub fn ensure_explicit_destructor(&mut self) {
    if let Some(ref class_name) = self.class_name {
      if self.methods.iter().find(|x| x.is_destructor).is_none() {
        self.methods.push(CppMethod {
          name: format!("~{}", class_name),
          scope: CppMethodScope::Class(class_name.clone()),
          is_virtual: false, // TODO: destructors may be virtual
          is_const: false,
          is_static: false,
          return_type: None,
          is_constructor: false,
          is_destructor: true,
          operator: None,
          is_variable: false,
          arguments: vec![],
          allows_variable_arguments: false,
          original_index: 1000,
        });
      }
    }
  }


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
        let (result_heap, result_stack) = CppMethodWithCSignature::from_cpp_method(&method);
        if let Some(result_heap) = result_heap {
          if let Some(result_stack) = result_stack {
            let mut stack_name = result_stack.c_base_name();
            let mut heap_name = result_heap.c_base_name();
            if stack_name == heap_name {
              stack_name = "SA_".to_string() + &stack_name;
              heap_name = "HA_".to_string() + &heap_name;
            }
            insert_into_hash(&mut hash1, stack_name, result_stack);
            insert_into_hash(&mut hash1, heap_name, result_heap);
          } else {
            let c_base_name = result_heap.c_base_name();
            insert_into_hash(&mut hash1, c_base_name, result_heap);
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
      for strategy in MethodCaptionStrategy::all() {
        let mut type_captions: Vec<_> = values.iter()
                                              .map(|x| x.caption(strategy.clone()))
                                              .collect();
        // println!("test1 {:?}", type_captions);
        type_captions.sort();
        type_captions.dedup();
        if type_captions.len() == values.len() {
          found_strategy = Some(strategy);
          break;
        }
      }
      if let Some(strategy) = found_strategy {
        for x in values {
          let caption = x.caption(strategy.clone());
          r.push(CppAndCMethod::new(x,
                                    self.include_file.clone() + &("_".to_string()) + &key +
                                    &((if caption.is_empty() {
                                        ""
                                      } else {
                                        "_"
                                      })
                                      .to_string()) +
                                    &caption));
        }
      } else {
        panic!("all type caption strategies have failed! Involved functions: \n{:?}",
               values);
      }
    }
    r.sort_by(|a, b| a.cpp_method.original_index.cmp(&b.cpp_method.original_index));
    r
  }
}
