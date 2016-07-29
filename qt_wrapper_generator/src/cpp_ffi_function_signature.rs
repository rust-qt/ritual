use cpp_ffi_function_argument::CppFfiFunctionArgument;
use caption_strategy::ArgumentCaptionStrategy;
use utils::JoinWithString;
use cpp_ffi_type::CppFfiType;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppFfiFunctionSignature {
  pub arguments: Vec<CppFfiFunctionArgument>,
  pub return_type: CppFfiType,
}

impl CppFfiFunctionSignature {
  pub fn caption(&self, strategy: ArgumentCaptionStrategy) -> String {
    let r = self.arguments
                .iter()
                .filter(|x| x.meaning.is_argument())
                .map(|x| x.caption(strategy.clone()))
                .join("_");
    if r.len() == 0 {
      "no_args".to_string()
    } else {
      r
    }
  }

  pub fn arguments_to_cpp_code(&self) -> Result<String, String> {
    let mut code = Vec::new();
    for arg in &self.arguments {
      match arg.to_cpp_code() {
        Ok(c) => code.push(c),
        Err(msg) => return Err(msg),
      }
    }
    Ok(code.join(", "))
  }
}
