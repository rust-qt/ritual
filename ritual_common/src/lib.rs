#![allow(clippy::cyclomatic_complexity)]

//! Utility types and functions used by `cpp_to_rust_generator` and
//! `cpp_to_rust_build_tools` crates.
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust)
//! for more information.

use crate::target::LibraryTarget;
use serde_derive::{Deserialize, Serialize};

pub mod cpp_build_config;
pub mod cpp_lib_builder;
pub mod env_var_names;
pub mod errors;
pub mod file_utils;
pub mod string_utils;
pub mod target;
pub mod utils;

pub use toml;

/// This type contains data serialized by the generator and placed to the
/// generated crate's directory. The build script reads and uses this value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildScriptData {
    /// Information required to build the C++ wrapper library
    pub cpp_build_config: cpp_build_config::CppBuildConfig,
    /// Name of C++ wrapper library
    pub cpp_wrapper_lib_name: String,
    /// Environments the generator was used in
    pub known_targets: Vec<LibraryTarget>,
}

#[cfg(test)]
mod tests;
