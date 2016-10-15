#![cfg_attr(feature="clippy", allow(redundant_closure))]

use std;
extern crate regex;
extern crate csv;

error_chain! {
  foreign_links {
    std::io::Error, IO;
    regex::Error, Regex;
    csv::Error, Csv;
  }

  errors {
    Unexpected(msg: String) {
      display("{}", msg)
    }

  }
}

extern crate backtrace;
use self::backtrace::Symbol;

impl Error {
  pub fn is_unexpected(&self) -> bool {
    if let ErrorKind::Unexpected(..) = *self.kind() {
      true
    } else {
      false
    }
  }

  pub fn discard_expected(&self) {
    if self.is_unexpected() {
      self.display_report();
      // TODO: don't panic on this in production
      panic!("unexpected error");
    }
  }

  pub fn display_report(&self) {
    use log;
    use utils::manifest_dir;
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

pub fn unexpected<S: Into<String>>(text: S) -> ErrorKind {
  ErrorKind::Unexpected(text.into())
}

impl<T> ChainErr<T> for Option<T> {
  fn chain_err<F, EK>(self, callback: F) -> Result<T>
    where F: FnOnce() -> EK,
          EK: Into<ErrorKind>
  {
    match self {
      Some(x) => Ok(x),
      None => Err(Error::from("None encountered")).chain_err(callback),
    }
  }
}
