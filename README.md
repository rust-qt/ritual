# ritual

[![Build Status](https://travis-ci.com/rust-qt/ritual.svg?branch=master)](https://travis-ci.com/rust-qt/ritual/branches)

`ritual` allows to use C++ libraries from Rust. This project provides Rust bindings to Qt.

## Using Qt from Rust

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

Each crate re-exports its depenencies, so, for example, you can access `qt_core` as `qt_widgets::qt_core` without adding an explicit dependency.

[Online documentation](https://rust-qt.github.io/rustdoc/qt/qt_core) of published Qt crates (you may also run `cargo doc --open` to generate documentation for your crate's dependencies).

## Repository structure

- `ritual` is the main part of the generator;
- `qt_ritual` provides Qt-specific generator features, generator configuration for the Qt crates, and a binary file for running the generator.
- `cpp_utils` provides essential utilities used by generated crates;
- `ritual_build` and `qt_ritual_build` provide build scripts for the generated crates;
- `ritual_common` and `qt_ritual_common` contain common functionality for other crates.

## Generating a crate for another C++ library

It's possible to use `ritual` on other C++ libraries, but the C++ standard library is not supported yet, so the options are limited at the moment.

To process your library, create a binary crate that depends on `ritual`, create a `GlobalConfig` and pass it to `ritual::cli::run_from_args` function. An example will be provided in the future.



## Dependencies

It's recommended to use latest stable Rust version. Compatibility with older versions is not guaranteed.

Qt crates require:

- A C++ build toolchain, compatible with the Rust toolchain in use:
  - On Linux: `make` and `g++`;
  - On Windows: MSVC (Visual Studio or build tools) or MinGW environment;
  - On OS X: the command line developer tools (full Xcode installation is not required);
- [Qt](https://www.qt.io/download);
- [cmake](https://cmake.org/).

Note that C++ toolchain, Rust toolchain, and Qt build must be compatible. For example, MSVC and MinGW targets on Windows are not compatible. 

`ritual` requires:

- A C++ build toolchain, compatible with the Rust toolchain in use:
  - On Linux: `make` and a C++ compiler;
  - On Windows: MSVC (Visual Studio or build tools) or MinGW environment;
  - On OS X: the command line developer tools (full Xcode installation is not required);
- The target C++ library (include and library files);
- [cmake](https://cmake.org/) ≥ 3.0;
- `libclang-dev` ≥ 3.5;
- `libsqlite3-dev` (only for `qt_ritual`).

Note that C++ toolchain, Rust toolchain, and Qt build must be compatible. For example, MSVC and MinGW targets on Windows are not compatible. 

The following environment variables may be required for `clang` parser to work correctly:

- `LLVM_CONFIG_PATH` (path to `llvm-config` binary)
- `CLANG_SYSTEM_INCLUDE_PATH` (e.g. `$CLANG_DIR/lib/clang/3.8.0/include` for `clang` 3.8.0).

If `libsqlite3` is not installed system-wide, setting `SQLITE3_LIB_DIR` environment variable may be required.

Run `cargo test` to make sure that dependencies are set up correctly.

## Environment variables

`ritual` may require `CLANG_SYSTEM_INCLUDE_PATH` environment variable set to path to `clang`'s system headers, e.g. `/usr/lib/llvm-3.8/lib/clang/3.8.0/include`. Without it, parsing may abort with an error like this:

```
fatal error: 'stddef.h' file not found
```

`RITUAL_TEMP_TEST_DIR` variable may be used to specify location of the temporary directory used by tests. If the directory is preserved between test runs, tests will run faster.

Build scripts of generated crates accept `RITUAL_LIBRARY_PATH`, `RITUAL_FRAMEWORK_PATH`, `RITUAL_INCLUDE_PATH` environment variables. They can be used to override paths selected by the build script (if any). If multiple paths need to be specified, separate them in the same way `PATH` variable is separated on your platform. 

C++ build tools and the linker may also read other environment variables, including `LIB`, `PATH`, `LIBRARY_PATH`, `LD_LIBRARY_PATH`, `DYLD_FRAMEWORK_PATH`. The generator has API for specifying library paths, passes them to `cmake` when building the C++ wrapper library, and reports the paths in build script's output, but it may not be enough for the linker to find the library, so you may need to set them manually.

## Generator workflow

The generator itself (`ritual`) is a library which exposes API for configurating different aspects of the process. In order to run the generator and produce an output crate, one must use a binary crate (such as `qt_ritual`) and launch the generator using its API.

The generator runs in the following steps:

1. If the target library has any dependencies which were already processed and converted to Rust crates, information collected during their generation is loaded from the cache directory and used for further processing.
1. `clang` C++ parser is executed to extract information about the library's types and methods from its header files.
1. The detected methods are checked to filter out invalid parse results.
1. A C++ wrapper library with C-compatible interface is generated. The library exposes each found method using a wrapper function.
1. A Rust code for the crate is generated. Functions from the C++ wrapper library are made available in the crate using Rust's [FFI support](https://doc.rust-lang.org/book/ffi.html). Rust code also contains `struct`s for all found C++ enums, structs and classes (including instantiations of template classes).
1. C++ library documentation (if available) and `ritual`'s processing data are used to generate a full-featured documentation for the crate ([example](https://rust-qt.github.io/rustdoc/qt/qt_core/index.html)).
1. The Rust code is saved to the output directory along with any extra files (tests, examples, etc.) provided by the caller. A build script necessary for building the crate is also attached.
1. Internal information of the generator is written to the database file and can be used when processing the library's dependants.

## C++/Rust features coverage

Supported features:

- Primitive types are mapped to Rust's primitive types (like `bool`) and FFI types (like `c_int`).
- Fixed-size numeric types (e.g `int8_t` or `qint8`) are mapped to Rust's fixed size types (e.g. `i8`).
- Pointers, references and values are mapped to special smart pointer types (`Ref`, `Ptr`, `CppBox`, etc.) provided by the `cpp_utils` crate.
- C++ namespaces are mapped to Rust modules.
- C++ classes, structs, and enums are mapped to Rust structs. This also applies to all instantiations of template classes encountered in the library's API, including template classes of dependencies.
- Free functions are mapped to free functions.
- Class methods are mapped to structs' implementations.
- Destructors are mapped to `CppDeletable` implementations and can be automatically invoked by `CppBox`.
- Function pointer types are mapped to Rust's equivalent representation. Function pointers with references or class values are not supported.
- `static_cast` and `dynamic_cast` are available in Rust through corresponding traits.
- Methods inherited from base classes are available via `Deref` implementation (if the class has multiple bases, only the first base's methods are directly available).
- Getter and setter methods are created for each public class field.
- Operators are translated to Rust's operator trait implementations when possible.

Names of Rust identifiers are modified according to Rust's naming conventions.

Not implemented yet but planned:

- Translate C++ `typedef`s to Rust type aliases.
- Implement `Debug` and `Display` traits for structs if applicable methods exist on C++ side.
- Implement iterator traits for collections.
- ([Implement subclassing API](https://github.com/rust-qt/ritual/issues/26)).

Not planned to support:

- Advanced template usage, like types with integer template arguments.
- Template partial specializations.

## Qt-specific features coverage

Implemented: 

- `QFlags<Enum>` types are converted to Rust's own similar implementation located at `qt_core::flags`).
- `qt_core` implements a way to use signals and slots. It's possible to use signals and slots of the built-in Qt classes and create slots bound to an arbitrary closure from Rust code. Argument types compatibility is checked at compile time.

Not implemented yet but planned:

- Creating custom signals from Rust code.

## Platform support

Linux, OS X and Windows are supported. `ritual` and Qt crates are [continuously tested on Travis](https://travis-ci.com/rust-qt/ritual/branches).

## Remarks

### Expressing library dependencies

`ritual` takes advantage of Rust's crate system. If a C++ library depends on another C++ library, generated Rust crate will also depend on the dependency's crate and reuse its types.

### Documentation generation

Documentation is important! `ritual` generates `rustdoc` comments with information about corresponding C++ types and methods. Overloaded methods have detailed documentation listing all available variants. Qt documentation is integrated in `rustdoc` comments.

### API stability

`ritual` is in active development, and its own API is not stable yet. That shouldn't be a big issue, because it's just a development tool, and the amount of code using the generator's API should be fairly small.

`ritual` also can't provide API stability of the generated crate. It's possible (and currently highly likely) that the generated API will significantly change when upgrading to a newer `ritual` version.

### Safety

It's impossible to bring Rust's safety to C++ APIs automatically, so most of the generated APIs are very unsafe to use and require thinking in C++ terms. Most of the generated functions are unsafe because raw pointers are not guaranteed to be valid, and most functions dereference some pointers.

### Executable size

If Rust crates and C++ wrapper libraries are all built statically, the linker only runs once for the final executable that uses the crates. It should be able to eliminate all unused wrapper functions and produce a reasonably small file that will only depend on original C++ libraries.

## Generating Qt crates

If you want to generate Qt crates from scratch, clone the project and run `qt_ritual`:

```
git clone https://github.com/rust-qt/ritual.git
cd ritual
cargo run --release --bin qt_ritual -- /path/to/workspace -c qt_core -o main
```

The workspace directory will be used for storing databases, temporary files, and the generated crates. Use the same workspace directory for all Qt crates to make sure that `ritual` can use types from previously generated crates.

`qmake` of the target Qt installation must be available in `PATH`.

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

- Submit a bug report, a feature request, or an improvement suggestion at the [issue tracker](https://github.com/rust-qt/ritual/issues);
- Suggest a C++ library to adapt;
- Write a test or an example for a Qt crate (porting examples from the official Qt documentation is a good option);
- Pick up an issue with [help wanted](https://github.com/rust-qt/ritual/labels/help%20wanted) tag.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
