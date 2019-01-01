#![recursion_limit = "1024"] // for error_chain

//! Utility types and functions used by `cpp_to_rust_generator` and
//! `cpp_to_rust_build_tools` crates.
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust)
//! for more information.
//!
extern crate backtrace;
extern crate bincode;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
extern crate num_cpus;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate term_painter;
pub extern crate toml;

pub mod errors;
pub mod log;

pub mod file_utils;
pub mod string_utils;
pub mod utils;

pub mod cpp_build_config;
pub mod cpp_lib_builder;
pub mod target;

/// This type contains data serialized by the generator and placed to the
/// generated crate's directory. The build script reads and uses this value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildScriptData {
    /// Information required to build the C++ wrapper library
    pub cpp_build_config: cpp_build_config::CppBuildConfig,
    /// Name of the original C++ library passed to the generator
    pub cpp_lib_version: Option<String>,
    /// Name of C++ wrapper library
    pub cpp_wrapper_lib_name: String,
}

#[cfg(test)]
mod tests;
