#![recursion_limit = "1024"] // for error_chain

//! Utility types and functions used by `cpp_to_rust_generator` and
//! `cpp_to_rust_build_tools` crates.


#[macro_use]
extern crate error_chain;
extern crate backtrace;
extern crate regex;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate bincode;
extern crate term_painter;
extern crate num_cpus;
#[macro_use]
extern crate lazy_static;
pub extern crate toml;

pub mod log;
pub mod errors;

pub mod file_utils;
pub mod string_utils;
pub mod utils;

pub mod cpp_build_config;
pub mod cpp_lib_builder;
pub mod target;

pub use serializable::BuildScriptData;


mod serializable;

#[cfg(test)]
mod tests;
