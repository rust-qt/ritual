
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

pub fn operator_c_name(cpp_name: &String, arguments_count: i32) -> Result<String, String> {
  let result = if cpp_name == "=" && arguments_count == 2 {
    "assign"
  } else if cpp_name == "+" && arguments_count == 2 {
    "add"
  } else if cpp_name == "-" && arguments_count == 2 {
    "sub"
  } else if cpp_name == "+" && arguments_count == 1 {
    "unary_plus"
  } else if cpp_name == "-" && arguments_count == 1 {
    "neg"
  } else if cpp_name == "*" && arguments_count == 2 {
    "mul"
  } else if cpp_name == "/" && arguments_count == 2 {
    "div"
  } else if cpp_name == "%" && arguments_count == 2 {
    "rem"
  } else if cpp_name == "++" && arguments_count == 1 {
    "inc"
  } else if cpp_name == "++" && arguments_count == 2 {
    "inc_postfix"
  } else if cpp_name == "--" && arguments_count == 1 {
    "dec"
  } else if cpp_name == "--" && arguments_count == 2 {
    "dec_postfix"
  } else if cpp_name == "==" && arguments_count == 2 {
    "eq"
  } else if cpp_name == "!=" && arguments_count == 2 {
    "neq"
  } else if cpp_name == ">" && arguments_count == 2 {
    "gt"
  } else if cpp_name == "<" && arguments_count == 2 {
    "lt"
  } else if cpp_name == ">=" && arguments_count == 2 {
    "ge"
  } else if cpp_name == "<=" && arguments_count == 2 {
    "le"
  } else if cpp_name == "!" && arguments_count == 1 {
    "not"
  } else if cpp_name == "&&" && arguments_count == 2 {
    "and"
  } else if cpp_name == "||" && arguments_count == 2 {
    "or"
  } else if cpp_name == "~" && arguments_count == 1 {
    "bit_not"
  } else if cpp_name == "&" && arguments_count == 2 {
    "bit_and"
  } else if cpp_name == "|" && arguments_count == 2 {
    "bit_or"
  } else if cpp_name == "^" && arguments_count == 2 {
    "bit_xor"
  } else if cpp_name == "<<" && arguments_count == 2 {
    "shl"
  } else if cpp_name == ">>" && arguments_count == 2 {
    "shr"
  } else if cpp_name == "+=" && arguments_count == 2 {
    "add_assign"
  } else if cpp_name == "-=" && arguments_count == 2 {
    "sub_assign"
  } else if cpp_name == "*=" && arguments_count == 2 {
    "mul_assign"
  } else if cpp_name == "/=" && arguments_count == 2 {
    "div_assign"
  } else if cpp_name == "%=" && arguments_count == 2 {
    "rem_assign"
  } else if cpp_name == "&=" && arguments_count == 2 {
    "bit_and_assign"
  } else if cpp_name == "|=" && arguments_count == 2 {
    "bit_or_assign"
  } else if cpp_name == "^=" && arguments_count == 2 {
    "bit_xor_assign"
  } else if cpp_name == "<<=" && arguments_count == 2 {
    "shl_assign"
  } else if cpp_name == ">>=" && arguments_count == 2 {
    "shr_assign"
  } else if cpp_name == "[]" && arguments_count == 2 {
    "index"
  } else if cpp_name == "()" && arguments_count == 1 {
    "call"
  } else if cpp_name == "," && arguments_count == 2 {
    "comma"
  } else {
    return Err(format!("unsupported operator {} (arguments count: {})",
                       cpp_name,
                       arguments_count));
  };
  return Ok(result.to_string());
}
