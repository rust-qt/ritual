#![cfg_attr(feature="clippy", allow(redundant_closure))]

use std;

error_chain! {
  foreign_links {
    std::io::Error, IO;
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
