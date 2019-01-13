//! Implementation of `cpp_to_rust` generator that
//! analyzes a C++ library and produces a Rust crate for it.
//! See [README]
//! (https://github.com/rust-qt/cpp_to_rust/tree/master/cpp_to_rust/cpp_to_rust_generator)
//! for more information.

#![deny(unused_must_use)]

pub mod config;
pub mod cpp_checker;
mod cpp_code_generator;
pub mod cpp_data;
mod cpp_ffi_data;
pub mod cpp_ffi_generator;
pub mod cpp_function;
mod cpp_operator;
pub mod cpp_type;
pub mod database;
pub mod processor;
pub mod workspace;

mod crate_writer;
mod rust_name_resolver;

mod doc_formatter;
//mod launcher;
//mod rust_generator;
mod cpp_parser;
mod rust_code_generator;
mod rust_info;
mod rust_type;
mod versions;

mod cpp_template_instantiator;

#[cfg(test)]
mod tests;

mod cpp_explicit_destructors;
mod type_allocation_places;

// TODO: deal with inheritance for subclassing support
mod cpp_inheritance;
