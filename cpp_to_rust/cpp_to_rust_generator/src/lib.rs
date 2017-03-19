// #![forbid(unused_must_use)]

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![cfg_attr(feature="clippy", warn(nonminimal_bool))]
#![cfg_attr(feature="clippy", warn(if_not_else))]
#![cfg_attr(feature="clippy", warn(shadow_same))]
#![cfg_attr(feature="clippy", warn(shadow_unrelated))]
#![cfg_attr(feature="clippy", warn(single_match_else))]
// some time in the future...
// #![warn(option_unwrap_used)]
// #![warn(result_unwrap_used)]
// #![warn(print_stdout)]

extern crate rustfmt;
extern crate tempdir;
extern crate regex;
extern crate clang;

extern crate cpp_to_rust_common as common;

mod cpp_ffi_generator;
mod cpp_code_generator;
mod caption_strategy;
pub mod config;
pub mod cpp_data;
mod cpp_ffi_data;
pub mod cpp_method;
pub mod cpp_type;
pub mod cpp_operator;
mod doc_formatter;
mod launcher;
mod rust_generator;
mod rust_code_generator;
mod rust_info;
mod rust_type;
mod cpp_parser;
mod serializable;
mod versions;

#[cfg(test)]
mod tests;

pub use launcher::is_completed;
