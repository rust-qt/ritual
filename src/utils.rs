
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












