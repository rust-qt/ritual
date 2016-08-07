use caption_strategy::ArgumentCaptionStrategy;
use cpp_ffi_type::CppFfiType;
use cpp_type::{CppTypeBase};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CppFfiArgumentMeaning {
  This,
  Argument(i8),
  ReturnValue,
}

impl CppFfiArgumentMeaning {
  pub fn is_argument(&self) -> bool {
    match self {
      &CppFfiArgumentMeaning::Argument(..) => true,
      _ => false,
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CppFfiFunctionArgument {
  pub name: String,
  pub argument_type: CppFfiType,
  pub meaning: CppFfiArgumentMeaning,
}

impl CppFfiFunctionArgument {
  pub fn caption(&self, strategy: ArgumentCaptionStrategy) -> String {
    match strategy {
      ArgumentCaptionStrategy::NameOnly => self.name.clone(),
      ArgumentCaptionStrategy::TypeOnly(type_strategy) => {
        self.argument_type.original_type.caption(type_strategy)
      }
      ArgumentCaptionStrategy::TypeAndName(type_strategy) => {
        format!("{}_{}",
                self.argument_type.original_type.caption(type_strategy),
                self.name)
      }
    }
  }

  pub fn to_cpp_code(&self) -> Result<String, String> {
    let type_text = try!(self.argument_type.ffi_type.to_cpp_code());
    match self.argument_type.ffi_type.base {
      CppTypeBase::FunctionPointer { .. } => Ok(type_text.replace("FN_PTR", &self.name)),
      _ => Ok(format!("{} {}", type_text, self.name))
    }
  }
}
