//! Various utilities for string operations.

/// Join items of a collection with separator.
pub trait JoinWithSeparator<S> {
  /// Result type of the operation
  type Output;
  /// Join items of `self` with `separator`.
  fn join(self, separator: S) -> Self::Output;
}

impl<S, S2, X> JoinWithSeparator<S2> for X
  where S: AsRef<str>,
        S2: AsRef<str>,
        X: Iterator<Item = S>
{
  type Output = String;
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

/// Iterator over words in a camel-case
/// or snake-case string.
pub struct WordIterator<'a> {
  string: &'a str,
  index: usize,
}

impl<'a> WordIterator<'a> {
  /// Create iterator over `string`.
  pub fn new(string: &str) -> WordIterator {
    WordIterator {
      string: string,
      index: 0,
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
    let mut i = self.index + 1;
    let current_word_is_number = i < self.string.len() && char_at(self.string, i).is_digit(10);
    while i < self.string.len() {
      let current = char_at(self.string, i);
      if current == '_' || current.is_uppercase() {
        break;
      }
      if !current_word_is_number && current.is_digit(10) {
        break;
      }
      i += 1;
    }
    let result = &self.string[self.index..i];
    self.index = i;
    Some(result)
  }
}

/// Convert to string with different cases
pub trait CaseOperations {
  /// Convert to class-case string ("WordWordWord")
  fn to_class_case(self) -> String;
  /// Convert to snake-case string ("word_word_word")
  fn to_snake_case(self) -> String;
  /// Convert to upper-case string ("WORD_WORD_WORD")
  fn to_upper_case_words(self) -> String;
}


fn iterator_to_class_case<S: AsRef<str>, T: Iterator<Item = S>>(it: T) -> String {
  it.map(|x| if char_at(x.as_ref(), 0).is_digit(10) {
           x.as_ref().to_uppercase()
         } else {
           format!("{}{}",
                   x.as_ref()[0..1].to_uppercase(),
                   x.as_ref()[1..].to_lowercase())
         })
    .join("")
}

fn ends_with_digit<S: AsRef<str>>(s: S) -> bool {
  let str = s.as_ref();
  if str.len() > 0 {
    str[str.len() - 1..str.len()]
      .chars()
      .next()
      .unwrap()
      .is_digit(10)
  } else {
    false
  }
}

fn iterator_to_snake_case<S: AsRef<str>, T: Iterator<Item = S>>(it: T) -> String {
  let mut parts: Vec<_> = it.map(|x| x.as_ref().to_lowercase()).collect();
  replace_all_sub_vecs(&mut parts, vec!["na", "n"]);
  replace_all_sub_vecs(&mut parts, vec!["open", "g", "l"]);
  replace_all_sub_vecs(&mut parts, vec!["i", "o"]);
  replace_all_sub_vecs(&mut parts, vec!["2", "d"]);
  replace_all_sub_vecs(&mut parts, vec!["3", "d"]);
  replace_all_sub_vecs(&mut parts, vec!["4", "d"]);
  let mut str = String::new();
  for (i, part) in parts.into_iter().enumerate() {
    if part.is_empty() {
      continue;
    }
    if i > 0 && !(part.chars().all(|c| c.is_digit(10)) && !ends_with_digit(&str)) {
      str.push('_');
    }
    str.push_str(&part);
  }
  str
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
    iterator_to_snake_case(WordIterator::new(self))
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
