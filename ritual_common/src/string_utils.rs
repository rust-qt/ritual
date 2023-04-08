//! Various utilities for string operations.

use itertools::Itertools;

/// Iterator over words in a camel-case
/// or snake-case string.
pub struct WordIterator<'a> {
    string: &'a str,
    index: usize,
}

impl<'a> WordIterator<'a> {
    /// Create iterator over `string`.
    pub fn new(string: &str) -> WordIterator<'_> {
        WordIterator { string, index: 0 }
    }
}

fn char_at(str: &str, index: usize) -> char {
    if index >= str.len() {
        panic!("char_at: index out of bounds");
    }
    str[index..=index].chars().next().unwrap()
}

impl<'a> Iterator for WordIterator<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        while self.index < self.string.len() && &self.string[self.index..=self.index] == "_" {
            self.index += 1;
        }
        if self.index >= self.string.len() {
            return None;
        }
        let mut i = self.index + 1;
        let current_word_is_number = i < self.string.len() && char_at(self.string, i).is_ascii_digit();
        while i < self.string.len() {
            let current = char_at(self.string, i);
            if current == '_' || current.is_uppercase() {
                break;
            }
            if !current_word_is_number && current.is_ascii_digit() {
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
    fn to_class_case(&self) -> String;
    /// Convert to snake-case string ("word_word_word")
    fn to_snake_case(&self) -> String;
    /// Convert to upper-case string ("WORD_WORD_WORD")
    fn to_upper_case_words(&self) -> String;
}

fn iterator_to_class_case<S: AsRef<str>, T: Iterator<Item = S>>(it: T) -> String {
    it.map(|x| {
        if char_at(x.as_ref(), 0).is_ascii_digit() {
            x.as_ref().to_uppercase()
        } else {
            format!(
                "{}{}",
                x.as_ref()[0..1].to_uppercase(),
                x.as_ref()[1..].to_lowercase()
            )
        }
    })
    .join("")
}

pub fn ends_with_digit<S: AsRef<str>>(s: S) -> bool {
    let str = s.as_ref();
    if str.is_empty() {
        false
    } else {
        str[str.len() - 1..str.len()]
            .chars()
            .next()
            .unwrap().is_ascii_digit()
    }
}

fn iterator_to_snake_case<S: AsRef<str>, T: Iterator<Item = S>>(it: T) -> String {
    let mut parts = it.map(|x| x.as_ref().to_lowercase()).collect_vec();
    replace_all_sub_vecs(&mut parts, &["na", "n"]);
    replace_all_sub_vecs(&mut parts, &["open", "g", "l"]);
    replace_all_sub_vecs(&mut parts, &["i", "o"]);
    replace_all_sub_vecs(&mut parts, &["2", "d"]);
    replace_all_sub_vecs(&mut parts, &["3", "d"]);
    replace_all_sub_vecs(&mut parts, &["4", "d"]);
    let mut string = String::new();
    for (i, part) in parts.into_iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        let all_digits = part.chars().all(|c| c.is_ascii_digit());
        if i > 0 && (!all_digits || ends_with_digit(&string)) {
            string.push('_');
        }
        string.push_str(&part);
    }
    string
}

fn iterator_to_upper_case_words<S: AsRef<str>, T: Iterator<Item = S>>(it: T) -> String {
    it.map(|x| x.as_ref().to_uppercase()).join("_")
}

fn replace_all_sub_vecs(parts: &mut Vec<String>, needle: &[&str]) {
    let mut any_found = true;
    while any_found {
        any_found = false;
        if parts.len() + 1 >= needle.len() {
            // TODO: maybe rewrite this
            for i in 0..parts.len() + 1 - needle.len() {
                if parts[i..i + needle.len()] == needle[..] {
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

impl CaseOperations for str {
    fn to_class_case(&self) -> String {
        iterator_to_class_case(WordIterator::new(self))
    }
    fn to_snake_case(&self) -> String {
        iterator_to_snake_case(WordIterator::new(self))
    }
    fn to_upper_case_words(&self) -> String {
        iterator_to_upper_case_words(WordIterator::new(self))
    }
}

impl<'a> CaseOperations for [&'a str] {
    fn to_class_case(&self) -> String {
        iterator_to_class_case(self.iter())
    }
    fn to_snake_case(&self) -> String {
        iterator_to_snake_case(self.iter())
    }
    fn to_upper_case_words(&self) -> String {
        iterator_to_upper_case_words(self.iter())
    }
}

pub fn trim_slice<T, F>(slice: &[T], mut f: F) -> &[T]
where
    F: FnMut(&T) -> bool,
{
    let first_good_index = if let Some(index) = slice.iter().position(|item| !f(item)) {
        index
    } else {
        return &[];
    };
    let last_good_index = slice
        .iter()
        .rposition(|item| !f(item))
        .expect("slice must contain good items as checked above");
    &slice[first_good_index..=last_good_index]
}

#[test]
fn test_trim_slice1() {
    assert_eq!(
        trim_slice(&["", "asd", "dsa", "", ""], |x| x.is_empty()),
        &["asd", "dsa"]
    );
}
