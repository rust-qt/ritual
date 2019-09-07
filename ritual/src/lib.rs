//! Implementation of `cpp_to_rust` generator that
//! analyzes a C++ library and produces a Rust crate for it.
//! See [README]
//! (https://github.com/rust-qt/cpp_to_rust/tree/master/cpp_to_rust/cpp_to_rust_generator)
//! for more information.

pub mod cli;
pub mod cluster_api;
pub mod config;
mod cpp_casts;
pub mod cpp_checker;
mod cpp_checks;
mod cpp_code_generator;
pub mod cpp_data;
pub mod cpp_ffi_data;
pub mod cpp_ffi_generator;
pub mod cpp_function;
mod cpp_implicit_methods;
mod cpp_inheritance; // TODO: deal with inheritance for subclassing support
mod cpp_omitting_arguments;
mod cpp_operator;
pub mod cpp_parser;
pub mod cpp_template_instantiator;
pub mod cpp_type;
mod crate_writer;
pub mod database;
mod doc_formatter;
pub mod processor;
mod rust_code_generator;
mod rust_generator;
pub mod rust_info;
pub mod rust_type;
mod type_allocation_places;
mod versions;
pub mod workspace;

#[cfg(test)]
mod tests;
