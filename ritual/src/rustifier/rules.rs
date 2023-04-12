use crate::{cpp_function::CppFunction, cpp_type::CppType};

#[derive(Debug, Default)]
pub struct Rules {
    pub types: Vec<TypeRule>,
}

#[derive(Debug)]
pub enum CppTypePosition {
    Argument,
    CppReturnType,
}

#[derive(Debug)]
pub struct TypeRule {
    pub cpp_type: CppType,
    pub cpp_type_position: CppTypePosition,
    pub rust_type: String,
    pub cpp_conversion: String,
    pub rust_conversion: String,
}

#[derive(Debug)]
pub struct FunctionRule {
    pub cpp_function: CppFunction,
}
