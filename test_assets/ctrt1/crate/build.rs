extern crate cpp_to_rust;
use cpp_to_rust::config::Config;

use std::path::PathBuf;

fn main() {
  let mut config = Config::new();
  config.add_include_directive("ctrt1/all.h");
  let mut path = PathBuf::from(
    std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR env var not found")
  );
  path = path.parent().expect("failed to get parent directory").to_path_buf();
  path.push("cpp");
  path.push("include");

  config.add_include_path(&path);
  config.add_target_include_path(&path);

  if let Err(err) = config.exec() {
    err.display_report();
    std::process::exit(1);
  }
}
