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

fn char_at(str: &str, index: usize) -> char {
  if index >= str.len() {
    panic!("char_at: index out of bounds");
  }
  str[index..index + 1].chars().next().unwrap()
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
      let current = char_at(self.string, i);
      if current == '_' {
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
              !char_at(self.string, i + 1).is_uppercase()
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
  fn to_class_case(self) -> String;
  fn to_snake_case(self) -> String;
  fn to_upper_case_words(self) -> String;
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

fn iterator_to_upper_case_words<S: AsRef<str>, T: Iterator<Item = S>>(it: T) -> String {
  it.map(|x| x.as_ref().to_uppercase()).join("_")
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

impl<'a> CaseOperations for &'a str {
  fn to_class_case(self) -> String {
    iterator_to_class_case(WordIterator::new(self))
  }
  fn to_snake_case(self) -> String {
    let mut parts: Vec<_> = WordIterator::new(self).map(|x| x.to_lowercase()).collect();
    replace_all_sub_vecs(&mut parts, vec!["na", "n"]);
    replace_all_sub_vecs(&mut parts, vec!["open", "g", "l"]);
    parts.join("_")
  }
  fn to_upper_case_words(self) -> String {
    iterator_to_upper_case_words(WordIterator::new(self))
  }
}

impl<'a> CaseOperations for Vec<&'a str> {
  fn to_class_case(self) -> String {
    iterator_to_class_case(self.into_iter())
  }
  fn to_snake_case(self) -> String {
    iterator_to_snake_case(self.into_iter())
  }
  fn to_upper_case_words(self) -> String {
    iterator_to_upper_case_words(self.into_iter())
  }
}
