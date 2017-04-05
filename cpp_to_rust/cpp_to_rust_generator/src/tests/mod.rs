mod cpp_type;
mod cpp_method;
mod cpp_ffi_data;
mod cpp_operator;
mod cpp_parser;
mod full_run;

use std::path::{Path, PathBuf};
use common::file_utils::{create_dir_all, PathBufWithAdded};

pub enum TempTestDir {
  System(::tempdir::TempDir),
  Custom(PathBuf),
}

impl TempTestDir {
  pub fn new(name: &str) -> TempTestDir {
    if let Ok(value) = ::std::env::var("CPP_TO_RUST_TEMP_TEST_DIR") {
      let path = PathBuf::from(value).with_added(name);
      create_dir_all(&path).unwrap();
      TempTestDir::Custom(path)
    } else {
      TempTestDir::System(::tempdir::TempDir::new(name).unwrap())
    }
  }

  pub fn path(&self) -> &Path {
    match *self {
      TempTestDir::System(ref dir) => dir.path(),
      TempTestDir::Custom(ref path) => path,
    }
  }
}

// Testing plan:
//
// - qt_specific
// - main (refactor)
// - cpp_data
// - cpp_ffi_generator
// - cpp_code_generator
//
// - utils (file operations)
//
// - rust_type
// - rust_info
// - rust_generator
// - rust_code_generator
//
//
