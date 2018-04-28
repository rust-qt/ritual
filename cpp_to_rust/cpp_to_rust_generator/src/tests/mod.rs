mod cpp_type;
mod cpp_method;
mod cpp_ffi_data;
mod cpp_operator;
mod cpp_parser;
mod full_run;

use std::path::{Path, PathBuf};
use common::file_utils::{canonicalize, create_dir_all, PathBufWithAdded};

#[derive(Debug)]
pub enum TempTestDir {
  System(::tempdir::TempDir),
  Custom(PathBuf),
}

impl TempTestDir {
  pub fn new(name: &str) -> TempTestDir {
    if let Ok(value) = ::std::env::var("CPP_TO_RUST_TEMP_TEST_DIR") {
      let path = canonicalize(PathBuf::from(value)).unwrap().with_added(name);
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
