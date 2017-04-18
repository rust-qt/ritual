//! Types for handling information about C++ methods.


use cpp_data::{CppVisibility, CppTypeAllocationPlace, CppData};
use cpp_ffi_data::{CppMethodWithFfiSignature, CppFfiType, CppFfiFunctionSignature,
                   CppFfiFunctionArgument, CppFfiArgumentMeaning};
use cpp_type::{CppType, CppTypeIndirection, CppTypeRole, CppTypeBase, CppTypeClassBase};
use common::errors::{Result, unexpected};
use common::string_utils::JoinWithSeparator;
use common::utils::MapIfOk;
pub use serializable::{CppFunctionArgument, CppMethodKind, CppMethod, CppMethodClassMembership,
                       CppFieldAccessorType, FakeCppMethod, CppMethodDoc};

pub use cpp_operator::{CppOperator, CppOperatorInfo};

/// Chosen type allocation place for the method
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ReturnValueAllocationPlace {
  /// The method returns a class object by value (or is a constructor), and
  /// it's translated to "output" FFI argument and placement new
  Stack,
  /// The method returns a class object by value (or is a constructor), and
  /// it's translated to pointer FFI return type and plain new
  Heap,
  /// The method does not return a class object by value, so
  /// the direct equivalent of the value is used in FFI.
  NotApplicable,
}

impl CppMethodKind {
  /// Returns true if this method is a constructor
  pub fn is_constructor(&self) -> bool {
    match *self {
      CppMethodKind::Constructor => true,
      _ => false,
    }
  }
  /// Returns true if this method is a destructor
  pub fn is_destructor(&self) -> bool {
    match *self {
      CppMethodKind::Destructor => true,
      _ => false,
    }
  }
  #[allow(dead_code)]
  /// Returns true if this method is a regular method or a free function
  pub fn is_regular(&self) -> bool {
    match *self {
      CppMethodKind::Regular => true,
      _ => false,
    }
  }
}

impl CppMethod {
  /// Checks if two methods have exactly the same set of input argument types
  pub fn argument_types_equal(&self, other: &CppMethod) -> bool {
    if self.arguments.len() != other.arguments.len() {
      return false;
    }
    if self.allows_variadic_arguments != other.allows_variadic_arguments {
      return false;
    }
    for (i, j) in self.arguments.iter().zip(other.arguments.iter()) {
      if i.argument_type != j.argument_type {
        return false;
      }
    }
    true
  }

  /// Creates FFI method signature for this method:
  /// - converts all types to FFI types;
  /// - adds "this" argument explicitly if present;
  /// - adds "output" argument for return value if `allocation_place` is `Stack`.
  pub fn c_signature(&self,
                     allocation_place: ReturnValueAllocationPlace)
                     -> Result<CppFfiFunctionSignature> {
    if self.allows_variadic_arguments {
      return Err("Variable arguments are not supported".into());
    }
    let mut r = CppFfiFunctionSignature {
      arguments: Vec::new(),
      return_type: CppFfiType::void(),
    };
    if let Some(ref info) = self.class_membership {
      if !info.is_static && info.kind != CppMethodKind::Constructor {
        r.arguments
          .push(CppFfiFunctionArgument {
                  name: "this_ptr".to_string(),
                  argument_type: CppType {
                      base: CppTypeBase::Class(info.class_type.clone()),
                      is_const: info.is_const,
                      is_const2: false,
                      indirection: CppTypeIndirection::Ptr,
                    }.to_cpp_ffi_type(CppTypeRole::NotReturnType)?,
                  meaning: CppFfiArgumentMeaning::This,
                });
      }
    }
    for (index, arg) in self.arguments.iter().enumerate() {
      let c_type = arg.argument_type
        .to_cpp_ffi_type(CppTypeRole::NotReturnType)?;
      r.arguments
        .push(CppFfiFunctionArgument {
                name: arg.name.clone(),
                argument_type: c_type,
                meaning: CppFfiArgumentMeaning::Argument(index as i8),
              });
    }
    let real_return_type = if let Some(info) = self.class_info_if_constructor() {
      CppType {
        is_const: false,
        is_const2: false,
        indirection: CppTypeIndirection::None,
        base: CppTypeBase::Class(info.class_type.clone()),
      }
    } else {
      self.return_type.clone()
    };
    let c_type = real_return_type.to_cpp_ffi_type(CppTypeRole::ReturnType)?;
    if real_return_type.needs_allocation_place_variants() {
      match allocation_place {
        ReturnValueAllocationPlace::Stack => {
          r.arguments
            .push(CppFfiFunctionArgument {
                    name: "output".to_string(),
                    argument_type: c_type,
                    meaning: CppFfiArgumentMeaning::ReturnValue,
                  });
        }
        ReturnValueAllocationPlace::Heap => {
          r.return_type = c_type;
        }
        ReturnValueAllocationPlace::NotApplicable => {
          return Err(unexpected("NotApplicable encountered but return value needs \
                                 allocation_place variants")
                         .into());
        }
      }
    } else {
      r.return_type = c_type;
    }
    Ok(r)
  }

  /// Generates the FFI function signature for this method.
  pub fn to_ffi_signature(&self,
                          cpp_data: &CppData,
                          type_allocation_places_override: Option<CppTypeAllocationPlace>)
                          -> Result<CppMethodWithFfiSignature> {
    let get_place = |name| -> Result<ReturnValueAllocationPlace> {
      let v = if let Some(ref x) = type_allocation_places_override {
        x.clone()
      } else {
        cpp_data.type_allocation_place(name)?
      };
      Ok(match v {
           CppTypeAllocationPlace::Heap => ReturnValueAllocationPlace::Heap,
           CppTypeAllocationPlace::Stack => ReturnValueAllocationPlace::Stack,
         })
    };

    let place = if self.is_constructor() || self.is_destructor() {
      let info = self
        .class_membership
        .as_ref()
        .expect("class info expected here");
      get_place(&info.class_type.name)?
    } else if self.return_type.needs_allocation_place_variants() {
      if let CppTypeBase::Class(CppTypeClassBase { ref name, .. }) = self.return_type.base {
        get_place(name)?
      } else {
        return Err(unexpected("class type expected here").into());
      }
    } else {
      ReturnValueAllocationPlace::NotApplicable
    };

    let c_signature = self.c_signature(place.clone())?;
    Ok(CppMethodWithFfiSignature {
         cpp_method: self.clone(),
         allocation_place: place,
         c_signature: c_signature,
       })
  }

  /// Returns fully qualified C++ name of this method,
  /// i.e. including namespaces and class name (if any).
  /// This method is not suitable for code generation.
  pub fn full_name(&self) -> String {
    if let Some(ref info) = self.class_membership {
      format!("{}::{}",
              CppTypeBase::Class(info.class_type.clone()).to_cpp_pseudo_code(),
              self.name)
    } else {
      self.name.clone()
    }
  }

  /// Returns the identifier this method would be presented with
  /// in Qt documentation.
  pub fn doc_id(&self) -> String {
    if let Some(ref info) = self.class_membership {
      format!("{}::{}", info.class_type.name, self.name)
    } else {
      self.name.clone()
    }
  }

  /// Returns short text representing values in this method
  /// (only for debugging output).
  pub fn short_text(&self) -> String {
    let mut s = String::new();
    if let Some(ref info) = self.class_membership {
      if info.is_virtual {
        if info.is_pure_virtual {
          s = format!("{} pure virtual", s);
        } else {
          s = format!("{} virtual", s);
        }
      }
      if info.is_static {
        s = format!("{} static", s);
      }
      if info.visibility == CppVisibility::Protected {
        s = format!("{} protected", s);
      }
      if info.visibility == CppVisibility::Private {
        s = format!("{} private", s);
      }
      if info.is_signal {
        s = format!("{} [signal]", s);
      }
      if info.is_slot {
        s = format!("{} [slot]", s);
      }
      match info.kind {
        CppMethodKind::Constructor => s = format!("{} [constructor]", s),
        CppMethodKind::Destructor => s = format!("{} [destructor]", s),
        CppMethodKind::Regular => {}
      }
    }
    if self.allows_variadic_arguments {
      s = format!("{} [var args]", s);
    }
    s = format!("{} {}", s, self.return_type.to_cpp_pseudo_code());
    s = format!("{} {}", s, self.full_name());
    if let Some(ref args) = self.template_arguments {
      s = format!("{}<{}>", s, args.names.iter().join(", "));
    }
    if let Some(ref args) = self.template_arguments_values {
      s = format!("{}<{}>",
                  s,
                  args.iter().map(|x| x.to_cpp_pseudo_code()).join(", "));
    }
    s = format!("{}({})",
                s,
                self
                  .arguments
                  .iter()
                  .map(|arg| {
      format!("{} {}{}",
              arg.argument_type.to_cpp_pseudo_code(),
              arg.name,
              if arg.has_default_value {
                " = ?".to_string()
              } else {
                String::new()
              })
    })
                  .join(", "));
    if let Some(ref info) = self.class_membership {
      if info.is_const {
        s = format!("{} const", s);
      }
    }
    s.trim().to_string()
  }

  /// Returns debugging output for `inheritance_chain` content.
  pub fn inheritance_chain_text(&self) -> String {
    self
      .inheritance_chain
      .iter()
      .map(|x| {
        let mut text = x.base_type.to_cpp_pseudo_code();
        if x.is_virtual {
          text = format!("virtual {}", text);
        }
        match x.visibility {
          CppVisibility::Protected => text = format!("protected {}", text),
          CppVisibility::Private => text = format!("private {}", text),
          CppVisibility::Public => {}
        }
        text
      })
      .join(" -> ")
  }

  /// Returns name of the class this method belongs to, if any.
  pub fn class_name(&self) -> Option<&String> {
    match self.class_membership {
      Some(ref info) => Some(&info.class_type.name),
      None => None,
    }
  }

  /// Returns true if this method is a constructor.
  pub fn is_constructor(&self) -> bool {
    match self.class_membership {
      Some(ref info) => info.kind.is_constructor(),
      None => false,
    }
  }

  /// Returns true if this method is a destructor.
  pub fn is_destructor(&self) -> bool {
    match self.class_membership {
      Some(ref info) => info.kind.is_destructor(),
      None => false,
    }
  }

  /// A convenience method. Returns `class_membership` if
  /// the method is a constructor, and `None` otherwise.
  pub fn class_info_if_constructor(&self) -> Option<&CppMethodClassMembership> {
    if let Some(ref info) = self.class_membership {
      if info.kind.is_constructor() {
        Some(info)
      } else {
        None
      }
    } else {
      None
    }
  }

  /// Returns the identifier that should be used in `QObject::connect`
  /// to specify this signal or slot.
  pub fn receiver_id(&self) -> Result<String> {
    let type_num = if let Some(ref info) = self.class_membership {
      if info.is_slot {
        "1"
      } else if info.is_signal {
        "2"
      } else {
        return Err("not a signal or slot".into());
      }
    } else {
      return Err("not a class method".into());
    };
    Ok(format!("{}{}({})",
               type_num,
               self.name,
               self
                 .arguments
                 .iter()
                 .map_if_ok(|arg| arg.argument_type.to_cpp_code(None))?
                 .join(",")))
  }



  #[allow(dead_code)]
  /// Returns true if this method is an operator.
  pub fn is_operator(&self) -> bool {
    self.operator.is_some()
  }

  /// Returns collection of all types found in the signature of this method,
  /// including argument types, return type and type of `this` implicit parameter.
  pub fn all_involved_types(&self) -> Vec<CppType> {
    let mut result: Vec<CppType> = Vec::new();
    if let Some(ref class_membership) = self.class_membership {
      result.push(CppType {
                    base: CppTypeBase::Class(class_membership.class_type.clone()),
                    is_const: class_membership.is_const,
                    is_const2: false,
                    indirection: CppTypeIndirection::Ptr,
                  });
    }
    for t in self.arguments.iter().map(|x| x.argument_type.clone()) {
      result.push(t);
    }
    result.push(self.return_type.clone());
    if let Some(ref operator) = self.operator {
      if let CppOperator::Conversion(ref cpp_type) = *operator {
        result.push(cpp_type.clone());
      }
    }
    result
  }
}

