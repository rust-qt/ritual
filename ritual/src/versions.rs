//! Versions of the dependencies of the generated crate.
//!
//! These versions are used in `Cargo.toml` of the generated crate.
//!
//! It's not easy to determine the versions automatically
//! because the build tools crate is not even a part of the generator build.
//! Make sure to update versions here when the actual version of the dependency changes.

/// Version of `ritual_build` crate.
pub const RITUAL_BUILD_VERSION: &str = "0.1.1";

/// Version of `cpp_core` crate.
pub const CPP_CORE_VERSION: &str = "0.3.0";
