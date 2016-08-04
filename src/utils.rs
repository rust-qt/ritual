use std::path::{Path, PathBuf};
use std::io;
use std::fs;
use std::fs::File;
use std::io::Read;

use log;

pub trait JoinWithString {
  fn join(self, separator: &'static str) -> String;
}

impl<X> JoinWithString for X
  where X: Iterator<Item = String>
{
  fn join(self, separator: &'static str) -> String {
    self.fold("".to_string(), |a, b| {
      let m = if a.len() > 0 {
        a + separator
      } else {
        a
      };
      m + &b
    })
  }
}

pub trait PathBufPushTweak {
  fn with_added<P: AsRef<Path>>(&self, path: P) -> Self;
}

impl PathBufPushTweak for PathBuf {
  fn with_added<P: AsRef<Path>>(&self, path: P) -> Self {
    let mut p = self.clone();
    p.push(path);
    p
  }
}

pub fn move_files(src: &PathBuf, dst: &PathBuf) -> io::Result<()> {
  if src.as_path().is_dir() {
    if !dst.as_path().is_dir() {
      log::info(format!("New dir created: {}", dst.to_str().unwrap()));
      try!(fs::create_dir(dst));
    }

    for item in try!(fs::read_dir(dst)) {
      let item = try!(item);
      if !src.with_added(item.file_name()).as_path().exists() {
        let path = item.path();
        if path.as_path().is_dir() {
          log::info(format!("Old dir removed: {}", path.to_str().unwrap()));
          try!(fs::remove_dir_all(path));
        } else {
          log::info(format!("Old file removed: {}", path.to_str().unwrap()));
          try!(fs::remove_file(path));
        }
      }
    }

    for item in try!(fs::read_dir(src)) {
      let item = try!(item);
      try!(move_files(&item.path().to_path_buf(),
                      &dst.with_added(item.file_name())));
    }
    try!(fs::remove_dir_all(src));
  } else {
    try!(move_one_file(src, dst));
  }
  Ok(())
}

pub fn copy_recursively(src: &PathBuf, dst: &PathBuf) -> io::Result<()> {
  if src.as_path().is_dir() {
    // if !dst.as_path().is_dir() {
    try!(fs::create_dir(dst));
    // }
    for item in try!(fs::read_dir(src)) {
      let item = try!(item);
      try!(copy_recursively(&item.path().to_path_buf(),
                            &dst.with_added(item.file_name())));
    }
  } else {
    try!(fs::copy(src, dst));
  }
  Ok(())
}

pub fn move_one_file(old_path: &PathBuf, new_path: &PathBuf) -> io::Result<()> {
  let is_changed = if new_path.as_path().is_file() {
    let mut string1 = String::new();
    let mut string2 = String::new();
    try!(try!(File::open(old_path)).read_to_string(&mut string1));
    try!(try!(File::open(new_path)).read_to_string(&mut string2));
    string1 != string2
  } else {
    true
  };

  if is_changed {
    if new_path.as_path().exists() {
      try!(fs::remove_file(new_path));
    }
    try!(fs::rename(old_path, new_path));
    log::info(format!("File changed: {}", new_path.to_str().unwrap()));
  } else {
    try!(fs::remove_file(old_path));
    log::info(format!("File not changed: {}", new_path.to_str().unwrap()));
  }
  Ok(())
}

extern crate inflector;

pub trait CaseOperations {
  fn to_class_case(&self) -> Self;
  fn to_snake_case(&self) -> Self;
}
impl CaseOperations for String {
  fn to_class_case(&self) -> Self {
    let mut x = inflector::Inflector::to_camel_case(self);
    if x.len() > 0 {
      let c = x.remove(0);
      let cu: String = c.to_uppercase().collect();
      x = cu + &x;
    }
    x
  }
  fn to_snake_case(&self) -> Self {
    inflector::Inflector::to_snake_case(self)
  }
}
