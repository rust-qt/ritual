
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
use cpp_operator::CppOperator;


fn run_parser(code: &'static str) -> CppData {
  let dir = tempdir::TempDir::new("test_cpp_parser_run").unwrap();
  let include_dir = dir.path().with_added("include");
  fs::create_dir(&include_dir).unwrap();
  let include_name = "myfakelib.h";
  let include_file_path = include_dir.with_added(&include_name);
  {
    let mut include_file = File::create(&include_file_path).unwrap();
    include_file.write(code.as_bytes()).unwrap();
    include_file.write("\n".as_bytes()).unwrap();
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
  let data = run_parser("int func1(int x);");
  assert!(data.template_instantiations.is_empty());
  assert!(data.types.is_empty());
  assert!(data.methods.len() == 1);
  assert_eq!(data.methods[0],
             CppMethod {
               name: "func1".to_string(),
               class_membership: None,
               operator: None,
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
               },
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
  let data = run_parser("bool func1(int x = 42);");
  assert!(data.template_instantiations.is_empty());
  assert!(data.types.is_empty());
  assert!(data.methods.len() == 1);
  assert_eq!(data.methods[0],
             CppMethod {
               name: "func1".to_string(),
               class_membership: None,
               operator: None,
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
               },
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
  bool func2(const Magic&);");
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
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
               },
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
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
               },
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
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
               },
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
  let data = run_parser("class SomeClass; \n int func1(SomeClass* x);");
  assert!(data.template_instantiations.is_empty());
  assert!(data.types.is_empty());
  assert!(data.methods.is_empty());
}

fn variadic_func() {
  let data = run_parser("int my_printf ( const char * format, ... );");
  assert!(data.template_instantiations.is_empty());
  assert!(data.types.is_empty());
  assert!(data.methods.len() == 1);
  assert_eq!(data.methods[0],
             CppMethod {
               name: "my_printf".to_string(),
               class_membership: None,
               operator: None,
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
               },
               arguments: vec![CppFunctionArgument {
                                 name: "format".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::Ptr,
                                   is_const: true,
                                   base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Char),
                                 },
                                 has_default_value: false,
                               }],
               allows_variadic_arguments: true,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
             });
}

fn free_template_func() {
  let data = run_parser("template<typename T> T abs(T value) { return 2*value; }");
  assert!(data.template_instantiations.is_empty());
  assert!(data.types.is_empty());
  assert!(data.methods.len() == 1);
  assert_eq!(data.methods[0],
             CppMethod {
               name: "abs".to_string(),
               class_membership: None,
               operator: None,
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::TemplateParameter {
                   nested_level: 0,
                   index: 0,
                 },
               },
               arguments: vec![CppFunctionArgument {
                                 name: "value".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::None,
                                   is_const: false,
                                   base: CppTypeBase::TemplateParameter {
                                     nested_level: 0,
                                     index: 0,
                                   },
                                 },
                                 has_default_value: false,
                               }],
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: Some(vec!["T".to_string()]),
             });
}

fn free_func_operator_sub() {
  for code in &["class C1 {}; \n C1 operator-(C1 a, C1 b);",
                "class C1 {}; \n C1 operator -(C1 a, C1 b);"] {
    let data = run_parser(code);
    assert!(data.template_instantiations.is_empty());
    assert!(data.types.len() == 1);
    assert!(data.methods.len() == 1);
    assert_eq!(data.methods[0],
               CppMethod {
                 name: "operator-".to_string(),
                 class_membership: None,
                 operator: Some(CppOperator::Subtraction),
                 return_type: CppType {
                   indirection: CppTypeIndirection::None,
                   is_const: false,
                   base: CppTypeBase::Class {
                     name: "C1".to_string(),
                     template_arguments: None,
                   },
                 },
                 arguments: vec![CppFunctionArgument {
                                   name: "a".to_string(),
                                   argument_type: CppType {
                                     indirection: CppTypeIndirection::None,
                                     is_const: false,
                                     base: CppTypeBase::Class {
                                       name: "C1".to_string(),
                                       template_arguments: None,
                                     },
                                   },
                                   has_default_value: false,
                                 },
                                 CppFunctionArgument {
                                   name: "b".to_string(),
                                   argument_type: CppType {
                                     indirection: CppTypeIndirection::None,
                                     is_const: false,
                                     base: CppTypeBase::Class {
                                       name: "C1".to_string(),
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
}

fn simple_class_method() {
  let data = run_parser("class MyClass {
    public:
      int func1(int x);
    private:
      int m_x;
    };");
  assert!(data.template_instantiations.is_empty());
  assert!(data.types.len() == 1);
  assert_eq!(data.types[0].name, "MyClass");
  match data.types[0].kind {
    CppTypeKind::Class { ref size, ref bases, ref fields, ref template_arguments } => {
      assert!(size.is_some());
      assert!(template_arguments.is_none());
      assert!(bases.is_empty());
      assert!(fields.len() == 1);
    }
    _ => panic!("invalid type kind"),
  }
  assert!(data.methods.len() == 1);
  assert_eq!(data.methods[0],
             CppMethod {
               name: "func1".to_string(),
               class_membership: Some(CppMethodClassMembership {
                 class_type: CppTypeBase::Class {
                   name: "MyClass".to_string(),
                   template_arguments: None,
                 },
                 kind: CppMethodKind::Regular,
                 is_virtual: false,
                 is_pure_virtual: false,
                 is_const: false,
                 is_static: false,
                 visibility: CppVisibility::Public,
                 is_signal: false,
               }),
               operator: None,
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
               },
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

fn advanced_class_methods() {
  let data = run_parser("class MyClass {
    public:
      MyClass(bool a, bool b, bool c);
      virtual ~MyClass();
      static int func1(int x);
      virtual void func2();
    protected:
      virtual void func3() = 0;
    public:
      int func4() const { return 1; }
      operator bool() const;
      template<class K, class V>
      int func6(V x) const { return 1; }
    };");
  assert_eq!(data.methods.len(), 8);
  assert_eq!(data.methods[0].name, "MyClass");
  assert!(data.methods[0].class_membership.as_ref().unwrap().kind.is_constructor());
  assert_eq!(data.methods[0].arguments.len(), 3);
  assert_eq!(data.methods[0].return_type, CppType::void());

  assert_eq!(data.methods[1].name, "~MyClass");
  assert!(data.methods[1].class_membership.as_ref().unwrap().kind.is_destructor());
  assert_eq!(data.methods[1].arguments.len(), 0);
  assert_eq!(data.methods[1].return_type, CppType::void());

  assert_eq!(data.methods[2].name, "func1");
  assert!(data.methods[2].class_membership.as_ref().unwrap().is_static);

  assert_eq!(data.methods[3].name, "func2");
  assert!(data.methods[3].class_membership.as_ref().unwrap().is_virtual);
  assert!(!data.methods[3].class_membership.as_ref().unwrap().is_pure_virtual);
  assert_eq!(data.methods[3].class_membership.as_ref().unwrap().visibility,
             CppVisibility::Public);

  assert_eq!(data.methods[4].name, "func3");
  assert!(data.methods[4].class_membership.as_ref().unwrap().is_virtual);
  assert!(data.methods[4].class_membership.as_ref().unwrap().is_pure_virtual);
  assert_eq!(data.methods[4].class_membership.as_ref().unwrap().visibility,
             CppVisibility::Protected);

  assert_eq!(data.methods[5].name, "func4");
  assert!(data.methods[5].class_membership.as_ref().unwrap().is_const);

  assert_eq!(data.methods[6].name, "operator bool");
  assert!(data.methods[6].class_membership.as_ref().unwrap().is_const);
  assert_eq!(data.methods[6].operator,
             Some(CppOperator::Conversion(CppType {
               indirection: CppTypeIndirection::None,
               is_const: false,
               base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
             })));
  assert_eq!(data.methods[6].return_type,
             CppType {
               indirection: CppTypeIndirection::None,
               is_const: false,
               base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
             });

  assert_eq!(data.methods[7].name, "func6");
  assert_eq!(data.methods[7].template_arguments,
             Some(vec!["K".to_string(), "V".to_string()]));
  assert_eq!(data.methods[7].arguments.len(), 1);
  assert_eq!(data.methods[7].arguments[0].argument_type,
             CppType {
               indirection: CppTypeIndirection::None,
               is_const: false,
               base: CppTypeBase::TemplateParameter {
                 nested_level: 0,
                 index: 1,
               },
             });
}

fn template_class_method() {
  let data = run_parser("
  template<class T>
  class MyVector {
    public:
      class Iterator {};
      T get(int index);
      Iterator begin();
    };");
  assert!(data.template_instantiations.is_empty());
  assert!(data.types.len() == 2);
  assert_eq!(data.types[0].name, "MyVector");
  match data.types[0].kind {
    CppTypeKind::Class { ref size, ref bases, ref fields, ref template_arguments } => {
      assert!(size.is_none());
      assert_eq!(template_arguments, &Some(vec!["T".to_string()]));
      assert!(bases.is_empty());
      assert!(fields.is_empty());
    }
    _ => panic!("invalid type kind"),
  }
  assert_eq!(data.types[1].name, "MyVector::Iterator");
  assert!(data.methods.len() == 2);
  assert_eq!(data.methods[0],
             CppMethod {
               name: "get".to_string(),
               class_membership: Some(CppMethodClassMembership {
                 class_type: CppTypeBase::Class {
                   name: "MyVector".to_string(),
                   template_arguments: Some(vec![CppType {
                                                   indirection: CppTypeIndirection::None,
                                                   is_const: false,
                                                   base: CppTypeBase::TemplateParameter {
                                                     nested_level: 0,
                                                     index: 0,
                                                   },
                                                 }]),
                 },
                 kind: CppMethodKind::Regular,
                 is_virtual: false,
                 is_pure_virtual: false,
                 is_const: false,
                 is_static: false,
                 visibility: CppVisibility::Public,
                 is_signal: false,
               }),
               operator: None,
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 base: CppTypeBase::TemplateParameter {
                   nested_level: 0,
                   index: 0,
                 },
               },
               arguments: vec![CppFunctionArgument {
                                 name: "index".to_string(),
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
  assert_eq!(data.methods[1].name, "begin");
  assert_eq!(data.methods[1].return_type,
             CppType {
               indirection: CppTypeIndirection::None,
               is_const: false,
               base: CppTypeBase::Class {
                 name: "MyVector::Iterator".to_string(),
                 template_arguments: None,
               },
             });
}

fn simple_enum() {
  let data = run_parser("
  enum Enum1 {
    Good,
    Bad
  };");
  assert_eq!(data.types.len(), 1);
  assert_eq!(data.types[0].name, "Enum1");
  assert_eq!(data.types[0].kind,
             CppTypeKind::Enum {
               values: vec![EnumValue {
                              name: "Good".to_string(),
                              value: 0,
                            },
                            EnumValue {
                              name: "Bad".to_string(),
                              value: 1,
                            }],
             });
}

fn simple_enum2() {
  let data = run_parser("
  namespace ns1 {
    enum Enum1 {
      Good = 1,
      Bad = 2,
      Questionable = Good | Bad
    };
  }");
  assert_eq!(data.types.len(), 1);
  assert_eq!(data.types[0].name, "ns1::Enum1");
  assert_eq!(data.types[0].kind,
             CppTypeKind::Enum {
               values: vec![EnumValue {
                              name: "Good".to_string(),
                              value: 1,
                            },
                            EnumValue {
                              name: "Bad".to_string(),
                              value: 2,
                            },
                            EnumValue {
                              name: "Questionable".to_string(),
                              value: 3,
                            }],
             });
}

#[test]
fn tests() {
  // clang can't be used from multiple threads, so these checks
  // must be run consequently
  simple_func();
  simple_func_with_default_value();
  functions_with_class_arg();
  func_with_unknown_type();
  variadic_func();
  free_template_func();
  free_func_operator_sub();
  simple_class_method();
  advanced_class_methods();
  template_class_method();
  simple_enum();
  simple_enum2();
}
