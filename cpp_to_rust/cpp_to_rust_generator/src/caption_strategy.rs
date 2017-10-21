
/// Mode used by `CppType::caption` to generate
/// type captions for C++ types
/// (used to generate FFI function names).
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TypeCaptionStrategy {
  /// Only base type is used
  Short,
  /// Base type, constness and indirection are used
  Full,
}

/// Mode used to generate C++ argument names
/// (used to generate FFI function names).
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ArgumentCaptionStrategy {
  /// Only the name of the argument
  NameOnly,
  /// Only the type of the argument
  /// (using specified type caption strategy)
  TypeOnly(TypeCaptionStrategy),
  /// Both type and name of the argument
  /// (using specified type caption strategy)
  TypeAndName(TypeCaptionStrategy),
}

impl ArgumentCaptionStrategy {
  /// Returns list of all available strategies
  /// (sorted from high to low priority)
  pub fn all() -> Vec<Self> {
    vec![
      ArgumentCaptionStrategy::NameOnly,
      ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Short),
      ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Short),
      ArgumentCaptionStrategy::TypeOnly(TypeCaptionStrategy::Full),
      ArgumentCaptionStrategy::TypeAndName(TypeCaptionStrategy::Full),
    ]
  }
}

/// Mode of generating FFI method captions
/// in case of name conflict
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MethodCaptionStrategy {
  /// Use arguments caption
  ArgumentsOnly(ArgumentCaptionStrategy),
  /// Use method constness
  ConstOnly,
  /// Use both arguments caption and method constness
  ConstAndArguments(ArgumentCaptionStrategy),
}

impl MethodCaptionStrategy {
  /// Returns list of all available strategies
  /// (sorted from high to low priority)
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
