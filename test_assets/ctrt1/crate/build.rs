extern crate cpp_to_rust;
use cpp_to_rust::config::Config;

fn main() {
  let config = Config::new();
  if let Err(err) = config.exec() {
    err.display_report();
    std::process::exit(1);
  }
}