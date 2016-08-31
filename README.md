# cpp_to_rust

cpp_to_rust project is aimed to create Rust wrappers for C++ libraries automatically.

## Dependencies

- Stable Rust >= 1.9.
- libclang-dev. Mainly developed with version 3.5, but higher versions should be compatible as well.
- cmake >= 3.0.
- C++ compiler and make.
- Target library (only Qt5 is currently supported).

## Methodology

The converter parses C++ headers of the library and generates a wrapper library with C interface that provides access to all supported functions and methods of the library. Then it creates source of new Rust crate and compiles it.

## How to run

### Running from command line

To start the process, you need the source folder containing data that is exclusive for the target library. The folder for QtCore library is available [here](https://github.com/rust-qt/qt_core_spec). Then execute in this project's folder:

    cargo run path/to/source path/to/output

First argument is the path to the source file. Output path will contain source files of the crate. C wrapper library will be placed in a subdirectory of the output directory. The converter will run "cargo test" and "cargo doc" on the crate to build and test it.

If you have multiple versions of Qt on your machine (for example, obtained via online installer) and you want to build against a specific version, you can edit "local_overrides.json" file in the output folder to specify path to qmake. Delete all other files in the output folder to remove cached data.

### Running from a build script

This option uses standard Cargo's build system. You only need to download the source folder and run "cargo build" in it. The converter will be built and executed automatically.

If you use custom Qt version that is not available in library path, you need to set LIBRARY_PATH and LD_LIBRARY_PATH environment variables before executing Cagro commands. When running the converter directly from command line, this is done automatically, but it's not possible to do it from a build script.

## Platform support

Only Linux is currently supported. Windows support is a high priority. Mac OS support is not planned, but contributions are welcome.

## Portability issues

The wrapper code is platform-dependent. The main problem is notion of sizes of structs, but they only matter if Rust-owned structs are used. Platform-dependency of types is generally mitigated by libc crate. However, the C++ library itself also may be platform-dependant. For example, it can use preprocessor directives to declare different types and methods for different platforms and environments. Different versions of the same library may also have different set of methods. Lack of methods will probably be reported by the linker, but type mismatch will not, and it can cause dangerous issues at runtime.

This project does not have a portability solution yet. It is theoretically possible to analyze header parsing result on all supported platforms and generate platform-independent and environment-independent code. The result would look like libc - any type or method always corresponds to the OS in use. However, it is hard to implement such analysis, and it is even harder to organize the process on all supported systems and all versions of all supported libraries.

So the project currently assumes that the user will use the generated wrapper only on the same system and with the same library that were used during its generation. It is possible that the generated crate will be usable when moved to machines with the same arch, OS, and library version but different directory layout. Beyond that, however, there are no plans to introduce portability at the moment.

Environment required to generate wrappers is somewhat heavy (see "Dependencies" section), so it may be troublesome to set it up on the target machine. Cross-compilation may be one solution to this problem, so supporting cross-compilation may become a goal in the future.

On the other side, requirement to perform header parsing on the target system will assure that the wrapper is correct, and any missing methods and mismatched types will be reported by the Rust compiler.

## Library coverage

The converter currently works with QtCore library only.

The first priority is to support all of Qt5 libraries. However, most of its code is not Qt-dependent, so it is possible to support arbitrary libraries in the future.

C++/Rust features coverage

Currently implemented features:

- All typedefs are replaced by original types.
- Primitive types are mapped to Rust's primitive types (like "bool") and types provided by libc crate (like "libc::c_int").
- Fixed size types (like "qint8") are mapped to Rust's fixed size types (like "i8").
- QFlags<Enum> types are converted to Rust's own similar implementation.
- C++ namespaces are mapped to Rust submodules.
- Library is also separated to submodules based on include files.
- Pointers, references and values are mapped to Rust's respective types.
- Function pointer types are mapped to Rust's equivalent representation. Function pointers with references or class values are not supported.
- Classes are mapped to structs of the same size. This also applies to all instantiations of template classes encountered in the library's API.
- Free functions are mapped to free functions.
- Class methods are mapped to structs' implementations.
- All names are converted to match Rust naming conventions.
- Method overloading is emulated with wrapping arguments in a tuple and creating a trait describing tuples acceptable by each method. Methods with default arguments are treated in the same way.
- Methods inherited from base classes are added directly to wrapper struct of the derived class.

Not implemented yet but planned:

- Generate meaningful Rust documentation for generated crate.
- Implement type information exchange between multiple wrapper crates to reuse type wrappers from dependancy libraries and perform inter-crate template instantiations.
- Implement operator traits for structs based on C++ operator methods.
- Implement Debug and Display traits for structs if applicable methods exist on C++ side.
- Implement iterator traits for collections.
- Provide access to Qt specific features (like signals and slots).
- Provide a way to emulate deriving from C++ classes to call protected functions and reimplement virtual functions.
- Provide access to a class's public variables.
- Provide access to static_cast, dynamic_cast and qobject_cast.
- Provide conversion from enums to int and back (used in Qt API).

Not planned to support:

- Advanced template usage, like types with integer template arguments.
- Template partial specializations.
- Template methods and functions.
- Types nested into template types, like Class1<T>::Class2.
- Typedef translation.

## Contributing

If you are willing to contribute, please contact me via [email](mailto:ri@idzaaus.org).

