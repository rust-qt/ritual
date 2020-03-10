# ritual

[![Build Status](https://travis-ci.com/rust-qt/ritual.svg?branch=master)](https://travis-ci.com/rust-qt/ritual/branches)

`ritual` allows to use C++ libraries from Rust. It analyzes the C++ API of a library and generates a fully-featured crate that provides convenient (but still unsafe) access to this API.

The main motivation for this project is to provide access to Qt from Rust. Ritual provides large amount of automation, supports incremental runs, and implements compatible API evolution. This is mostly dictated by the huge size of API provided by Qt and significant API differences between Qt versions. However, ritual is designed to be universal and can also be used to easily create bindings for other C++ libraries.

# Examples and guides

- [How to use Qt from Rust](https://github.com/rust-qt/examples)
- [How to use ritual on a C++ library of your choice](https://github.com/rust-qt/generator-example)

The rest of this readme is focused on ritual's development.

# Repository structure

- `ritual` is the main part of the generator;
- `qt_ritual` provides Qt-specific generator features, generator configuration for the Qt crates, and a binary file for running the generator.
- `cpp_core` provides essential utilities used by generated crates;
- `ritual_build` and `qt_ritual_build` provide build scripts for the generated crates;
- `ritual_common` and `qt_ritual_common` contain common functionality for other crates.

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

If you use Qt, you should also take into account [Qt licensing](https://www.qt.io/licensing/).

# Contributing

Contributions are always welcome! You can contribute in different ways:

- Submit a bug report, a feature request, or an improvement suggestion at the [issue tracker](https://github.com/rust-qt/ritual/issues);
- Write a test or an example for a Qt crate (porting examples from the official Qt documentation is a good option);
- Pick up an issue with [help wanted](https://github.com/rust-qt/ritual/labels/help%20wanted) tag.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
