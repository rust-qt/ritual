
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ArgumentCaptionStrategy {
  NameOnly,
  TypeOnly,
  TypeAndName,
}

impl ArgumentCaptionStrategy {
  pub fn all() -> Vec<Self> {
    vec![ArgumentCaptionStrategy::NameOnly,
         ArgumentCaptionStrategy::TypeOnly,
         ArgumentCaptionStrategy::TypeAndName]
  }
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MethodCaptionStrategy {
  ArgumentsOnly(ArgumentCaptionStrategy),
  ConstOnly,
  ConstAndArguments(ArgumentCaptionStrategy),
}

impl MethodCaptionStrategy {
  pub fn all() -> Vec<Self> {
    let mut r = vec![];
    for i in ArgumentCaptionStrategy::all() {
      r.push(MethodCaptionStrategy::ArgumentsOnly(i));
    }
    r.push(MethodCaptionStrategy::ConstOnly);
    for i in ArgumentCaptionStrategy::all() {
      r.push(MethodCaptionStrategy::ConstAndArguments(i));
    }
    r
  }
}
