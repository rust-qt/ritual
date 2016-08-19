
// make sure to add test calls to tests() func
#![forbid(dead_code)]

extern crate tempdir;

use cpp_parser;
use cpp_data::*;
use utils::PathBufPushTweak;
use std::fs;
use std::fs::File;
use std::io::Write;
use cpp_method::*;
use cpp_type::*;


fn run_parser(code: &'static str) -> CppData {
  assert!(code.ends_with("\n"));
  let dir = tempdir::TempDir::new("test_cpp_parser_run").unwrap();
  let include_dir = dir.path().with_added("include");
  fs::create_dir(&include_dir).unwrap();
  let include_name = "myfakelib.h";
  let include_file_path = include_dir.with_added(&include_name);
  {
    let mut include_file = File::create(&include_file_path).unwrap();
    include_file.write(code.as_bytes()).unwrap();
  }
  let mut result = cpp_parser::run(cpp_parser::CppParserConfig {
    include_dirs: vec![include_dir],
    header_name: include_name.to_string(),
    tmp_cpp_path: dir.path().with_added("1.cpp"),
    name_blacklist: Vec::new(),
  });
  for method in &mut result.methods {
    if let Some(ref mut origin_location) = method.origin_location {
      assert_eq!(origin_location.include_file_path,
                 include_file_path.display().to_string());
      assert!(origin_location.line > 0);
    } else {
      panic!("no origin_location in pare result");
    }
    method.origin_location = None;
  }
  result
}

fn simple_func() {
  let data = run_parser("int func1(int x);\n");
  assert!(data.template_instantiations.is_empty());
  assert!(data.types.is_empty());
  assert!(data.methods.len() == 1);
  assert_eq!(data.methods[0],
             CppMethod {
               name: "func1".to_string(),
               class_membership: None,
               operator: None,
               return_type: Some(CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
               }),
               arguments: vec![CppFunctionArgument {
                                 name: "x".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::None,
                                   is_const: false,
                                   base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
                                 },
                                 has_default_value: false,
                               }],
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
             });
}

fn simple_func_with_default_value() {
  let data = run_parser("bool func1(int x = 42);\n");
  assert!(data.template_instantiations.is_empty());
  assert!(data.types.is_empty());
  assert!(data.methods.len() == 1);
  assert_eq!(data.methods[0],
             CppMethod {
               name: "func1".to_string(),
               class_membership: None,
               operator: None,
               return_type: Some(CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
               }),
               arguments: vec![CppFunctionArgument {
                                 name: "x".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::None,
                                   is_const: false,
                                   base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
                                 },
                                 has_default_value: true,
                               }],
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
             });
}

fn functions_with_class_arg() {
  let data = run_parser("class Magic { public: int a, b; };
  bool func1(Magic x);
  bool func1(Magic* x);
  bool func2(const Magic&);
  \n");
  assert!(data.template_instantiations.is_empty());
  assert_eq!(data.types.len(), 1);
  assert_eq!(data.types[0].name, "Magic");
  match data.types[0].kind {
    CppTypeKind::Class { ref size, ref bases, ref fields, ref template_arguments } => {
      assert!(size.is_some());
      assert!(template_arguments.is_none());
      assert!(bases.is_empty());
      assert_eq!(fields,
                 &vec![CppClassField {
                         name: "a".to_string(),
                         field_type: CppType {
                           indirection: CppTypeIndirection::None,
                           is_const: false,
                           base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
                         },
                         visibility: CppVisibility::Public,
                       },
                       CppClassField {
                         name: "b".to_string(),
                         field_type: CppType {
                           indirection: CppTypeIndirection::None,
                           is_const: false,
                           base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
                         },
                         visibility: CppVisibility::Public,
                       }]);


    }
    _ => panic!("invalid type kind"),
  }
  assert!(data.methods.len() == 3);
  assert_eq!(data.methods[0],
             CppMethod {
               name: "func1".to_string(),
               class_membership: None,
               operator: None,
               return_type: Some(CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
               }),
               arguments: vec![CppFunctionArgument {
                                 name: "x".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::None,
                                   is_const: false,
                                   base: CppTypeBase::Class {
                                     name: "Magic".to_string(),
                                     template_arguments: None,
                                   },
                                 },
                                 has_default_value: false,
                               }],
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
             });
  assert_eq!(data.methods[1],
             CppMethod {
               name: "func1".to_string(),
               class_membership: None,
               operator: None,
               return_type: Some(CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
               }),
               arguments: vec![CppFunctionArgument {
                                 name: "x".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::Ptr,
                                   is_const: false,
                                   base: CppTypeBase::Class {
                                     name: "Magic".to_string(),
                                     template_arguments: None,
                                   },
                                 },
                                 has_default_value: false,
                               }],
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
             });
  assert_eq!(data.methods[2],
             CppMethod {
               name: "func2".to_string(),
               class_membership: None,
               operator: None,
               return_type: Some(CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
               }),
               arguments: vec![CppFunctionArgument {
                                 name: "arg1".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::Ref,
                                   is_const: true,
                                   base: CppTypeBase::Class {
                                     name: "Magic".to_string(),
                                     template_arguments: None,
                                   },
                                 },
                                 has_default_value: false,
                               }],
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
             });
}

fn func_with_unknown_type() {
  let data = run_parser("class SomeClass; \n int func1(SomeClass* x);\n");
  assert!(data.template_instantiations.is_empty());
  assert!(data.types.is_empty());
  assert!(data.methods.is_empty());
}

#[test]
fn tests() {
  // clang can't be used from multiple threads, so these checks
  // must be run consequently
  simple_func();
  simple_func_with_default_value();
  functions_with_class_arg();
  func_with_unknown_type();
}
