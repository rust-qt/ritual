qt_generator
============

Generator of Rust-Qt crates.

## Usage

Clone the project, compile and run with `cargo`.

```
git clone https://github.com/rust-qt/cpp_to_rust.git
cd cpp_to_rust/qt_generator/qt_generator
cargo run --release -- --help
```

All options passed to `cargo` after `--` are passed to `qt_generator`. For example:

`cargo run --release -- -c /path/to/cache -o /path/to/output -l all`

Output directory will contain the generated crates. Cache directory is used for temporary files and inter-library generation.

## Dependencies

In addition to [cpp_to_rust_generator](https://github.com/rust-qt/cpp_to_rust/tree/master/cpp_to_rust/cpp_to_rust_generator) dependencies, `qt_generator` requires `libsqlite3-dev` for parsing documentation. If `libsqlite3` is not installed system-wide, setting `SQLITE3_LIB_DIR` environment variable may be required.

`qmake` of the target Qt installation must be available in `PATH` (for both the generator and the Qt crates).

