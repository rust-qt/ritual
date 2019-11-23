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

# Setting up environment

## Using docker (recommended)

To make sure the parsing results are consistent and reproducible, it's recommended to use a reproducible environment, such as provided by `docker`.

Ritual provides `Dockerfile`s containing its dependencies:

- `docker.builder.dockerfile` is the base image suitable for working on C++'s standard library. It also should be used as a base image when working on other C++ libraries.
- `docker.qt.dockerfile` is the image used for generating Qt crates. 

You can build the images with these commands:
```
cd ritual
# for any libraries
docker build . -f docker.builder.dockerfile -t ritual_builder

# only for Qt
docker build . -f docker.qt.dockerfile --target qt_downloader -t ritual_qt_downloader
docker build . -f docker.qt.dockerfile -t ritual_qt
```

Note that the image contains only the environment. No pre-built ritual artifacts are included. This allows you to edit the source code of your generator and re-run it without the slow process of rebuilding the docker image. You can use `cargo` to run the generator, just like you would normally do it on the host system.

When running the container, mount `/build` to a persistent directory on the host system. This directory will contain all temporary build files, so making it persistent will allow you to recreate the container without having to recompile everything from scratch.  

In addition to the build directory, you should also mount one or more directories containing the source code of your generator and the ritual workspace directory (see below) to make it available in the container. The paths to these directories can be arbitrary.

This is an example of command that runs a shell in the container:
```
docker run \
    --mount type=bind,source=~/ritual/repo,destination=/repo \
    --mount type=bind,source=~/ritual/qt_workspace,destination=/qt_workspace \
    --mount type=bind,source=~/ritual/tmp,destination=/build \
    --name ritual_qt \
    --hostname ritual_qt \
    -it \
    ritual_qt \
    bash
```

Use `cargo` to run the generator inside the container, just like in the host system.

## Without docker

In case you don't want or can't use `docker`, you can just install all required dependencies on your host system and run your generator natively with `cargo`, like any Rust project.

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

# Environment variables

`ritual` may require `CLANG_SYSTEM_INCLUDE_PATH` environment variable set to path to `clang`'s system headers, e.g. `/usr/lib/llvm-3.8/lib/clang/3.8.0/include`. Without it, parsing may abort with an error like this:

```
fatal error: 'stddef.h' file not found
```

`RITUAL_TEMP_TEST_DIR` variable may be used to specify location of the temporary directory used by tests. If the directory is preserved between test runs, tests will run faster.

`RITUAL_WORKSPACE_TARGET_DIR` variable overrides the `cargo`'s target directory when `ritual` runs `cargo` on the generated crates.

Build scripts of generated crates accept `RITUAL_LIBRARY_PATH`, `RITUAL_FRAMEWORK_PATH`, `RITUAL_INCLUDE_PATH` environment variables. They can be used to override paths selected by the build script (if any). If multiple paths need to be specified, separate them in the same way `PATH` variable is separated on your platform. Additionally, `RITUAL_CMAKE_ARGS` allows you to specify additional arguments passed to `cmake` when building C++ glue library.

C++ build tools and the linker may also read other environment variables, including `LIB`, `PATH`, `LIBRARY_PATH`, `LD_LIBRARY_PATH`, `DYLD_FRAMEWORK_PATH`. The generator has API for specifying library paths, passes them to `cmake` when building the C++ wrapper library, and reports the paths in build script's output, but it may not be enough for the linker to find the library, so you may need to set them manually.

# Generator workflow

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

# C++/Rust features coverage

Supported features:

- Primitive types are mapped to Rust's primitive types (like `bool`) and FFI types (like `c_int`).
- Fixed-size numeric types (e.g `int8_t` or `qint8`) are mapped to Rust's fixed size types (e.g. `i8`).
- Pointers, references and values are mapped to special smart pointer types (`Ref`, `Ptr`, `CppBox`, etc.) provided by the `cpp_core` crate.
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
- C++ STL-style iterators are accessible from Rust via adaptors.

Names of Rust identifiers are modified according to Rust's naming conventions.

Documentation is important! `ritual` generates `rustdoc` comments with information about corresponding C++ types and methods. Qt documentation is integrated in `rustdoc` comments.

Not implemented yet but can be implemented in the future:

- Translate C++ `typedef`s to Rust type aliases.
- Implement `Debug` and `Display` traits for structs if applicable methods exist on C++ side.
- ([Implement subclassing API](https://github.com/rust-qt/ritual/issues/26)).

Not planned to support:

- Advanced template usage, like types with integer template arguments.
- Template partial specializations.

# Qt-specific features coverage

Implemented: 

- `QFlags<Enum>` types are converted to Rust's own similar implementation located at `qt_core::flags`).
- `qt_core` implements a way to use signals and slots. It's possible to use signals and slots of the built-in Qt classes and create slots bound to an arbitrary closure from Rust code. Argument types compatibility is checked at compile time.

Not implemented yet but planned:

- Creating custom signals from Rust code.

# API stability and versioning

Ritual can analyze multiple different versions of the C++ library and generate a crate that supports all of them. Parts of the API that are common across versions are guaranteed to have the same Rust API as well. For parts of the API that are not always available, the Rust bindings will have feature attributes that only enable them if the current local version of the C++ library has them. Trying to use a feature not available in the installed version of C++ library will result in a compile-time error.

When a new version of the C++ library is released, ritual can preserve all existing API in the generated crate and add newly introduced API items under a feature flag. This allows to make semver-compatible changes to the generated crate to support all available versions of the C++ library. 

# Managing dependencies

C++, like most languages, allows libraries to use types from other libraries in their public API. When Rust bindings are generated, they should ideally reuse common dependencies instead of producing a copy of wrappers in each crate. Ritual supports exporting types from already processed dependencies and using them in the public API. 

If a ritual-based crate is published on `crates.io` and you want to use it as a dependency when generating your own bindings, ritual can export the information from it as well. This allows independent developers to base upon each other's work instead of repeating it. 

In addition to Qt crates, ritual project provides the `cpp_std` crate that provides access to C++'s standard library types. It should be used when processing a library that uses STL types in its API. However, `cpp_std` is still in early development and only provides access to a small part of the standard library.

# Platform support

Linux, macOS, and Windows are supported. `ritual` and Qt crates are [continuously tested on Travis](https://travis-ci.com/rust-qt/ritual/branches).

# Safety

It's impossible to bring Rust's safety to C++ APIs automatically, so most of the generated APIs are unsafe to use and require thinking in C++ terms. Most of the generated functions are unsafe because raw pointers are not guaranteed to be valid, and most functions dereference some pointers.

One of intended uses of ritual is to generate a low level interface and then write a safe interface on top of it (which can only be done manually). For huge libraries like Qt, when it's not feasible to design a safe fully-featured API for the whole library, it's recommended to contain unsafe usage in a module and implement a safe interface for the parts of API required for your project. 

# Executable size

If Rust crates and C++ wrapper libraries are all built statically, the linker only runs once for the final executable that uses the crates. It should be able to eliminate all unused wrapper functions and produce a reasonably small file that will only depend on original C++ libraries.

# Generating cpp_std and Qt crates

Note: as described above, it's recommended to use docker for creating a suitable environment.

Qt crates can be generated like this:
```
cd ritual
cargo run --release --bin qt_ritual -- /path/to/workspace -c qt_core -o main
```

The workspace directory will be used for storing databases, temporary files, and the generated crates. Use the same workspace directory for all Qt crates to make sure that ritual can use types from previously generated crates.

Similarly, this is how `cpp_std` can be generated:
```
cargo run --release --bin std_ritual -- /path/to/workspace -c cpp_std -o main
```

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
