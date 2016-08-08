use std::env;
use std::path::PathBuf;

fn main() {
  let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
  let mut c_lib_path = PathBuf::from(manifest_dir);
  c_lib_path.push("c_lib");
  c_lib_path.push("install");
  c_lib_path.push("lib");
  println!("cargo:rustc-link-search={}", c_lib_path.to_str().unwrap());
}
