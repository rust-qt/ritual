extern crate ansi_term;
use self::ansi_term::Colour;
use std::borrow::Borrow;
use std;

pub fn error<T: Borrow<str>>(text: T) {
  println!("{}", Colour::Red.paint(text.borrow()));
}
pub fn warning<T: Borrow<str>>(text: T) {
  if std::env::var("CPP_TO_RUST_QUIET").is_err() {
    println!("{}", Colour::Purple.paint(text.borrow()));
  }
}
pub fn info<T: Borrow<str>>(text: T) {
  println!("{}", Colour::Green.paint(text.borrow()));
}
pub fn debug<T: Borrow<str>>(text: T) {
  if std::env::var("CPP_TO_RUST_QUIET").is_err() {
    println!("{}", text.borrow());
  }
}

#[allow(unused_variables)]
pub fn noisy<T: Borrow<str>>(text: T) {}


use log;
use utils::manifest_dir;
use errors::Error;
extern crate backtrace;
use self::backtrace::Symbol;

impl Error {
  pub fn display_report(&self) {
    if let Some(backtrace) = self.backtrace() {
      log::error(format!("{:?}", backtrace));
      log::error("");
      let dir = manifest_dir();
      let mut next_frame_num = 0;
      for frame in backtrace.frames() {
        for symbol in frame.symbols() {
          if let Some(path) = symbol.filename() {
            if let Ok(relative_path) = path.strip_prefix(&dir) {
              let path_items: Vec<_> = relative_path.iter().collect();
              if path_items.len() == 2 && path_items[0] == "src" && path_items[1] == "errors.rs" {
                continue;
              }
              let name = if let Some(name) = symbol.name() {
                name.to_string()
              } else {
                "<no name>".to_string()
              };
              if next_frame_num == 0 {
                log::error("Best of stack backtrace:");
              }
              log::error(format!("{:>w$}: {}", next_frame_num, name, w = 4));
              log::info(format!("      at {}:{}",
                                relative_path.display(),
                                if let Some(n) = symbol.lineno() {
                                  n.to_string()
                                } else {
                                  "<no lineno>".to_string()
                                }));
              log::info("");
              next_frame_num += 1;
            }
          }
        }
      }
      if next_frame_num > 0 {
        log::error("");
      }
    }
    log::error("Error:");
    let items: Vec<_> = self.iter().collect();
    for (i, err) in items.iter().rev().enumerate() {
      log::error(format!("{:>w$}: {}", i, err, w = 4));
    }
  }
}
