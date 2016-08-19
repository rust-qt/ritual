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
      let m = if a.len() > 0 { a + separator } else { a };
      m + &b
    })
  }
}

pub trait PathBufPushTweak {
  fn with_added<P: AsRef<Path>>(&self, path: P) -> PathBuf;
}

impl PathBufPushTweak for PathBuf {
  fn with_added<P: AsRef<Path>>(&self, path: P) -> PathBuf {
    let mut p = self.clone();
    p.push(path);
    p
  }
}

impl PathBufPushTweak for Path {
  fn with_added<P: AsRef<Path>>(&self, path: P) -> PathBuf {
    let mut p = self.to_path_buf();
    p.push(path);
    p
  }
}


pub fn move_files(src: &PathBuf,
                  dst: &PathBuf,
                  no_delete_exception: Vec<String>)
                  -> io::Result<()> {
  if src.as_path().is_dir() {
    if !dst.as_path().is_dir() {
      log::info(format!("New dir created: {}", dst.to_str().unwrap()));
      try!(fs::create_dir(dst));
    }

    for item in try!(fs::read_dir(dst)) {
      let item = try!(item);
      if !src.with_added(item.file_name()).as_path().exists() {
        let path = item.path();
        if no_delete_exception.iter()
          .find(|&x| x == &item.file_name().into_string().unwrap())
          .is_some() {
          log::info(format!("Old item preserved (exceptional): {}",
                            path.to_str().unwrap()));
        } else {
          if path.as_path().is_dir() {
            log::info(format!("Old dir removed: {}", path.to_str().unwrap()));
            try!(fs::remove_dir_all(path));
          } else {
            log::info(format!("Old file removed: {}", path.to_str().unwrap()));
            try!(fs::remove_file(path));
          }
        }
      }
    }

    for item in try!(fs::read_dir(src)) {
      let item = try!(item);
      try!(move_files(&item.path().to_path_buf(),
                      &dst.with_added(item.file_name()),
                      Vec::new()));
    }
    try!(fs::remove_dir_all(src));
  } else {
    try!(move_one_file(src, dst));
  }
  Ok(())
}

pub fn copy_recursively(src: &PathBuf, dst: &PathBuf) -> io::Result<()> {
  if src.as_path().is_dir() {
    try!(fs::create_dir(dst));
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

pub struct WordIterator<'a> {
  string: &'a String,
  index: usize,
}

impl<'a> WordIterator<'a> {
  pub fn new(string: &String) -> WordIterator {
    WordIterator {
      string: string,
      index: 0,
    }
  }
}

impl<'a> Iterator for WordIterator<'a> {
  type Item = &'a str;
  fn next(&mut self) -> Option<&'a str> {
    while self.index < self.string.len() && &self.string[self.index..self.index + 1] == "_" {
      self.index = self.index + 1;
    }
    if self.index >= self.string.len() {
      return None;
    }
    let mut i = self.index + 1;

    loop {
      let ok = if i == self.string.len() {
        true
      } else {
        let current = &self.string[i..i + 1].chars().next().unwrap();
        current == &'_' || current.is_uppercase()
      };
      if ok {
        let result = &self.string[self.index..i];
        self.index = i;
        return Some(result);
      }
      i = i + 1;
    }
  }
}

pub trait CaseOperations {
  fn to_class_case(&self) -> String;
  fn to_snake_case(&self) -> String;
}
pub trait VecCaseOperations {
  fn to_class_case(self) -> String;
  fn to_snake_case(self) -> String;
}


fn iterator_to_class_case<'a, S: AsRef<str>, T: Iterator<Item = S>>(it: T) -> String {
  it.map(|x| {
      format!("{}{}",
              x.as_ref()[0..1].to_uppercase(),
              x.as_ref()[1..].to_lowercase())
    })
    .join("")
}

fn iterator_to_snake_case<'a, S: AsRef<str>, T: Iterator<Item = S>>(it: T) -> String {
  it.map(|x| x.as_ref().to_lowercase()).join("_")
}


impl CaseOperations for String {
  fn to_class_case(&self) -> Self {
    iterator_to_class_case(WordIterator::new(self))
  }
  fn to_snake_case(&self) -> Self {
    let mut parts: Vec<_> = WordIterator::new(self).map(|x| x.to_lowercase()).collect();
    let mut any_found = true;
    while any_found {
      any_found = false;
      for i in 0..parts.len() - 1 {
        if parts[i] == "na" && parts[i + 1] == "n" {
          parts.remove(i + 1);
          parts[i] = "nan".to_string();
          any_found = true;
          break;
        }
      }
    }
    parts.join("_")
  }
}

impl<'a> VecCaseOperations for Vec<&'a str> {
  fn to_class_case(self) -> String {
    iterator_to_class_case(self.into_iter())
  }
  fn to_snake_case(self) -> String {
    iterator_to_snake_case(self.into_iter())
  }
}
