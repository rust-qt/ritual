# cpp_to_rust

**cpp_to_rust** project is aimed to create Rust wrappers for C++ libraries automatically.

## Dependencies

- Stable Rust >= 1.9.
- `libclang-dev`. Mainly developed with version 3.5, but higher versions should be compatible as well.
- cmake >= 3.0.
- make and C++ compiler compatible with the Rust toolchain in use. On OS X, the command line developer tools are needed, but full Xcode installation is not needed.
- The C++ library to wrap, built for the same toolchain as Rust.

If you have multiple versions of Qt on your machine (for example, obtained via online installer) and you want to build against a specific version, modify `PATH` environment variable so that `qmake` command of the correct Qt version is available. `cpp_to_rust` uses `qmake` command to find library paths.

## Methodology

The converter parses C++ headers of the library and generates a wrapper library with C interface that provides access to all supported functions and methods of the library. Then it creates source of new Rust crate and compiles it.

## How to run

### Input files

The generator requires multiple files to run: `Cargo.toml` file template for the library, `spec.json` file with additional information, tests and extra source files. Exact specification of input files is not stabilized at the moment. Prepared input files are available for multiple libraries:

- [qt_core](https://github.com/rust-qt/qt_core)
- [qt_gui](https://github.com/rust-qt/qt_gui)
- [qt_widgets](https://github.com/rust-qt/qt_widgets)

### Running from command line

Clone `cpp_to_rust` and the input folder. Execute in `cpp_to_rust`'s folder:

    cargo run -- -s path/to/input -o path/to/output

Output path will contain source files of the crate. C wrapper library will be placed in a subdirectory of the output directory. The converter will run `cargo test` and `cargo doc` on the crate to build and test it.

To use generated library, include it in your project's `Cargo.toml` by specifying relative or absolute path to its directory:

    qt_core = { path = "../output/qt_core" }

When processing a library with dependencies on other C++ libraries (e.g. QtWidgets that depends on QtCore and QtGui), dependencies need to be processed first. Use `-d <OUT_DIR1> <OUT_DIR2>...` argument to add dependencies to `cpp_to_rust`. You may not need to include dependencies in your project's `Cargo.toml` file because each generated crate re-exports all its dependencies. But if you decide to do it, make sure that you use the same crate directory that was used in `-d`.

### Running from a build script

This option uses standard Cargo's build system. The input files folder itself is a full-featured crate with a build script that uses `cpp_to_rust` to generate the sources. You can either clone the input folder and run `cargo build` in it, or just include the input files folder directly in your project:

    qt_widgets = { git = "https://github.com/rust-qt/qt_widgets.git" }

If the library is published on crates.io, you can also include it using its version:

    qt_widgets = "0.0"

C++ dependencies are found and buily automatically when using this method.

Main downside of this method is that `cargo` compiles a dependency crate from scratch for each of your projects and for each build configuration (e.g. debug and release) in a separate folder, so it's not possible to cache anything. `cpp_to_rust` will be executed for each project and each build configuration, and it may significantly slow down the build process, especially for large libraries with multiple dependencies. When executing `cpp_to_rust` from command line, it runs exactly once, and generated crate will not run `cpp_to_rust` when included in a project.

If you use custom Qt version that is not available in library path, you need to set LIBRARY_PATH and LD_LIBRARY_PATH environment variables before executing Cagro commands. When running the converter directly from command line, this is done automatically, but it's not possible to do it from a build script.

## Platform support

`cpp_to_rust` is continuously tested on the following platforms:

- with Travis: [![Build Status](https://travis-ci.org/rust-qt/cpp_to_rust.svg?branch=master)](https://travis-ci.org/rust-qt/cpp_to_rust)

  - Ubuntu Trusty x64 (stable-x86_64-unknown-linux-gnu);
  - OS X 10.9.5 (stable-x86_64-apple-darwin);

- with Appveyor: [![Build status](https://ci.appveyor.com/api/projects/status/m4yo29j2f5wfu3w0/branch/master?svg=true)](https://ci.appveyor.com/project/Riateche/cpp-to-rust/branch/master)

  - Windows Server 2012 R2 Windows 7 x64 with MSVC 14 (stable-x86_64-pc-windows-msvc).

It's also occasionally tested on other systems (Windows 7 x64, Debian Jessie x64).

## Library coverage

`cpp_to_rust` was tested on a limited set of libraries. See [Input files](#input-files) section for a list of currently supported libraries.

The first priority is to support all of Qt5 libraries. However, most of its code is not Qt-dependent, so it is possible to support arbitrary libraries in the future.

## C++/Rust features coverage

Currently implemented features:

- All typedefs are replaced by original types.
- Primitive types are mapped to Rust's primitive types (like "bool") and types provided by libc crate (like "libc::c_int").
- Fixed size types (like "qint8") are mapped to Rust's fixed size types (like "i8").
- QFlags<Enum> types are converted to Rust's own similar implementation.
- C++ namespaces are mapped to Rust submodules.
- Library is also separated to submodules based on include files.
- Pointers, references and values are mapped to Rust's respective types.
- Function pointer types are mapped to Rust's equivalent representation. Function pointers with references or class values are not supported.
- Classes are mapped to structs of the same size. This also applies to all instantiations of template classes encountered in the library's API, including template classes of dependencies.
- Free functions are mapped to free functions.
- Class methods are mapped to structs' implementations.
- All names are converted to match Rust naming conventions.
- Method overloading is emulated with wrapping arguments in a tuple and creating a trait describing tuples acceptable by each method. Methods with default arguments are treated in the same way.
- Methods inherited from base classes are added directly to wrapper struct of the derived class.
- If a type is wrapped in a dependency, it will be reused, not duplicated.
- Rustdoc comments are generated (a work in progress). Qt documentation is parsed and used in rustdoc comments.

Not implemented yet but planned:

- Implement operator traits for structs based on C++ operator methods.
- Implement Debug and Display traits for structs if applicable methods exist on C++ side.
- Implement iterator traits for collections.
- Provide access to Qt specific features (like signals and slots).
- Provide a way to emulate deriving from C++ classes to call protected functions and reimplement virtual functions.
- Provide access to a class's public variables.
- Provide access to `static_cast`, `dynamic_cast` and `qobject_cast`.
- Provide conversion from enums to int and back (used in Qt API).

Not planned to support:

- Advanced template usage, like types with integer template arguments.
- Template partial specializations.
- Template methods and functions.
- Types nested into template types, like `Class1<T>::Class2`.
- Typedef translation.

## Portability issues

The wrapper code is platform-dependent. The main problem is notion of sizes of structs, but they only matter if Rust-owned structs are used. Platform-dependency of types is generally mitigated by libc crate. However, the C++ library itself also may be platform-dependant. For example, it can use preprocessor directives to declare different types and methods for different platforms and environments. Different versions of the same library may also have different set of methods. Lack of methods will probably be reported by the linker, but type mismatch will not, and it can cause dangerous issues at runtime.

This project does not have a portability solution yet. It is theoretically possible to analyze header parsing result on all supported platforms and generate platform-independent and environment-independent code. The result would look like libc - any type or method always corresponds to the OS in use. However, it is hard to implement such analysis, and it is even harder to organize the process on all supported systems and all versions of all supported libraries.

So the project currently assumes that the user will use the generated wrapper only on the same system and with the same library that were used during its generation. It is possible that the generated crate will be usable when moved to machines with the same arch, OS, and library version but different directory layout. Beyond that, however, there are no plans to introduce portability at the moment.

Environment required to generate wrappers is somewhat heavy (see "Dependencies" section), so it may be troublesome to set it up on the target machine. Cross-compilation may be one solution to this problem, so supporting cross-compilation may become a goal in the future.

On the other side, requirement to perform header parsing on the target system will assure that the wrapper is correct, and any missing methods and mismatched types will be reported by the Rust compiler.

## Contributing

Contributions are always welcome! There are easy ways to contribute:

- Suggest a C++ library to adapt;
- Submit a bug report, a feature request, or an improvement suggestion at the [issue tracker](https://github.com/rust-qt/cpp_to_rust/issues);
- Write a test for `cpp_to_rust` itself or any of wrapped libraries;
- Pick up an issue with [help wanted](https://github.com/rust-qt/cpp_to_rust/labels/help%20wanted) tag or any other issue you like.





