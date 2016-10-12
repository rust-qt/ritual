#![cfg_attr(feature="clippy", allow(redundant_closure))]

use std;
extern crate regex;

error_chain! {
  foreign_links {
    std::io::Error, IO;
    regex::Error, Regex;
  }

  errors {
    Unexpected(msg: String) {
      display("{}", msg)
    }

  }
}

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
}

pub fn unexpected<S: Into<String>>(text: S) -> Error {
  ErrorKind::Unexpected(text.into()).into()
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
