#[test]
fn join() {
  use utils::JoinWithString;
  let a1 = vec!["a", "b", "c"];
  assert_eq!(a1.join(""), "abc");
  assert_eq!(a1.join("_"), "a_b_c");

  let a2: Vec<String> = vec![];
  assert_eq!(a2.join(""), "");
  assert_eq!(a2.join("_"), "");

  let a3 = ["Q", "W", "E"];
  assert_eq!(a3.join("x"), "QxWxE");

  let a4 = vec!["one", "two"];
  assert_eq!(a4.iter().map(|x| x.to_uppercase()).join("!"), "ONE!TWO");
}

#[test]
fn path_buf_with_added() {
  use utils::PathBufPushTweak;
  use std::path::PathBuf;
  let x = PathBuf::from("/tmp");
  let mut y = x.clone();
  y.push("name");
  assert_eq!(x.with_added("name"), y);
  assert_eq!(x.as_path().with_added("name"), y);
}

#[test]
fn word_iterator() {
  use utils::WordIterator;
  let string = "one_two_three".to_string();
  let mut a1 = WordIterator::new(&string);
  assert_eq!(a1.next(), Some("one"));
  assert_eq!(a1.next(), Some("two"));
  assert_eq!(a1.next(), Some("three"));
  assert_eq!(a1.next(), None);
}

#[test]
fn word_iterator2() {
  use utils::WordIterator;
  let string = "RustIsAwesome".to_string();
  let mut a1 = WordIterator::new(&string);
  assert_eq!(a1.next(), Some("Rust"));
  assert_eq!(a1.next(), Some("Is"));
  assert_eq!(a1.next(), Some("Awesome"));
  assert_eq!(a1.next(), None);
}

fn split_to_words(s: &'static str) -> Vec<&'static str> {
  use utils::WordIterator;
  WordIterator::new(s).collect()
}

#[test]
fn word_iterator3() {
  assert_eq!(split_to_words("one_two"), vec!["one", "two"]);
  assert_eq!(split_to_words("ONE_two"), vec!["ONE", "two"]);
  assert_eq!(split_to_words("OneXTwo"), vec!["One", "X", "Two"]);
  assert_eq!(split_to_words("QThreadPool"), vec!["Q", "Thread", "Pool"]);
}


#[test]
fn case_operations() {
  use utils::CaseOperations;

  let s1 = "first_second_last".to_string();
  assert_eq!(s1.to_class_case(), "FirstSecondLast");
  assert_eq!(s1.to_snake_case(), "first_second_last");

  let s2 = "FirstSecondLast".to_string();
  assert_eq!(s2.to_class_case(), "FirstSecondLast");
  assert_eq!(s2.to_snake_case(), "first_second_last");

  let s3 = "First_Second_last".to_string();
  assert_eq!(s3.to_class_case(), "FirstSecondLast");
  assert_eq!(s3.to_snake_case(), "first_second_last");

  let s4 = "isNaN".to_string();
  assert_eq!(s4.to_class_case(), "IsNaN");
  assert_eq!(s4.to_snake_case(), "is_nan");

  let s5 = "Base64Format".to_string();
  assert_eq!(s5.to_class_case(), "Base64Format");
  assert_eq!(s5.to_snake_case(), "base64_format");

  let s6 = "toBase64".to_string();
  assert_eq!(s6.to_class_case(), "ToBase64");
  assert_eq!(s6.to_snake_case(), "to_base64");

  let s7 = "too_many__underscores".to_string();
  assert_eq!(s7.to_class_case(), "TooManyUnderscores");
  assert_eq!(s7.to_snake_case(), "too_many_underscores");

  let s8 = "OpenGLFunctions".to_string();
  assert_eq!(s8.to_class_case(), "OpenGLFunctions");
  assert_eq!(s8.to_snake_case(), "opengl_functions");

}
