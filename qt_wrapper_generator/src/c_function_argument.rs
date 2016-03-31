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
  pub fn caption(&self, strategy: ArgumentCaptionStrategy) -> String {
    match strategy {
      ArgumentCaptionStrategy::NameOnly => self.name.clone(),
      ArgumentCaptionStrategy::TypeOnly(type_strategy) => self.argument_type.caption(type_strategy),
      ArgumentCaptionStrategy::TypeAndName(type_strategy) => {
        self.argument_type.caption(type_strategy) + &("_".to_string()) + &self.name
      }
    }
  }

  pub fn to_c_code(&self) -> String {
    self.argument_type.c_type.to_c_code() + &(" ".to_string()) + &self.name
  }
}
