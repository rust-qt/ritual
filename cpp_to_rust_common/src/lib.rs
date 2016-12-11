#![recursion_limit = "1024"] // for error_chain
#[macro_use]
extern crate error_chain;
extern crate backtrace;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate ansi_term;

pub mod log;
pub mod errors;

pub mod file_utils;
pub mod string_utils;
pub mod utils;

pub mod cpp_lib_builder;
