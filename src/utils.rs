use std::path::{Path, PathBuf};
use std::io;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use std::hash::Hash;
use std;
use log;

#[cfg(all(windows, target_env = "msvc"))]
pub fn is_msvc() -> bool {
  true
}

#[cfg(not(all(windows, target_env = "msvc")))]
pub fn is_msvc() -> bool {
  false
}


pub trait JoinWithString<S> {
  fn join(self, separator: S) -> String;
}

impl<S, S2, X> JoinWithString<S2> for X
  where S: AsRef<str>,
        S2: AsRef<str>,
        X: Iterator<Item = S>
{
  fn join(self, separator: S2) -> String {
    self.fold("".to_string(), |a, b| {
      let m = if a.len() > 0 {
        a + separator.as_ref()
      } else {
        a
      };
      m + b.as_ref()
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


pub fn move_files(src: &PathBuf, dst: &PathBuf) -> io::Result<()> {
  if src.as_path().is_dir() {
    if !dst.as_path().is_dir() {
      log::noisy(format!("New dir created: {}", dst.to_str().unwrap()));
      try!(fs::create_dir(dst));
    }

    for item in try!(fs::read_dir(dst)) {
      let item = try!(item);
      if !src.with_added(item.file_name()).as_path().exists() {
        let path = item.path();
        if path.as_path().is_dir() {
          log::noisy(format!("Old dir removed: {}", path.to_str().unwrap()));
          try!(fs::remove_dir_all(path));
        } else {
          log::noisy(format!("Old file removed: {}", path.to_str().unwrap()));
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
    log::noisy(format!("File changed: {}", new_path.to_str().unwrap()));
  } else {
    try!(fs::remove_file(old_path));
    log::noisy(format!("File not changed: {}", new_path.to_str().unwrap()));
  }
  Ok(())
}

#[derive(PartialEq, Debug)]
enum WordCase {
  Upper,
  Lower,
  Capitalized,
}

pub struct WordIterator<'a> {
  string: &'a str,
  index: usize,
  previous_word_case: Option<WordCase>,
}

impl<'a> WordIterator<'a> {
  pub fn new(string: &str) -> WordIterator {
    WordIterator {
      string: string,
      index: 0,
      previous_word_case: None,
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
    let mut i = self.index;
    let mut word_case = WordCase::Lower;
    while i < self.string.len() {
      let current = &self.string[i..i + 1].chars().next().unwrap();
      if current == &'_' {
        break;
      }
      if i - self.index == 0 {
        // first letter
        if current.is_uppercase() {
          word_case = WordCase::Capitalized;
        } else {
          word_case = WordCase::Lower;
        }
      } else if i - self.index == 1 {
        if current.is_uppercase() {
          if word_case == WordCase::Capitalized {
            let next_not_upper = if i + 1 < self.string.len() {
              !self.string[i + 1..i + 2].chars().next().unwrap().is_uppercase()
            } else {
              true
            };
            if next_not_upper || self.previous_word_case == Some(WordCase::Capitalized) {
              break;
            } else {
              word_case = WordCase::Upper;
            }
          } else if word_case == WordCase::Lower {
            break;
          }
        }
      } else {
        match word_case {
          WordCase::Lower | WordCase::Capitalized => {
            if current.is_uppercase() {
              break;
            }
          }
          WordCase::Upper => {
            if !current.is_uppercase() {
              break;
            }
          }
        }
      }
      i = i + 1;
    }
    let result = &self.string[self.index..i];
    self.index = i;
    self.previous_word_case = Some(word_case);
    return Some(result);
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

fn replace_all_sub_vecs(parts: &mut Vec<String>, needle: Vec<&str>) {
  let mut any_found = true;
  while any_found {
    any_found = false;
    if parts.len() + 1 >= needle.len() {
      for i in 0..parts.len() + 1 - needle.len() {
        if &parts[i..i + needle.len()] == &needle[..] {
          for _ in 0..needle.len() - 1 {
            parts.remove(i + 1);
          }
          parts[i] = needle.join("");
          any_found = true;
          break;
        }
      }
    }
  }
}

impl CaseOperations for String {
  fn to_class_case(&self) -> Self {
    iterator_to_class_case(WordIterator::new(self))
  }
  fn to_snake_case(&self) -> Self {
    let mut parts: Vec<_> = WordIterator::new(self).map(|x| x.to_lowercase()).collect();
    replace_all_sub_vecs(&mut parts, vec!["na", "n"]);
    replace_all_sub_vecs(&mut parts, vec!["open", "g", "l"]);
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


pub fn add_to_multihash<K: Eq + Hash + Clone, T, V: Default + Extend<T>>(hash: &mut HashMap<K, V>,
                                                                         key: &K,
                                                                         value: T) {
  if !hash.contains_key(key) {
    hash.insert(key.clone(), Default::default());
  }
  hash.get_mut(key).unwrap().extend(std::iter::once(value));
}
