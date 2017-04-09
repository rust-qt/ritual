use common::errors::{Result, unexpected, ChainErr};
use common::string_utils::CaseOperations;
use common::utils::MapIfOk;

pub use serializable::{RustName, RustTypeIndirection, RustType, CompleteType,
                       RustToCTypeConversion};


impl RustName {
  pub fn new(parts: Vec<String>) -> Result<RustName> {
    if parts.is_empty() {
      return Err(unexpected("RustName can't be empty").into());
    }
    Ok(RustName { parts: parts })
  }

  pub fn crate_name(&self) -> Option<&String> {
    assert!(self.parts.len() > 0);
    if self.parts.len() > 1 {
      Some(&self.parts[0])
    } else {
      None
    }
  }
  pub fn last_name(&self) -> Result<&String> {
    self
      .parts
      .last()
      .chain_err(|| unexpected("RustName can't be empty"))
  }
  pub fn full_name(&self, current_crate: Option<&str>) -> String {
    if let Some(current_crate) = current_crate {
      if let Some(self_crate) = self.crate_name() {
        if self_crate == current_crate {
          return format!("::{}", self.parts[1..].join("::"));
        }
      }
    }
    if self.parts.len() == 1 {
      self.parts[0].clone()
    } else {
      format!("::{}", self.parts.join("::"))
    }
  }

  pub fn includes(&self, other: &RustName) -> bool {
    let extra_modules_count = other.parts.len() as isize - self.parts.len() as isize;
    extra_modules_count > 0 && other.parts[0..self.parts.len()] == self.parts[..]
  }

  pub fn includes_directly(&self, other: &RustName) -> bool {
    let extra_modules_count = other.parts.len() as isize - self.parts.len() as isize;
    self.includes(other) && extra_modules_count == 1
  }
}

trait ToRustName {
  fn to_rust_name() -> Result<RustName>;
}

impl ToRustName for u8 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["u8".to_string()])
  }
}
impl ToRustName for i8 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["i8".to_string()])
  }
}
impl ToRustName for u16 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["u16".to_string()])
  }
}
impl ToRustName for i16 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["i16".to_string()])
  }
}
impl ToRustName for u32 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["u32".to_string()])
  }
}
impl ToRustName for i32 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["i32".to_string()])
  }
}
impl ToRustName for u64 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["u64".to_string()])
  }
}
impl ToRustName for i64 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["i64".to_string()])
  }
}
impl ToRustName for f32 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["f32".to_string()])
  }
}
impl ToRustName for f64 {
  fn to_rust_name() -> Result<RustName> {
    RustName::new(vec!["f64".to_string()])
  }
}







impl RustType {
  #[allow(dead_code)]
  pub fn caption(&self, context: &RustName) -> Result<String> {
    Ok(match *self {
         RustType::Void => "void".to_string(),
         RustType::Common {
           ref base,
           ref generic_arguments,
           ref is_const,
           ref is_const2,
           ref indirection,
         } => {

      let mut name = if base.parts.len() == 1 {
        base.parts[0].to_snake_case()
      } else {
        let mut remaining_context: &[String] = &context.parts;
        let mut parts: &[String] = &base.parts;
        if &parts[0] == "libc" {
          parts = &parts[1..];
        };
        let mut good_parts = Vec::new();
        for part in parts {
          if !remaining_context.is_empty() && part == &remaining_context[0] {
            remaining_context = &remaining_context[1..];
          } else {
            remaining_context = &[];
            let snake_part = part.to_snake_case();
            if good_parts.last() != Some(&snake_part) {
              good_parts.push(snake_part);
            } else {
            }
          }
        }
        if good_parts.is_empty() {
          base.last_name()?.clone()
        } else {
          good_parts.join("_")
        }
      };
      if let Some(ref args) = *generic_arguments {
        name = format!("{}_{}",
                       name,
                       args
                         .iter()
                         .map_if_ok(|x| x.caption(context))?
                         .join("_"));
      }
      let mut_text = if *is_const { "" } else { "_mut" };
      match *indirection {
        RustTypeIndirection::None => {}
        RustTypeIndirection::Ref { .. } => {
          name = format!("{}{}_ref", name, mut_text);
        }
        RustTypeIndirection::Ptr => {
          name = format!("{}{}_ptr", name, mut_text);
        }
        RustTypeIndirection::PtrPtr => {
          let mut_text2 = if *is_const2 { "" } else { "_mut" };
          name = format!("{}{}_ptr{}_ptr", name, mut_text, mut_text2);
        }
        RustTypeIndirection::PtrRef { .. } => {
          let mut_text2 = if *is_const2 { "" } else { "_mut" };
          name = format!("{}{}_ptr{}_ref", name, mut_text, mut_text2);
        }
      }
      name
    }
         RustType::FunctionPointer { .. } => "fn".to_string(),
       })
  }

  #[allow(dead_code)]
  pub fn is_ref(&self) -> bool {
    match *self {
      RustType::Common { ref indirection, .. } => {
        match *indirection {
          RustTypeIndirection::Ref { .. } |
          RustTypeIndirection::PtrRef { .. } => true,
          _ => false,
        }
      }
      RustType::Void |
      RustType::FunctionPointer { .. } => false,
    }
  }

  pub fn with_lifetime(&self, new_lifetime: String) -> RustType {
    let mut r = self.clone();
    if let RustType::Common { ref mut indirection, .. } = r {
      match *indirection {
        RustTypeIndirection::Ref { ref mut lifetime } |
        RustTypeIndirection::PtrRef { ref mut lifetime } => *lifetime = Some(new_lifetime),
        _ => {}
      }
    }
    r
  }

  pub fn lifetime(&self) -> Option<&String> {
    match *self {
      RustType::Common { ref indirection, .. } => {
        match *indirection {
          RustTypeIndirection::Ref { ref lifetime } |
          RustTypeIndirection::PtrRef { ref lifetime } => lifetime.as_ref(),
          _ => None,
        }
      }
      _ => None,
    }
  }
  pub fn last_is_const(&self) -> Result<bool> {
    if let RustType::Common {
             ref is_const,
             ref is_const2,
             ref indirection,
             ..
           } = *self {
      match *indirection {
        RustTypeIndirection::PtrPtr { .. } |
        RustTypeIndirection::PtrRef { .. } => Ok(*is_const2),
        _ => Ok(*is_const),
      }
    } else {
      Err("not a Common type".into())
    }
  }
  pub fn is_const(&self) -> Result<bool> {
    match *self {
      RustType::Common { ref is_const, .. } => Ok(*is_const),
      _ => Err("not a Common type".into()),
    }
  }
  pub fn set_const(&mut self, value: bool) -> Result<()> {
    match *self {
      RustType::Common { ref mut is_const, .. } => {
        *is_const = value;
        Ok(())
      }
      _ => Err("not a Common type".into()),
    }
  }

  pub fn is_unsafe_argument(&self) -> bool {
    match *self {
      RustType::Common {
        ref indirection,
        ref base,
        ref generic_arguments,
        ..
      } => {
        match *indirection {
          RustTypeIndirection::None |
          RustTypeIndirection::Ref { .. } => {}
          RustTypeIndirection::Ptr |
          RustTypeIndirection::PtrPtr |
          RustTypeIndirection::PtrRef { .. } => {
            return true;
          }
        }
        if base.full_name(None) == "std::option::Option" {
          if let Some(ref args) = *generic_arguments {
            if let Some(ref arg) = args.get(0) {
              if arg.is_unsafe_argument() {
                return true;
              }
            }
          }
        }
        false
      }
      RustType::Void => false,
      RustType::FunctionPointer { .. } => true,
    }
  }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RustFFIArgument {
  pub name: String,
  pub argument_type: RustType,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RustFFIFunction {
  pub return_type: RustType,
  pub name: String,
  pub arguments: Vec<RustFFIArgument>,
}

impl CompleteType {
  pub fn ptr_to_ref(&self, is_const1: bool) -> Result<CompleteType> {
    let mut r = self.clone();
    if let RustType::Common {
             ref mut is_const,
             ref mut indirection,
             ..
           } = r.rust_api_type {
      if *indirection != RustTypeIndirection::Ptr {
        return Err("not a pointer type".into());
      }
      *indirection = RustTypeIndirection::Ref { lifetime: None };
      *is_const = is_const1;
    } else {
      return Err("not a RustType::Common".into());
    }
    if r.rust_api_to_c_conversion != RustToCTypeConversion::None {
      return Err("rust_api_to_c_conversion is not none".into());
    }
    r.rust_api_to_c_conversion = RustToCTypeConversion::RefToPtr;
    Ok(r)
  }
  pub fn ptr_to_value(&self) -> Result<CompleteType> {
    let mut r = self.clone();
    if let RustType::Common {
             ref mut is_const,
             ref mut indirection,
             ..
           } = r.rust_api_type {
      if *indirection != RustTypeIndirection::Ptr {
        return Err("not a pointer type".into());
      }
      *indirection = RustTypeIndirection::None;
      *is_const = true;
    } else {
      return Err("not a RustType::Common".into());
    }
    if r.rust_api_to_c_conversion != RustToCTypeConversion::None {
      return Err("rust_api_to_c_conversion is not none".into());
    }
    r.rust_api_to_c_conversion = RustToCTypeConversion::ValueToPtr;
    Ok(r)
  }
}
