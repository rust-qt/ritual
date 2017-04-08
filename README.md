# cpp_to_rust

[![Linux + OS X](https://travis-ci.org/rust-qt/cpp_to_rust.svg?branch=master)](https://travis-ci.org/rust-qt/cpp_to_rust)
[![Windows](https://ci.appveyor.com/api/projects/status/m4yo29j2f5wfu3w0/branch/master?svg=true)](https://ci.appveyor.com/project/Riateche/cpp-to-rust)

`cpp_to_rust` allows to use C++ libraries from Rust. The main target of this project is Qt.

## Repository structure

The project consists of the following Rust crates:

- [cpp_to_rust/cpp_to_rust_generator](cpp_to_rust/cpp_to_rust_generator) implements the generator;
- [cpp_to_rust/cpp_to_rust_build_tools](cpp_to_rust/cpp_to_rust_build_tools) implements the build script for generated crates;
- [cpp_to_rust/cpp_to_rust_common](cpp_to_rust/cpp_to_rust_common) contains common code for the previous two crates;
- [cpp_to_rust/cpp_utils](cpp_to_rust/cpp_utils) provides essential utilities used by generated crates;
- [qt_generator/qt_generator](qt_generator/qt_generator) contains generator configuration for the Qt crates and a binary file for running the generator;
- [qt_generator/qt_build_tools](qt_generator/qt_build_tools) implements the advanced build script for Qt crates;
- [qt_generator/qt_generator_common](qt_generator/qt_generator_common) contains common code for the previous two crates.

See `README.md` in each crate for detailed description.

## Generator workflow

The generator itself (`cpp_to_rust_generator`) is a library which exposes API for configurating different aspects of the process. In order to run the generator and produce an output crate, one must use a binary crate (such as `qt_generator`) and launch the generator using its API.

The generator runs in the following steps:

1. If the target library has any dependencies which were already processed and converted to Rust crates, information collected during their generation is loaded from the cache directory and used for further processing.
2. `clang` C++ parser is executed to extract information about the library's types and methods from its header files.
3. A C++ wrapper library with C-compatible interface is generated. The library exposes each found method using a wrapper function.
4. A Rust code for the crate is generated. Functions from the C++ wrapper library are made available in the crate using Rust's [FFI support](https://doc.rust-lang.org/book/ffi.html). Rust code also contains `enum`s and `struct`s for all found C++ enums, structs and classes (including instantiations of template classes).
5. C++ library documentation (if available) and `cpp_to_rust`'s processing data are used to generate a full-featured documentation for the crate ([example](https://rust-qt.github.io/rustdoc/qt/qt_core/index.html)).
6. The Rust code is saved to the output directory along with any extra files (tests, examples, etc.) provided by the caller. A build script necessary for building the crate is also attached.
7. Internal information of the generator is written to the cache directory and can be used when processing the library's dependants.

The generated crate can be built using `cargo` and included to an other project as a dependency, just as any other crate.

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
- `static_cast` and `dynamic_cast` are available in Rust through corresponding traits.

Names of Rust identifiers are modified according to Rust's naming conventions.

When direct translation is not possible:

- Contents of each include file of the C++ library are placed into a separate submodule.
- Method overloading is emulated with wrapping arguments in a tuple and creating a trait describing tuples acceptable by each method. Methods with default arguments are treated in the same way.
- Single inheritance is translated to `Deref` and `DerefMut` implementation, allowing to call base class methods on derived objects. When deref coercions are not enough, `static_cast` should be used to convert from derived to base class.
- Getter and setter methods are created for each public class field.

Not implemented yet but planned:

- Translate C++ `typedef`s to Rust type aliases.
- Implement operator traits for structs based on C++ operator methods ([issue](https://github.com/rust-qt/cpp_to_rust/issues/27)). Operators are currently exposed as regular functions with `op_` prefix.
- Implement Debug and Display traits for structs if applicable methods exist on C++ side.
- Implement iterator traits for collections.
- Subclassing API ([issue](https://github.com/rust-qt/cpp_to_rust/issues/26)).
- Provide access to a class's public variables ([issue](https://github.com/rust-qt/cpp_to_rust/issues/18)).
- Provide conversion from enums to int and back (used in Qt API).
- Support C++ types nested into template types, like `Class1<T>::Class2`.

Not planned to support:

- Advanced template usage, like types with integer template arguments.
- Template partial specializations.
- Template methods and functions.

## Qt-specific features coverage

Implemented: 

- `QFlags<Enum>` types are converted to Rust's own similar implementation located at `qt_core::flags`).
- `qt_core::connections` implements a way to use signals and slots. It's possible to use signals and slots of the built-in Qt classes and create slots bound to an arbitrary closure from Rust code. Argument types compability is checked at compile time.

Not implemented yet but planned:

- Creating custom signals from Rust code.

## Platform support

Linux, OS X and Windows are supported. `cpp_to_rust` and Qt crates are continuously tested using Travis and Appveyor on the following platforms and targets:

  - Ubuntu Trusty x64 (stable-x86_64-unknown-linux-gnu);
  - OS X 10.9.5 (stable-x86_64-apple-darwin);
  - Windows Server 2012 R2 Windows 7 x64 with MSVC 14 (stable-x86_64-pc-windows-msvc).

Windows MinGW toolchain is partially supported.
  
## Dependencies

All crates require stable Rust ≥ 1.12 and some dependencies delivered by `cargo` automatically.  

The generator additionally requires:

- `libclang-dev` ≥ 3.5 (CI uses 3.8 and 3.9).
- The target C++ library (include and library files), compatible with the Rust toolchain in use.
- cmake ≥ 3.0.
- A C++ building toolchain, compatible with the Rust toolchain in use:
  - On Linux: `make` and a C++ compiler;
  - On Windows: MSVC or MinGW environment;
  - On OS X: the command line developer tools (full Xcode installation is not required).

The Qt generator also requires `libsqlite3-dev` (used for parsing documentation).

The generated crate requires:

- The target C++ library;
- `cmake`;
- A C++ building toolchain.

## How to use

Generated Qt crates will be published in cargo registry. See [qt_core](https://crates.io/crates/qt_core),
[qt_gui](https://crates.io/crates/qt_gui), [qt_widgets](https://crates.io/crates/qt_widgets), [qt_ui_tools](https://crates.io/crates/qt_ui_tools) (more crates will be adde eventually).

However, published crates only support a certain Qt version (currently 5.8) and don't export platform-specific API. If you want to use another version or tweak the generator configuration, clone the repository and run `qt_generator`. See [qt_generator](qt_generator/qt_generator) for more information.

If you want to generate a crate for another C++ library, create a new binary crate and call the generator using its API.
See [full_run.rs](cpp_to_rust/cpp_to_rust_generator/src/tests/full_run.rs) and [qt_generator](qt_generator/qt_generator) for examples. See also [API documentation](https://rust-qt.github.io/rustdoc/cpp_to_rust/cpp_to_rust).

## Environment variables

`cpp_to_rust` reads the following environment variables:

- `CLANG_SYSTEM_INCLUDE_PATH` - path to clang's system headers, e.g. `/usr/lib/llvm-3.8/lib/clang/3.8.0/include`. May be necessary to set if clang can't find system headers. Adding this directory as include directory via build script has different effect and may not be enough.

C++ build tools and the linker may also read other environment variables, including `LIB`, `PATH`, `LIBRARY_PATH`, `LD_LIBRARY_PATH`, `DYLD_FRAMEWORK_PATH`. The generator has API for specifying library paths, passes them to `cmake` when building the C++ wrapper library, and reports the paths in build script's output, but it may not be enough for the linker to find the library, so you may need to set them manually.

## Remarks

### Expressing library dependencies

`cpp_to_rust` takes advantage of Rust's crate system. If a C++ library depends on another C++ library, generated Rust crate will also depend on the dependency's crate and reuse its types.

### Documentation generation

Documentation is important! `cpp_to_rust` generates `rustdoc` comments with information about corresponding C++ types and methods. Overloaded methods have detailed documentation listing all available variants. Qt documentation is integrated in `rustdoc` comments.

### Allocating C++ objects on the stack and the heap

`cpp_to_rust` supports two allocation place modes for C++ objects. Appropriate mode is chosen automatically and separately for each type and can be overriden in generator configuration. Allocation place mode only affects methods which return class values (not references, pointers or primitive types) and constructors.

Box mode: 

1. The returned object is placed on the heap on C++ side using `new`. Constructors are called like `new MyClass(args)`, and functions that return objects by value are called like `new MyClass(function(args))`.
2. Pointer returned by `new` is passed through FFI to the Rust wrapper and to `cpp_utils::CppBox::new`. `CppBox<T>` is returned to the caller.
3. When `CppBox` is dropped, it calls the deleter function, which calls `delete object` on C++ side.
4. The raw pointer can be moved out of `CppBox` and passed to another function that can take ownership of the object.

Struct mode:
  
1. An uninitialized Rust struct is created on the stack. The size of the struct is the same as the size of the C++ object. (Object size on the current platform is determined by the build script).  
2. Pointer to the struct is passed to the C++ wrapper function that uses placement `new`. Constructors are called like `new(buf) MyClass(args)`, where `buf` is pointer to the struct created in Rust. The struct is filled with valid data.
3. The caller retains ownership of the struct and can move it to the heap using `Box` or `Vec`, pass it to another place and take references and pointers to it. 
4. When the struct is dropped, a C++ destructor is called using a C++ wrapper function. Memory of the struct itself is managed by Rust.
  
Box mode is a more general and safe way to store C++ objects in Rust, but it can produce unnecessary overhead when working with multiple small objects. 

Struct mode is more limited and dangerous. You can't use it if pointers to the object are stored somewhere because Rust can move the struct in memory and the pointer can become invalid. And sometimes it's not clear whether they are stored or not. It is also forbidden to pass such structs to functions that take ownership, as they would try to delete it and free the memory that is managed by Rust. However, struct mode allows to avoid heap allocations and can be used for small simple structs and classes.
  
The generator automatically chooses Box mode for types which are passed to other functions as pointers because these functions may take ownership of the object. Also Box mode is default for types with any virtual methods. All other types use Struct mode by default.

Pointers, references and primitive types are (in most cases) passed through FFI border as-is, so allocation place mode has no effect on them. If a class value is used as an argument, it's converted to a const reference on Rust side to allow passing the value regardless of ownership and placement in memory.

### Cross-platform portability

The generator currently assumes that the C++ library's API is consistent across all supported platforms. Any platform-specific (e.g. Windows only) classes and methods should be blacklisted in the generator's configuration. As long as it's done, the generated crate should work successfully on all platforms supported by the generator. The build script of the generated crate will build the C++ wrapper library using currently available toolchain and determine actual struct sizes when necessary.

When working with platform-specific API, it's possible to run the generator on each target platform and use its results within that platform.

If using previously generated crate with a different version of the C++ library, the version needs to be source compatible with the version used for generation (but it's not required to be binary compatible). In case of Qt, that means that older and newer patch releases and newer minor releases should be compatible with an older crate.

### API stability

The generator is in active development, and its own API is not stable yet. That shouldn't be a big issue, because it's just a development tool, and amount of code using the generator's API should be fairly small.

The big issue is that the generator can't provide API stability of the generated crate. It's possible (and currently highly likely) that the generated API will significantly change when switching to a newer version of the generator.

The generator may also introduce breaking changes when switching to another version of the C++ library, even if these versions are compatible on C++ side. For example:

- Introducing new enum variants may change names of previously existing ones in Rust.
- Introducing new method overloads may change names of corresponding methods in Rust.

Fixing these issues would require two steps:

1. Stabilize behavior of the generator. It's currently unclear what's the best way to generate API, but eventually the changes should come to a minimum.
2. Implement ability to freeze API of a crate and force generator to make backward-compatible API when processing a newer version.

### Safety

It's impossible to bring Rust's safety to C++ APIs automatically, so most of the generated APIs are very unsafe to use and require thinking in C++ terms. In Rust, it's totally safe to produce a raw pointer, but unsafe to dereference it. The generator marks all wrapper functions accepting raw pointers as unsafe because raw pointers are not guaranteed to be valid, and the function will almost definitely try to dereference the pointer (it may check for null if we're lucky).

It's possible to introduce a generator API for marking methods as safe and/or changing their signatures (e.g. converting raw pointers to references, `Option`s or anything more suitable), but it hasn't been done yet.

### FFI types

C++ wrapper functions may only contain C-compatible types in their signatures, so references and class values are replaced with pointers, and wrapper functions perform necessary conversions to and from original C++ types. In Rust code the types are converted back to references and values.

### Executable size

If Rust crates and C++ wrapper libraries are all built statically, the linker only runs once for the final executable that uses the crates. It should be able to eliminate all unused wrapper functions and produce a reasonably small file that will only depend on original C++ libraries.

## Contributing

Contributions are always welcome! You can contribute in different ways:

- Suggest a C++ library to adapt;
- Submit a bug report, a feature request, or an improvement suggestion at the [issue tracker](https://github.com/rust-qt/cpp_to_rust/issues);
- Write a test for `cpp_to_rust` itself or any of wrapped libraries;
- Pick up an issue with [help wanted](https://github.com/rust-qt/cpp_to_rust/labels/help%20wanted) tag or any other issue you like.
