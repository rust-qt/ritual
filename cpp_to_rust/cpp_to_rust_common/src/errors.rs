#![cfg_attr(feature="clippy", allow(redundant_closure))]

//! Error handling types based on `error_chain` crate.

use std;


error_chain! {
  foreign_links {
    std::io::Error, IO;
    ::regex::Error, Regex;
  }

  errors {
    Unexpected(msg: String) {
      display("{}", msg)
    }

  }
}

use backtrace::Symbol;

impl Error {
  /// Returns true if this error was not deemed possible
  /// during development.
  pub fn is_unexpected(&self) -> bool {
    if let ErrorKind::Unexpected(..) = *self.kind() {
      true
    } else {
      false
    }
  }

  /// Panics if the error is unexpected.
  /// Does nothing otherwise.
  pub fn discard_expected(&self) {
    if self.is_unexpected() {
      self.display_report();
      // TODO: don't panic on this in production
      panic!("unexpected error");
    }
  }

  /// Outputs formatted stack trace and
  /// chained error messages to the `Error` logging channel.
  pub fn display_report(&self) {
    use log;
    if let Some(backtrace) = self.backtrace() {
      log::error(format!("{:?}", backtrace));
      log::error("");
      let mut next_frame_num = 0;
      for frame in backtrace.frames() {
        for symbol in frame.symbols() {
          if let Some(path) = symbol.filename() {
            let path_is_good = |x: std::path::Component| if let Some(x) = x.as_os_str().to_str() {
              x == "libstd" || x == "libpanic_unwind" || x == "libcore" || x == "errors.rs" ||
              x.starts_with("backtrace") || x.starts_with("error-chain")
            } else {
              false
            };
            if path.components().any(path_is_good) {
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
            let line = if let Some(n) = symbol.lineno() {
              n.to_string()
            } else {
              "<no lineno>".to_string()
            };
            log::error(format!("      at {}:{}", path.display(), line));
            log::error("");
            next_frame_num += 1;
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

/// Constructs an unexpected error
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

/// Works like `unwrap()`, but in case of an error,
/// outputs formatted stack trace and
/// chained error messages to the `Error` logging channel.
pub fn fancy_unwrap<T>(value: Result<T>) -> T {
  if let Err(ref err) = value {
    err.display_report();
  }
  value.unwrap()
}
