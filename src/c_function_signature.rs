use c_function_argument::CFunctionArgument;
use c_type::CTypeExtended;
use caption_strategy::ArgumentCaptionStrategy;
use utils::JoinWithString;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CFunctionSignature {
  pub arguments: Vec<CFunctionArgument>,
  pub return_type: CTypeExtended,
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
