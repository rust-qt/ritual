use enums::{IndirectionChange};


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CType {
  pub is_pointer: bool,
  pub is_const: bool,
  pub base: String,
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppToCTypeConversion {
  pub indirection_change: IndirectionChange,
  pub renamed: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CTypeExtended {
  pub c_type: CType,
  pub conversion: CppToCTypeConversion,
}

impl CTypeExtended {
  pub fn void() -> Self {
    CTypeExtended {
      c_type: CType::void(),
      conversion: CppToCTypeConversion {
        indirection_change: IndirectionChange::NoChange,
        renamed: false,
      },
    }
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
