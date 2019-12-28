//! Utility types and functions used by `ritual` and
//! `ritual_build` crates.
//!
//! See [README](https://github.com/rust-qt/ritual) of the repository root for more information.

#![forbid(unsafe_code)]
#![allow(clippy::cognitive_complexity)]

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

use std::ops::Deref;
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

#[derive(Debug)]
pub struct ReadOnly<T>(T);

impl<T> ReadOnly<T> {
    pub fn new(value: T) -> Self {
        ReadOnly(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ReadOnly<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

#[cfg(test)]
mod tests;
