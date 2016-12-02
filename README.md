# cpp_to_rust

[![Linux + OS X](https://travis-ci.org/rust-qt/cpp_to_rust.svg?branch=master)](https://travis-ci.org/rust-qt/cpp_to_rust)
[![Windows](https://ci.appveyor.com/api/projects/status/m4yo29j2f5wfu3w0/branch/master?svg=true)](https://ci.appveyor.com/project/Riateche/cpp-to-rust)

`cpp_to_rust` automatically creates Rust crates that provide API to C++ libraries.

`cpp_to_rust` supports many language features, but some important features are not implemented yet, so it's totally not production-ready (see [this section](#crust-features-coverage) for more details).

`cpp_to_rust` mainly targets Qt libraries and uses some Qt-specific workarounds, but it may be able to process other C++ libraries fairly well. Some of Qt-specific functionality is separated to [qt_build_tools](https://github.com/rust-qt/qt_build_tools).

## How it works

The generation process is executed by the build script. It includes the following steps:

1. If a library has any dependencies based on `cpp_to_rust`, information collected during their generation is loaded and used for further processing.
2. `clang` C++ parser is executed to extract information about the library's types and methods from its header files.
3. A C++ wrapper library with C-compatible interface is generated. The library exposes each found method using a wrapper function.
4. A Rust code for the crate is generated. Functions from the C++ wrapper library are made available in the crate using Rust's [FFI support](https://doc.rust-lang.org/book/ffi.html). Rust code also contains `enum`s and `struct`s for all found C++ enums, structs and classes (including instantiations of template classes).
5. Build script and `cargo` build everything together into a single crate.
6. C++ library documentation and `cpp_to_rust`'s processing data are used to generate a full-featured documentation for the crate ([example](https://rust-qt.github.io/rustdoc/qt/qt_core/index.html)).

## C++/Rust features coverage

Many things are directly translated from C++ to Rust:

- Primitive types are mapped to Rust's primitive types (like `bool`) and types provided by libc crate (like `libc::c_int`).
- Fixed-size numeric types (e.g `int8_t` or `qint8`) are mapped to Rust's fixed size types (e.g. `i8`).
- Pointers, references and values are mapped to Rust's respective types.
- C++ namespaces are mapped to Rust submodules.
- C++ classes and structs are mapped to Rust structs. This also applies to all instantiations of template classes encountered in the library's API, including template classes of dependencies.
- Free functions are mapped to free functions.
- Class methods are mapped to structs' implementations.
- Destructors are mapped to `Drop` and `CppDeletable` implementations.
- Function pointer types are mapped to Rust's equivalent representation. Function pointers with references or class values are not supported.
- `QFlags<Enum>` types are converted to Rust's own similar implementation.
- `static_cast` and `dynamic_cast` are available in Rust through corresponding traits.

Names of Rust identifiers are modified according to Rust's naming conventions.

When direct translation is not possible:

- Contents of each include file of the C++ library are placed into a separate submodule.
- Method overloading is emulated with wrapping arguments in a tuple and creating a trait describing tuples acceptable by each method. Methods with default arguments are treated in the same way.
- Methods inherited from base classes are added directly to the wrapper struct of the derived class.

Not implemented yet but planned:

- Translate C++ `typedef`s to Rust type aliases.
- Implement operator traits for structs based on C++ operator methods ([issue](https://github.com/rust-qt/cpp_to_rust/issues/27)).
- Implement Debug and Display traits for structs if applicable methods exist on C++ side.
- Implement iterator traits for collections.
- Signals and slots API ([issue](https://github.com/rust-qt/cpp_to_rust/issues/7)).
- Subclassing API ([issue](https://github.com/rust-qt/cpp_to_rust/issues/26)).
- Provide access to a class's public variables ([issue](https://github.com/rust-qt/cpp_to_rust/issues/18)).
- Provide conversion from enums to int and back (used in Qt API).
- Support C++ types nested into template types, like `Class1<T>::Class2`.

Not planned to support:

- Advanced template usage, like types with integer template arguments.
- Template partial specializations.
- Template methods and functions.

## Platform support

Linux, OS X and Windows are supported. `cpp_to_rust` is continuously tested on the following platforms and targets:

  - Ubuntu Trusty x64 (stable-x86_64-unknown-linux-gnu);
  - OS X 10.9.5 (stable-x86_64-apple-darwin);
  - Windows Server 2012 R2 Windows 7 x64 with MSVC 14 (stable-x86_64-pc-windows-msvc).

## Dependencies

- Stable Rust ≥ 1.12.
- `libclang-dev` ≥ 3.5 (CI uses 3.8 and 3.9).
- cmake ≥ 3.0.
- `make` and a C++ compiler compatible with the Rust toolchain in use. On OS X, the command line developer tools are required, but full Xcode installation is not required.
- The C++ library to wrap, built for the same toolchain as Rust.

## How to use

Crates generated by `cpp_to_rust` can be added to your project as dependencies, just as any other library crates. We maintain the following crates:

- [qt_core](https://github.com/rust-qt/qt_core)
- [qt_gui](https://github.com/rust-qt/qt_gui)
- [qt_widgets](https://github.com/rust-qt/qt_widgets)

(eventually all Qt libraries will be added).

If you want to run `cpp_to_rust` on another library of your choice, you need to [create a crate](http://doc.crates.io/guide.html), [set up a build script](http://doc.crates.io/build-script.html) and write that build script. An example build script is available in [test_assets/ctrt1/crate/build.rs](https://github.com/rust-qt/cpp_to_rust/blob/master/test_assets/ctrt1/crate/build.rs) file. You can also use the crates listed above as examples. You also need to write a specific include macro in `lib.rs` file of the crate: see [test_assets/ctrt1/crate/src/lib.rs](https://github.com/rust-qt/cpp_to_rust/blob/master/test_assets/ctrt1/crate/src/lib.rs) for example.

[Documentation](https://rust-qt.github.io/rustdoc/cpp_to_rust/cpp_to_rust)

`cpp_to_rust`'s own API is not stable yet and will certainly change a lot.

## Environment variables

`cpp_to_rust` reads the following environment variables:

- `CLANG_SYSTEM_INCLUDE_PATH` - path to clang's system headers, e.g. `/usr/lib/llvm-3.8/lib/clang/3.8.0/include`. May be necessary to set if clang can't find system headers. Adding this directory as include directory via build script has different effect and may not be enough.
- `CPP_TO_RUST_CACHE` - path to the cache directory. If set, `cpp_to_rust` will use this directory to cache various data and will skip some processing steps if they were cached. This directory can and should be the same for all crates that are processed together. Make sure to clean or change the cache directory if something changes (e.g. `cpp_to_rust`'s version, toolchain, C++ library version, etc.).
- `CPP_TO_RUST_QUIET` - if set, turns off warning and debug log levels.
- [Environment variables set by Cargo for build scripts](http://doc.crates.io/environment-variables.html#environment-variables-cargo-sets-for-build-scripts).

C++ build tools and the linker may also read other environment variables, including `LIB`, `PATH`, `LIBRARY_PATH`, `LD_LIBRARY_PATH`, `DYLD_FRAMEWORK_PATH`. `cpp_to_rust` may add new directories to these variables, but it can't manage environment of the final linker, so you may need to set them manually.

## Remarks

### Expressing library dependencies

`cpp_to_rust` takes advantage of Rust's crate system. If a C++ library depends on another C++ library, generated Rust crate will also depend on the dependency's crate and reuse its types.

### Documentation generation

Documentation is important! `cpp_to_rust` generates `rustdoc` comments with information about corresponding C++ types and methods. Overloaded methods have detailed documentation listing all available variants. Qt documentation is integrated in `rustdoc` comments.

### Allocating C++ objects on the stack and the heap

`cpp_to_rust` supports two allocation place modes for C++ objects. The user can select the mode by passing `AsBox` or `AsStruct` as additional argument to constructors and other functions that return objects by value in C++.
 
`AsBox` mode:
 
1. The C++ object is created in the C++ wrapper library using `new`. Constructors are called like `new MyClass(args)`, and functions that return objects by value are called like `new MyClass(function(args))`.
2. Pointer returned by `new` is passed through FFI to the Rust wrapper and to `CppBox::new`. `CppBox<T>` is returned to the caller.
3. When `CppBox` is dropped, it calls the deleter function, which calls `delete object` on C++ side.
4. The raw pointer can be moved out of `CppBox` and passed to another function that can take ownership of the object.
  
`AsStruct` mode:
  
1. An uninitialized Rust struct is created on the stack. The size of the struct is the same as the size of the C++ object.  
2. Pointer to the struct is passed to the C++ wrapper function that uses placement new. Constructors are called like `new(buf) MyClass(args)`, where `buf` is pointer to the struct created in Rust. The struct is filled with valid data.
3. The caller retains ownership of the struct and can move it to the heap using `Box` or `Vec`, pass it to another place and take references and pointers to it. 
4. When the struct is dropped, a C++ destructor is called using a C++ wrapper function. Memory of the struct itself is managed by Rust.
  
`AsBox` is a more general and safe way to store C++ objects in Rust, but it can produce unnecessary overhead when working with multiple small objects. 

`AsStruct` is more limited and dangerous. You can't use it if pointers to the object are stored somewhere because Rust can move the struct in memory and the pointer can become invalid. And sometimes it's not clear whether they are stored or not. It is also forbidden to pass such structs to functions that take ownership, as they would try to delete it and free the memory that is managed by Rust. However, `AsStruct` allows to avoid heap allocations and can be used for small simple structs and classes.
  
Selecting one of these modes in currently up to the caller, but some kind of smart defaults and limitations may be implemented in the future.
  





### Portability and API stability issues

Assuming that the C++ library has exactly the same API on all platforms, the generated C++ and Rust code is almost totally portable. The only issue is using sizes of classes because they depend on the platform.

However, in reality C++ libraries often have API differences on different platforms. Even if they are subtle, they can result in changing types and methods in Rust API and consequent build issues in applications that use the crate. Using another version of the C++ library will also cause immediate issues because a C++ wrapper uses every possible function of the library and will definitely fail to build if any functions are missing.

Until these issues are resolved, cross-platform use of generated crates is significantly limited. One possible approach is described [here](https://github.com/rust-qt/cpp_to_rust/issues/6#issuecomment-252108305). It would allow to generate code that works on any supported platform and doesn't even require to install `clang` parser.

### FFI types

C++ wrapper functions may only contain C-compatible types in their signatures, so references and class values are replaced with pointers, and wrapper functions perform necessary conversions to and from original C++ types. In Rust code the types are converted back to references and values.

### Executable size

If Rust crates and C++ wrapper libraries are all built statically, the linker only runs once for the final executable that uses the crates. It should be able to eliminate all unused wrapper functions and produce a reasonably small file that will only depend on original C++ libraries.

However, C++ wrapper libraries are currently built dynamically for MSVC because its linker fails to process so many functions at once. This results in larger executable size and additional DLL dependencies.

## Contributing

Contributions are always welcome! You can contribute in different ways:

- Suggest a C++ library to adapt;
- Submit a bug report, a feature request, or an improvement suggestion at the [issue tracker](https://github.com/rust-qt/cpp_to_rust/issues);
- Write a test for `cpp_to_rust` itself or any of wrapped libraries;
- Pick up an issue with [help wanted](https://github.com/rust-qt/cpp_to_rust/labels/help%20wanted) tag or any other issue you like.
