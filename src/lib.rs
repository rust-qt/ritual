#![forbid(unused_must_use)]

mod cpp_ffi_generator;
mod cpp_code_generator;
mod caption_strategy;
mod cpp_data;
mod cpp_ffi_data;
mod cpp_method;
mod cpp_type;
mod cpp_operator;
pub mod log;
mod qt_doc_parser;
mod qt_specific;
mod rust_generator;
mod rust_code_generator;
mod rust_info;
mod rust_type;
mod utils;
mod cpp_parser;
mod serializable;
pub mod launcher;

#[cfg(test)]
mod tests;
