use c_type::CTypeExtended;
use enums::CFunctionArgumentCppEquivalent;
use caption_strategy::ArgumentCaptionStrategy;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CFunctionArgument {
  pub name: String,
  pub argument_type: CTypeExtended,
  pub cpp_equivalent: CFunctionArgumentCppEquivalent,
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
