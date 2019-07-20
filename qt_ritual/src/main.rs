//! Generator of Rust-Qt crates.
//!
//! See [README](https://github.com/rust-qt/ritual) of the repository root for more information.

#![allow(clippy::collapsible_if)]

use qt_ritual::lib_configs::global_config;
use ritual::cli;
use ritual_common::errors::FancyUnwrap;

pub fn main() {
    cli::run_from_args(global_config()).fancy_unwrap();
}
