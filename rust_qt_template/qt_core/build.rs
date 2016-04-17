use std::env;

fn main () {
  if let Ok(qtcw_install_dir) = env::var("QTCW_LIB_DIR") {
    println!("cargo:rustc-link-search={}", qtcw_install_dir);
  } else {
    panic!("QTCW_LIB_DIR env var is not present");
  }
}
