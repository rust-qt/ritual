
#[derive(Debug, PartialEq, Eq, Clone)]
enum ArgumentCaptionStrategy {
  NameOnly,
  TypeOnly,
  TypeAndName,
}

impl ArgumentCaptionStrategy {
  fn all() -> Vec<Self> {
    vec![ArgumentCaptionStrategy::NameOnly,
         ArgumentCaptionStrategy::TypeOnly,
         ArgumentCaptionStrategy::TypeAndName]
  }
}


#[derive(Debug, PartialEq, Eq, Clone)]
enum MethodCaptionStrategy {
  ArgumentsOnly(ArgumentCaptionStrategy),
  ConstOnly,
  ConstAndArguments(ArgumentCaptionStrategy),
}

impl MethodCaptionStrategy {
  fn all() -> Vec<Self> {
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
