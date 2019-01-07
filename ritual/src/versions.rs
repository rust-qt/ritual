//! Versions of the dependencies of the generated crate.
//!
//! These versions are used in `Cargo.toml` of the generated crate.
//!
//! It's not easy to determine the versions automatically
//! because the build tools crate is not even a part of the generator build.
//! Make sure to update versions here when the actual version of the dependency changes.

/// Version of `cpp_to_rust_build_tools` crate.
pub const BUILD_TOOLS_VERSION: &str = "0.2.3";

/// Version of `cpp_utils` crate.
pub const CPP_UTILS_VERSION: &str = "0.2.0";

/// Version of `libc` crate.
pub const LIBC_VERSION: &str = "0.2";
