#![cfg_attr(feature="clippy", allow(redundant_closure))]

use std;
use cpp_type::{CppType, CppTypeIndirection, CppBuiltInNumericType};
use cpp_ffi_data::{CppFfiFunctionArgument, CppFfiType};

error_chain! {
  foreign_links {
    std::io::Error, IO;
  }

  errors {
    QMakeQueryFailed
    CMakeFailed
    MakeFailed
    CargoFailed
    CWrapperBuildFailed
    SourceDirDoesntExist(path: String) {
      display("source dir doesn't exist: {:?}", path)
    }
    JoinPathsFailed
    AddEnvFailed

    CppTypeToCodeFailed(t: CppType) {
      display("failed to_cpp_code({:?})", t)
    }
    FfiArgumentToCodeFailed(arg: CppFfiFunctionArgument) {
      display("failed to_cpp_code({:?})", arg)
    }
    TemplateArgsCountMismatch {
      display("template arguments count mismatch")
    }
    ExtraTemplateParametersLeft(text: String) {
      display("found remaining template parameters: {}", text)
    }
    NotEnoughTemplateArguments
    TypeNotAvailable(t: CppType) {
      display("type is not available: {}", t.to_cpp_pseudo_code())
    }
    NotApplicableAllocationPlaceInConstructor
    TooMuchIndirection { left: CppTypeIndirection, right: CppTypeIndirection } {
      display("too much indirection: {:?} to {:?}", left, right)
    }
    UnexpectedFunctionPointerInnerText
    FunctionPointerInnerTextMissing
    VariadicFunctionPointer {
      display("function pointers with variadic arguments are not supported")
    }
    TemplateFunctionPointer {
      display("function pointers containing template parameters are not supported")
    }
    NestedFunctionPointer {
      display("Function pointers containing nested function pointers are not supported")
    }
    FunctionPointerWithReference {
      display("Function pointers containing references are not supported")
    }
    FunctionPointerWithClassValue {
      display("Function pointers containing classes by value are not supported")
    }
    TemplateParameterToCodeAttempt {
      display("template parameters are not allowed to produce C++ code without instantiation")
    }
    TemplateParameterToFFIAttempt {
      display("template parameters cannot be expressed in FFI")
    }
    RValueReference {
      display("rvalue references are not supported")
    }
    QFlagsInvalidIndirection {
      display("only value or const reference is allowed for QFlags type")
    }
    NoRustType(name: String) {
      display("type has no Rust equivalent: {}", name)
    }
    InvalidFfiIndirection(t: CppTypeIndirection) {
      display("unsupported indirection for FFI type: {:?}", t)
    }
    UnsupportedNumericType(t: CppBuiltInNumericType) {
      display("unsupported numeric type: {:?}", t)
    }
    TypeToCompleteFailed(t: CppFfiType) {
      display("failed: complete_type({:?})", t)
    }
    CppCodeGeneratorFailed

    StackAllocatedNonVoidWrapper {
      display("stack allocated wrappers are expected to return void")
    }
    ValueToPointerConflictsWithNotApplicable {
      display("ValueToPointer conflicts with NotApplicable")
    }
    NoThisInDestructor
    NoThisInMethod
    NoReturnValueArgument
    Unexpected(msg: &'static str) {
      display("{}", msg)
    }

  }
}


impl Error {
  pub fn is_unexpected(&self) -> bool {
    use self::ErrorKind::*;
    match *self.kind() {
      Unexpected(..) |
      StackAllocatedNonVoidWrapper |
      ValueToPointerConflictsWithNotApplicable |
      NoThisInDestructor |
      NoThisInMethod |
      NoReturnValueArgument |
      NotApplicableAllocationPlaceInConstructor => true,
      _ => false,
    }
  }
  pub fn discard_expected(&self) {
    if self.is_unexpected() {
      self.display_report();
      // TODO: don't panic on this in production
      panic!("unexpected error");
    }
  }
}
