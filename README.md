# cpp_to_rust

[![Linux + OS X](https://travis-ci.org/rust-qt/cpp_to_rust.svg?branch=master)](https://travis-ci.org/rust-qt/cpp_to_rust)
[![Windows](https://ci.appveyor.com/api/projects/status/m4yo29j2f5wfu3w0/branch/master?svg=true)](https://ci.appveyor.com/project/Riateche/cpp-to-rust)

`cpp_to_rust` allows to use C++ libraries from Rust. The main target of this project is Qt.

## Using published Qt crates

This project maintains the following Qt crates (more will hopefully be added in the future):

| Crate       | Version |
| ----------- | ------- |
| qt_core     | [![](http://meritbadge.herokuapp.com/qt_core)](https://crates.io/crates/qt_core) |
| qt_gui      | [![](http://meritbadge.herokuapp.com/qt_gui)](https://crates.io/crates/qt_gui) |
| qt_widgets  | [![](http://meritbadge.herokuapp.com/qt_widgets)](https://crates.io/crates/qt_widgets) |
| qt_ui_tools | [![](http://meritbadge.herokuapp.com/qt_ui_tools)](https://crates.io/crates/qt_ui_tools) |
| qt_3d_core | [![](http://meritbadge.herokuapp.com/qt_3d_core)](https://crates.io/crates/qt_3d_core) |
| qt_3d_render | [![](http://meritbadge.herokuapp.com/qt_3d_render)](https://crates.io/crates/qt_3d_render) |
| qt_3d_input | [![](http://meritbadge.herokuapp.com/qt_3d_input)](https://crates.io/crates/qt_3d_input) |
| qt_3d_logic | [![](http://meritbadge.herokuapp.com/qt_3d_logic)](https://crates.io/crates/qt_3d_logic) |
| qt_3d_extras | [![](http://meritbadge.herokuapp.com/qt_3d_extras)](https://crates.io/crates/qt_3d_extras) |

If you just want to use these crates, add them as dependencies to your `Cargo.toml`, for example:

```
[dependencies]
qt_widgets = "0.2"
```

And add corresponding `extern crate` directives to the crate root (`main.rs` or `lib.rs`):

```
extern crate qt_widgets;
```

Each crate re-exports its dependencies, so, for example, you can access `qt_core` as `qt_widgets::qt_core` without adding an explicit dependency.

[Online documentation](https://rust-qt.github.io/rustdoc/qt/qt_core) of published Qt crates (you may also run `cargo doc --open` to generate documentation for your crate's dependencies).

Published crates required a certain Qt version (currently 5.8.0) and don't export platform-specific API.

## Using the generator

If you want to use another Qt version, access platform-specific Qt APIs or tweak the generator configuration, refer to README of [qt_generator](https://github.com/rust-qt/cpp_to_rust/tree/master/qt_generator/qt_generator) for more information.

If you want to generate Rust crates for another C++ library or learn about implementation details, see README of [cpp_to_rust_generator](https://github.com/rust-qt/cpp_to_rust/tree/master/cpp_to_rust/cpp_to_rust_generator).

## Repository structure

The project consists of the following Rust crates:

- [cpp_to_rust/cpp_to_rust_generator](https://github.com/rust-qt/cpp_to_rust/tree/master/cpp_to_rust/cpp_to_rust_generator) implements the generator;
- [cpp_to_rust/cpp_to_rust_build_tools](https://github.com/rust-qt/cpp_to_rust/tree/master/cpp_to_rust/cpp_to_rust_build_tools) implements the build script for generated crates;
- [cpp_to_rust/cpp_to_rust_common](https://github.com/rust-qt/cpp_to_rust/tree/master/cpp_to_rust/cpp_to_rust_common) contains common code for the previous two crates;
- [cpp_to_rust/cpp_utils](https://github.com/rust-qt/cpp_to_rust/tree/master/cpp_to_rust/cpp_utils) provides essential utilities used by generated crates;
- [qt_generator/qt_generator](https://github.com/rust-qt/cpp_to_rust/tree/master/qt_generator/qt_generator) contains generator configuration for the Qt crates and a binary file for running the generator;
- [qt_generator/qt_build_tools](https://github.com/rust-qt/cpp_to_rust/tree/master/qt_generator/qt_build_tools) implements the advanced build script for Qt crates;
- [qt_generator/qt_generator_common](https://github.com/rust-qt/cpp_to_rust/tree/master/qt_generator/qt_generator_common) contains common code for the previous two crates.

See `README.md` in each crate for detailed description.

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

If you use Qt, you should also take into account [Qt licensing](https://www.qt.io/licensing/).

## Contributing

Contributions are always welcome! You can contribute in different ways:

- Suggest a C++ library to adapt;
- Submit a bug report, a feature request, or an improvement suggestion at the [issue tracker](https://github.com/rust-qt/cpp_to_rust/issues);
- Write a test for `cpp_to_rust_generator` crate;
- Write a test or an example for a Qt crate (porting examples from the official Qt documentation is a good option);
- Pick up an issue with [help wanted](https://github.com/rust-qt/cpp_to_rust/labels/help%20wanted) tag or any other issue you like.

Please use `develop` as the target branch for your pull requests.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the project by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
