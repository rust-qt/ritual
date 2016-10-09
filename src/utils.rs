use std::path::PathBuf;
use std::collections::HashMap;
use std::hash::Hash;
use std;
use log;
use std::process::Command;
use std::env;
use errors::{ErrorKind, Result, ChainErr};

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
      let m = if a.is_empty() {
        a
      } else {
        a + separator.as_ref()
      };
      m + b.as_ref()
    })
  }
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
      self.index += 1;
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
      i += 1;
    }
    let result = &self.string[self.index..i];
    self.index = i;
    self.previous_word_case = Some(word_case);
    Some(result)
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


fn iterator_to_class_case<S: AsRef<str>, T: Iterator<Item = S>>(it: T) -> String {
  it.map(|x| {
      format!("{}{}",
              x.as_ref()[0..1].to_uppercase(),
              x.as_ref()[1..].to_lowercase())
    })
    .join("")
}

fn iterator_to_snake_case<S: AsRef<str>, T: Iterator<Item = S>>(it: T) -> String {
  it.map(|x| x.as_ref().to_lowercase()).join("_")
}

#[cfg_attr(feature="clippy", allow(needless_range_loop))]
fn replace_all_sub_vecs(parts: &mut Vec<String>, needle: Vec<&str>) {
  let mut any_found = true;
  while any_found {
    any_found = false;
    if parts.len() + 1 >= needle.len() {
      // TODO: maybe rewrite this
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


/// Runs a command, checks that it is successful, and
/// returns its output if requested
pub fn run_command(command: &mut Command, fetch_stdout: bool) -> String {
  log::info(format!("Executing command: {:?}", command));
  if fetch_stdout {
    match command.output() {
      Ok(output) => {
        match command.status() {
          Ok(status) => {
            if !status.success() {
              panic!("Command failed: {:?} (status: {})", command, status);
            }
          }
          Err(error) => {
            panic!("Execution failed: {}", error);
          }
        }
        String::from_utf8(output.stdout).unwrap()
      }
      Err(error) => {
        panic!("Execution failed: {}", error);
      }
    }
  } else {
    match command.status() {
      Ok(status) => {
        if !status.success() {
          panic!("Command failed: {:?} (status: {})", command, status);
        }
      }
      Err(error) => {
        panic!("Execution failed: {}", error);
      }
    }
    String::new()
  }
}

#[cfg_attr(feature="clippy", allow(or_fun_call))]
pub fn add_env_path_item(env_var_name: &'static str,
                         mut new_paths: Vec<PathBuf>)
                         -> std::ffi::OsString {
  for path in env::split_paths(&env::var(env_var_name).unwrap_or(String::new())) {
    if new_paths.iter().find(|&x| x == &path).is_none() {
      new_paths.push(path);
    }
  }
  env::join_paths(new_paths).unwrap()
}
