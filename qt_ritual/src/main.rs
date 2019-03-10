//! Generator of Rust-Qt crates.
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust/tree/master/qt_generator/qt_generator)
//! for more information.

#![allow(clippy::collapsible_if)]

use crate::lib_configs::global_config;
use ritual::cli;
use ritual_common::errors::FancyUnwrap;

mod detect_signal_argument_types;
mod detect_signals_and_slots;
mod doc_decoder;
mod doc_parser;
mod fix_header_names;
mod lib_configs;
mod slot_wrappers;
mod versions;

#[cfg(test)]
mod test_moqt;

pub fn main() {
    cli::run_from_args(global_config()).fancy_unwrap();
}
