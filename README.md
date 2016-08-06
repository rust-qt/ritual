cpp_to_rust
===========

cpp_to_rust project is aimed to create Rust wrappers for C++ libraries automatically.

Dependencies
------------
- Stable Rust.
- libclang-dev.
- cmake.
- C++ compiler and make.
- Target library (only Qt5 is currently supported). 

Methodology
-----------
The converter parses C++ headers of the library and generates a wrapper library with C interface that provides access to all supported functions and methods of the library. Then it creates source of new Rust crate and compiles it.

How to run
----------
To start the process, you need a library specification folder. You can clone it from [this repository](https://github.com/rust-qt/qt_core_spec). It contains data that is exclusive for the target library. Then execute in this project's folder:

    cargo run path/to/spec.json path/to/output

First argument is the path to the spec.json file from the library specification. Output path will contain source and binary files for the C wrapper library and source files for the new crate. The converter will run "cargo test" on the crate to build and test it.

If you have multiple versions of Qt on your machine (for example, obtained via online installer) and you want to build against a specific version, you can edit "local_overrides.json" file in the output folder to specify path to qmake. Delete all other files in the output folder to remove cached data.

Platform support
----------------
Only Linux is currently supported. Windows support is a high priority. Mac OS support is not planned, but contributions are welcome.

Portability issues
------------------
The wrapper code is platform-dependent. The main problem is notion of sizes of structs, but they only matter if Rust-owned structs are used. Platform-dependency of types is generally mitigated by libc crate. However, the C++ library itself also may be platform-dependant. For example, it can use preprocessor directives to declare different types and methods for different platforms and environments. Different versions of the same library may also have different set of methods. Lack of methods will probably be reported by the linker, but type mismatch will not, and it can cause dangerous issues at runtime.

This project does not have a portability solution yet. It is theoretically possible to analyze header parsing result on all supported platforms and generate platform-independent and environment-independent code. The result would look like libc - any type or method always corresponds to the OS in use. However, it is hard to implement such analysis, and it is even harder to organize the process on all supported systems and all versions of all supported libraries. 

So the project currently assumes that the user will use the generated wrapper only on the same system and with the same library that were used during its generation. Some library paths are hardcoded into generated source. It is possible that hardcoded paths will be removed in the future and the wrapper will be usable when moved on machines with the same arch, OS, and library version but different directory layout. Beyond that, however, there are no plans to introduce portability at the moment. 

Environment required to generate wrappers is somewhat heavy (see "Dependencies" section), so it may be troublesome to set it up on the target machine. Cross-compilation may be one solution to this problem, so supporting cross-compilation may become a goal in the future. 

On the other side, requirement to perform header parsing on the target system will assure that the wrapper is correct, and any missing methods and mismatched types will be reported by the Rust compiler.

Library coverage
----------------
The converter currently works with QtCore library only.

The first priority is to support all of Qt5 libraries. However, most of its code is not Qt-dependent, so it is possible to support arbitrary libraries in the future. 

C++/Rust features coverage
--------------------------
Currently implemented features:

- All typedefs are replaced by original types.
- Primitive types are mapped to Rust's primitive types (like "bool") and types provided by libc crate (like "libc::c_int").
- Fixed size types (like "qint8") are mapped to Rust's fixed size types (like "i8").
- QFlags<Enum> types are converted to Rust's own similar implementation.
- C++ namespaces are mapped to Rust submodules.
- Library is also separated to submodules based on include files.
- Pointers, references and values are mapped to Rust's respective types.
- Classes are mapped to structs of the same size. Classes without size and template classes are not supported yet.
- Free functions are mapped to free functions.
- Class methods are mapped to structs' implementations.
- All names are converted to match Rust naming conventions.
- Method overloading is emulated with wrapping arguments in a tuple and creating a trait describing tuples acceptable by each method. Methods with default arguments are treated in the same way.

Not implemented yet but planned:

- Create wrappers for all encountered instantiations of class templates.
- Implement type information exchange between multiple wrapper crates to reuse type wrappers from dependancy libraries and perform inter-crate template instantiations.   
- Support function types. Currently any methods containing function types are not wrapped.
- Implement operator traits for structs based on C++ operator methods.
- Implement Debug and Display traits for structs if applicable methods exist on C++ side.
- Implement iterator traits for collections.
- Provide access to Qt specific features (like signals and slots).
- Provide a way to emulate deriving from C++ classes to call protected functions and reimplement virtual functions.
- Provide access to a class's public variables.
- Provide access to static_cast, dynamic_cast and qobject_cast.
- Provide access to class's methods inherited from its base classes.
- Provide conversion from enums to int and back (used in Qt API).
- Generate meaningful Rust documentation for the crate.

Not planned to support:

- Advanced template usage, like types with integer template arguments.
- Template methods and functions.
- Typedef translation.

Contributing
------------
If you are willing to contribute, please contact me via [email](mailto:ri@idzaaus.org).

