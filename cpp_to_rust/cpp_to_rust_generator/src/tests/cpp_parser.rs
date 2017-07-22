extern crate tempdir;

use cpp_parser;
use cpp_data::*;
use cpp_method::*;
use cpp_type::*;
use cpp_operator::CppOperator;
use common::file_utils::{create_dir, create_file, PathBufWithAdded};

use std::path::PathBuf;

fn run_parser(code: &'static str) -> ParserCppData {
  let dir = tempdir::TempDir::new("test_cpp_parser_run").unwrap();
  let include_dir = dir.path().with_added("include");
  create_dir(&include_dir).unwrap();
  let include_name = "myfakelib.h";
  let include_file_path = include_dir.with_added(&include_name);
  {
    let mut include_file = create_file(&include_file_path).unwrap();
    include_file.write(code).unwrap();
    include_file.write("\n").unwrap();
  }
  let mut result = cpp_parser::run(cpp_parser::CppParserConfig {
                                     include_paths: vec![include_dir],
                                     include_directives: vec![PathBuf::from(include_name)],
                                     target_include_paths: Vec::new(),
                                     tmp_cpp_path: dir.path().with_added("1.cpp"),
                                     name_blacklist: Vec::new(),
                                     framework_paths: Vec::new(),
                                     clang_arguments: Vec::new(),
                                   },
                                   &[])
      .unwrap();
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

#[test]
fn simple_func() {
  let data = run_parser("int func1(int x);");
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
                 is_const2: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
               },
               arguments: vec![CppMethodArgument {
                                 name: "x".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::None,
                                   is_const: false,
                                   is_const2: false,
                                   base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
                                 },
                                 has_default_value: false,
                               }],
               arguments_before_omitting: None,
               doc: None,
               inheritance_chain: Vec::new(),
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
               template_arguments_values: None,
               declaration_code: Some("int func1 ( int x )".to_string()),
               is_ffi_whitelisted: false,
               is_unsafe_static_cast: false,
               is_direct_static_cast: false,
             });
}

#[test]
fn simple_func_with_default_value() {
  let data = run_parser("bool func1(int x = 42) {\nreturn false;\n}");
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
                 is_const2: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
               },
               arguments: vec![CppMethodArgument {
                                 name: "x".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::None,
                                   is_const: false,
                                   is_const2: false,
                                   base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
                                 },
                                 has_default_value: true,
                               }],
               arguments_before_omitting: None,
               doc: None,
               inheritance_chain: Vec::new(),
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
               template_arguments_values: None,
               declaration_code: Some("bool func1 ( int x = 42 )".to_string()),
               is_ffi_whitelisted: false,
               is_unsafe_static_cast: false,
               is_direct_static_cast: false,
             });
}

#[test]
fn functions_with_class_arg() {
  let data = run_parser("class Magic { public: int a, b; };
  bool func1(Magic x);
  bool func1(Magic* x);
  bool func2(const Magic&);");
  assert_eq!(data.types.len(), 1);
  assert_eq!(data.types[0].name, "Magic");
  if let CppTypeKind::Class {
           ref bases,
           ref fields,
           ref template_arguments,
           ref using_directives,
         } = data.types[0].kind {
    assert!(template_arguments.is_none());
    assert!(using_directives.is_empty());
    assert!(bases.is_empty());
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "a");
    assert_eq!(fields[0].field_type,
               CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 is_const2: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
               });
    assert_eq!(fields[0].visibility, CppVisibility::Public);

    assert_eq!(fields[1].name, "b");
    assert_eq!(fields[1].field_type,
               CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 is_const2: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
               });
    assert_eq!(fields[1].visibility, CppVisibility::Public);
  } else {
    panic!("invalid type kind");
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
                 is_const2: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
               },
               arguments: vec![CppMethodArgument {
                                 name: "x".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::None,
                                   is_const: false,
                                   is_const2: false,
                                   base: CppTypeBase::Class(CppTypeClassBase {
                                                              name: "Magic".to_string(),
                                                              template_arguments: None,
                                                            }),
                                 },
                                 has_default_value: false,
                               }],
               arguments_before_omitting: None,
               doc: None,
               inheritance_chain: Vec::new(),
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
               template_arguments_values: None,
               declaration_code: Some("bool func1 ( Magic x )".to_string()),
               is_ffi_whitelisted: false,
               is_unsafe_static_cast: false,
               is_direct_static_cast: false,
             });
  assert_eq!(data.methods[1],
             CppMethod {
               name: "func1".to_string(),
               class_membership: None,
               operator: None,
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 is_const2: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
               },
               arguments: vec![CppMethodArgument {
                                 name: "x".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::Ptr,
                                   is_const: false,
                                   is_const2: false,
                                   base: CppTypeBase::Class(CppTypeClassBase {
                                                              name: "Magic".to_string(),
                                                              template_arguments: None,
                                                            }),
                                 },
                                 has_default_value: false,
                               }],
               arguments_before_omitting: None,
               doc: None,
               inheritance_chain: Vec::new(),
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
               template_arguments_values: None,
               declaration_code: Some("bool func1 ( Magic * x )".to_string()),
               is_ffi_whitelisted: false,
               is_unsafe_static_cast: false,
               is_direct_static_cast: false,
             });
  assert_eq!(data.methods[2],
             CppMethod {
               name: "func2".to_string(),
               class_membership: None,
               operator: None,
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 is_const2: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
               },
               arguments: vec![CppMethodArgument {
                                 name: "arg1".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::Ref,
                                   is_const: true,
                                   is_const2: false,
                                   base: CppTypeBase::Class(CppTypeClassBase {
                                                              name: "Magic".to_string(),
                                                              template_arguments: None,
                                                            }),
                                 },
                                 has_default_value: false,
                               }],
               arguments_before_omitting: None,
               doc: None,
               inheritance_chain: Vec::new(),
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
               template_arguments_values: None,
               declaration_code: Some("bool func2 ( const Magic & )".to_string()),
               is_ffi_whitelisted: false,
               is_unsafe_static_cast: false,
               is_direct_static_cast: false,
             });
}

#[test]
fn func_with_unknown_type() {
  let data = run_parser("class SomeClass; \n int func1(SomeClass* x);");
  assert!(data.types.is_empty());
  assert!(data.methods.is_empty());
}

#[test]
fn variadic_func() {
  let data = run_parser("int my_printf ( const char * format, ... );");
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
                 is_const2: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
               },
               arguments: vec![CppMethodArgument {
                                 name: "format".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::Ptr,
                                   is_const: true,
                                   is_const2: false,
                                   base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Char),
                                 },
                                 has_default_value: false,
                               }],
               arguments_before_omitting: None,
               doc: None,
               inheritance_chain: Vec::new(),
               allows_variadic_arguments: true,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
               template_arguments_values: None,
               declaration_code: Some("int my_printf ( const char * format , ... )".to_string()),
               is_ffi_whitelisted: false,
               is_unsafe_static_cast: false,
               is_direct_static_cast: false,
             });
}

#[test]
fn free_template_func() {
  let data = run_parser("template<typename T> T abs(T value) { return 2*value; }");
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
                 is_const2: false,
                 base: CppTypeBase::TemplateParameter {
                   nested_level: 0,
                   index: 0,
                 },
               },
               arguments: vec![CppMethodArgument {
                                 name: "value".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::None,
                                   is_const: false,
                                   is_const2: false,
                                   base: CppTypeBase::TemplateParameter {
                                     nested_level: 0,
                                     index: 0,
                                   },
                                 },
                                 has_default_value: false,
                               }],
               arguments_before_omitting: None,
               doc: None,
               inheritance_chain: Vec::new(),
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: Some(TemplateArgumentsDeclaration {
                                          nested_level: 0,
                                          names: vec!["T".to_string()],
                                        }),
               template_arguments_values: None,
               declaration_code: Some("template < typename T > T abs ( T value )".to_string()),
               is_ffi_whitelisted: false,
               is_unsafe_static_cast: false,
               is_direct_static_cast: false,
             });
}

#[test]
fn free_func_operator_sub() {
  for code in &["class C1 {}; \n C1 operator-(C1 a, C1 b);",
                "class C1 {}; \n C1 operator -(C1 a, C1 b);"] {
    let data = run_parser(code);
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
                   is_const2: false,
                   base: CppTypeBase::Class(CppTypeClassBase {
                                              name: "C1".to_string(),
                                              template_arguments: None,
                                            }),
                 },
                 arguments: vec![CppMethodArgument {
                                   name: "a".to_string(),
                                   argument_type: CppType {
                                     indirection: CppTypeIndirection::None,
                                     is_const: false,
                                     is_const2: false,
                                     base: CppTypeBase::Class(CppTypeClassBase {
                                                                name: "C1".to_string(),
                                                                template_arguments: None,
                                                              }),
                                   },
                                   has_default_value: false,
                                 },
                                 CppMethodArgument {
                                   name: "b".to_string(),
                                   argument_type: CppType {
                                     indirection: CppTypeIndirection::None,
                                     is_const: false,
                                     is_const2: false,
                                     base: CppTypeBase::Class(CppTypeClassBase {
                                                                name: "C1".to_string(),
                                                                template_arguments: None,
                                                              }),
                                   },
                                   has_default_value: false,
                                 }],
                 arguments_before_omitting: None,
                 doc: None,
                 inheritance_chain: Vec::new(),
                 allows_variadic_arguments: false,
                 include_file: "myfakelib.h".to_string(),
                 origin_location: None,
                 template_arguments: None,
                 template_arguments_values: None,
                 declaration_code: Some("C1 operator - ( C1 a , C1 b )".to_string()),
                 is_ffi_whitelisted: false,
                 is_unsafe_static_cast: false,
                 is_direct_static_cast: false,
               });
  }
}

#[test]
fn simple_class_method() {
  let data = run_parser("class MyClass {
    public:
      int func1(int x);
    private:
      int m_x;
    };");
  assert!(data.types.len() == 1);
  assert_eq!(data.types[0].name, "MyClass");
  if let CppTypeKind::Class {
           ref bases,
           ref fields,
           ref template_arguments,
           ..
         } = data.types[0].kind {
    assert!(template_arguments.is_none());
    assert!(bases.is_empty());
    assert!(fields.len() == 1);
  } else {
    panic!("invalid type kind");
  }
  assert!(data.methods.len() == 1);
  assert_eq!(data.methods[0],
             CppMethod {
               name: "func1".to_string(),
               class_membership: Some(CppMethodClassMembership {
                                        class_type: CppTypeClassBase {
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
                                        is_slot: false,
                                        fake: None,
                                      }),
               operator: None,
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 is_const2: false,
                 base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
               },
               arguments: vec![CppMethodArgument {
                                 name: "x".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::None,
                                   is_const: false,
                                   is_const2: false,
                                   base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
                                 },
                                 has_default_value: false,
                               }],
               arguments_before_omitting: None,
               doc: None,
               inheritance_chain: Vec::new(),
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
               template_arguments_values: None,
               declaration_code: Some("int func1 ( int x )".to_string()),
               is_ffi_whitelisted: false,
               is_unsafe_static_cast: false,
               is_direct_static_cast: false,
             });
}

#[cfg_attr(feature="clippy", allow(cyclomatic_complexity))]
#[test]
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
  assert!(data.methods[0]
            .class_membership
            .as_ref()
            .unwrap()
            .kind
            .is_constructor());
  assert_eq!(data.methods[0].arguments.len(), 3);
  assert_eq!(data.methods[0].return_type, CppType::void());

  assert_eq!(data.methods[1].name, "~MyClass");
  assert!(data.methods[1]
            .class_membership
            .as_ref()
            .unwrap()
            .kind
            .is_destructor());
  assert_eq!(data.methods[1].arguments.len(), 0);
  assert_eq!(data.methods[1].return_type, CppType::void());

  assert_eq!(data.methods[2].name, "func1");
  assert!(data.methods[2]
            .class_membership
            .as_ref()
            .unwrap()
            .is_static);

  assert_eq!(data.methods[3].name, "func2");
  assert!(data.methods[3]
            .class_membership
            .as_ref()
            .unwrap()
            .is_virtual);
  assert!(!data.methods[3]
             .class_membership
             .as_ref()
             .unwrap()
             .is_pure_virtual);
  assert_eq!(data.methods[3]
               .class_membership
               .as_ref()
               .unwrap()
               .visibility,
             CppVisibility::Public);

  assert_eq!(data.methods[4].name, "func3");
  assert!(data.methods[4]
            .class_membership
            .as_ref()
            .unwrap()
            .is_virtual);
  assert!(data.methods[4]
            .class_membership
            .as_ref()
            .unwrap()
            .is_pure_virtual);
  assert_eq!(data.methods[4]
               .class_membership
               .as_ref()
               .unwrap()
               .visibility,
             CppVisibility::Protected);

  assert_eq!(data.methods[5].name, "func4");
  assert!(data.methods[5]
            .class_membership
            .as_ref()
            .unwrap()
            .is_const);

  assert_eq!(data.methods[6].name, "operator bool");
  assert!(data.methods[6]
            .class_membership
            .as_ref()
            .unwrap()
            .is_const);
  assert_eq!(data.methods[6].operator,
             Some(CppOperator::Conversion(CppType {
               indirection: CppTypeIndirection::None,
               is_const: false,
               is_const2: false,
               base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
             })));
  assert_eq!(data.methods[6].return_type,
             CppType {
               indirection: CppTypeIndirection::None,
               is_const: false,
               is_const2: false,
               base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Bool),
             });

  assert_eq!(data.methods[7].name, "func6");
  assert_eq!(data.methods[7].template_arguments,
             Some(TemplateArgumentsDeclaration {
                    nested_level: 0,
                    names: vec!["K".to_string(), "V".to_string()],
                  }));
  assert_eq!(data.methods[7].arguments.len(), 1);
  assert_eq!(data.methods[7].arguments[0].argument_type,
             CppType {
               indirection: CppTypeIndirection::None,
               is_const: false,
               is_const2: false,
               base: CppTypeBase::TemplateParameter {
                 nested_level: 0,
                 index: 1,
               },
             });
}

#[test]
fn template_class_method() {
  let data = run_parser("
  template<class T>
  class MyVector {
    public:
      class Iterator {};
      T get(int index);
      Iterator begin();
    };");
  assert!(data.types.len() == 1);
  assert_eq!(data.types[0].name, "MyVector");
  if let CppTypeKind::Class {
           ref bases,
           ref fields,
           ref template_arguments,
           ..
         } = data.types[0].kind {
    assert_eq!(template_arguments,
               &Some(TemplateArgumentsDeclaration {
                       nested_level: 0,
                       names: vec!["T".to_string()],
                     }));
    assert!(bases.is_empty());
    assert!(fields.is_empty());
  } else {
    panic!("invalid type kind");
  }
  assert!(data.methods.len() == 1);
  assert_eq!(data.methods[0],
             CppMethod {
               name: "get".to_string(),
               class_membership: Some(CppMethodClassMembership {
                                        class_type: CppTypeClassBase {
                                          name: "MyVector".to_string(),
                                          template_arguments: Some(vec![CppType {
                                                   indirection: CppTypeIndirection::None,
                                                   is_const: false,
                                                   is_const2: false,
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
                                        is_slot: false,
                                        fake: None,
                                      }),
               operator: None,
               return_type: CppType {
                 indirection: CppTypeIndirection::None,
                 is_const: false,
                 is_const2: false,
                 base: CppTypeBase::TemplateParameter {
                   nested_level: 0,
                   index: 0,
                 },
               },
               arguments: vec![CppMethodArgument {
                                 name: "index".to_string(),
                                 argument_type: CppType {
                                   indirection: CppTypeIndirection::None,
                                   is_const: false,
                                   is_const2: false,
                                   base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
                                 },
                                 has_default_value: false,
                               }],
               arguments_before_omitting: None,
               doc: None,
               inheritance_chain: Vec::new(),
               allows_variadic_arguments: false,
               include_file: "myfakelib.h".to_string(),
               origin_location: None,
               template_arguments: None,
               template_arguments_values: None,
               declaration_code: Some("T get ( int index )".to_string()),
               is_ffi_whitelisted: false,
               is_unsafe_static_cast: false,
               is_direct_static_cast: false,
             });
}

#[test]
fn template_class_template_method() {
  let data = run_parser("
  template<class T>
  class MyVector {
    public:
      template<typename F>
      F get_f();

      T get_t();
    };");
  assert_eq!(data.methods[0].name, "get_f");
  assert_eq!(data.methods[0].template_arguments,
             Some(TemplateArgumentsDeclaration {
                    nested_level: 1,
                    names: vec!["F".to_string()],
                  }));
  assert_eq!(data.methods[0].return_type,
             CppType {
               indirection: CppTypeIndirection::None,
               is_const: false,
               is_const2: false,
               base: CppTypeBase::TemplateParameter {
                 nested_level: 1,
                 index: 0,
               },
             });

  assert_eq!(data.methods[1].name, "get_t");
  assert_eq!(data.methods[1].template_arguments, None);
  assert_eq!(data.methods[1].return_type,
             CppType {
               indirection: CppTypeIndirection::None,
               is_const: false,
               is_const2: false,
               base: CppTypeBase::TemplateParameter {
                 nested_level: 0,
                 index: 0,
               },
             });

}

#[test]
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
               values: vec![CppEnumValue {
                              name: "Good".to_string(),
                              value: 0,
                              doc: None,
                            },
                            CppEnumValue {
                              name: "Bad".to_string(),
                              value: 1,
                              doc: None,
                            }],
             });
}

#[test]
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
               values: vec![CppEnumValue {
                              name: "Good".to_string(),
                              value: 1,
                              doc: None,
                            },
                            CppEnumValue {
                              name: "Bad".to_string(),
                              value: 2,
                              doc: None,
                            },
                            CppEnumValue {
                              name: "Questionable".to_string(),
                              value: 3,
                              doc: None,
                            }],
             });
}

#[test]
fn template_instantiation() {
  let data = run_parser("
  template<typename T> class Vector {};
  class C1 {
  public:
    Vector<int> values();
  };
");
  assert_eq!(data.methods.len(), 1);
  let int = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int),
  };
  assert_eq!(data.methods[0].return_type,
             CppType {
               indirection: CppTypeIndirection::None,
               is_const: false,
               is_const2: false,
               base: CppTypeBase::Class(CppTypeClassBase {
                                          name: "Vector".to_string(),
                                          template_arguments: Some(vec![int.clone()]),
                                        }),
             });
  // TODO: test template_instantiations
  /*
  assert!(data
            .template_instantiations
            .iter()
            .find(|x| &x.class_name == "Vector")
            .is_some());
  assert!(data
            .template_instantiations
            .iter()
            .find(|x| &x.class_name == "Vector")
            .unwrap()
            .instantiations
            .len() == 1);
  assert!(&data
             .template_instantiations
             .iter()
             .find(|x| &x.class_name == "Vector")
             .unwrap()
             .instantiations
             .get(0)
             .unwrap()
             .template_arguments == &vec![int]);*/
}

#[test]
fn derived_class_simple() {
  let data = run_parser("class Base {}; class Derived : public Base {};");
  assert!(data.types.len() == 2);
  assert_eq!(data.types[0].name, "Base");
  if let CppTypeKind::Class { ref bases, .. } = data.types[0].kind {
    assert!(bases.is_empty());
  } else {
    panic!("invalid type kind");
  }
  assert_eq!(data.types[1].name, "Derived");
  if let CppTypeKind::Class { ref bases, .. } = data.types[1].kind {
    assert_eq!(bases,
               &vec![CppBaseSpecifier {
                       base_type: CppType {
                         indirection: CppTypeIndirection::None,
                         is_const: false,
                         is_const2: false,
                         base: CppTypeBase::Class(CppTypeClassBase {
                                                    name: "Base".to_string(),
                                                    template_arguments: None,
                                                  }),
                       },
                       is_virtual: false,
                       visibility: CppVisibility::Public,
                     }]);
  } else {
    panic!("invalid type kind");
  }
}

#[test]
fn derived_class_simple_private() {
  let data = run_parser("class Base {}; class Derived : Base {};");
  assert!(data.types.len() == 2);
  assert_eq!(data.types[0].name, "Base");
  if let CppTypeKind::Class { ref bases, .. } = data.types[0].kind {
    assert!(bases.is_empty());
  } else {
    panic!("invalid type kind");
  }
  assert_eq!(data.types[1].name, "Derived");
  if let CppTypeKind::Class { ref bases, .. } = data.types[1].kind {
    assert_eq!(bases,
               &vec![CppBaseSpecifier {
                       base_type: CppType {
                         indirection: CppTypeIndirection::None,
                         is_const: false,
                         is_const2: false,
                         base: CppTypeBase::Class(CppTypeClassBase {
                                                    name: "Base".to_string(),
                                                    template_arguments: None,
                                                  }),
                       },
                       is_virtual: false,
                       visibility: CppVisibility::Private,
                     }]);
  } else {
    panic!("invalid type kind");
  }
}

#[test]
fn derived_class_simple_virtual() {
  let data = run_parser("class Base {}; class Derived : public virtual Base {};");
  assert!(data.types.len() == 2);
  assert_eq!(data.types[0].name, "Base");
  if let CppTypeKind::Class { ref bases, .. } = data.types[0].kind {
    assert!(bases.is_empty());
  } else {
    panic!("invalid type kind");
  }
  assert_eq!(data.types[1].name, "Derived");
  if let CppTypeKind::Class { ref bases, .. } = data.types[1].kind {
    assert_eq!(bases,
               &vec![CppBaseSpecifier {
                       base_type: CppType {
                         indirection: CppTypeIndirection::None,
                         is_const: false,
                         is_const2: false,
                         base: CppTypeBase::Class(CppTypeClassBase {
                                                    name: "Base".to_string(),
                                                    template_arguments: None,
                                                  }),
                       },
                       is_virtual: true,
                       visibility: CppVisibility::Public,
                     }]);
  } else {
    panic!("invalid type kind");
  }
}

#[test]
fn derived_class_multiple() {
  let data = run_parser("
    class Base1 {}; class Base2 {};
    class Derived : public Base2, public Base1 {};");
  assert!(data.types.len() == 3);
  assert_eq!(data.types[0].name, "Base1");
  assert_eq!(data.types[1].name, "Base2");
  assert_eq!(data.types[2].name, "Derived");
  if let CppTypeKind::Class { ref bases, .. } = data.types[2].kind {
    assert_eq!(bases,
               &vec![CppBaseSpecifier {
                       base_type: CppType {
                         indirection: CppTypeIndirection::None,
                         is_const: false,
                         is_const2: false,
                         base: CppTypeBase::Class(CppTypeClassBase {
                                                    name: "Base2".to_string(),
                                                    template_arguments: None,
                                                  }),
                       },
                       is_virtual: false,
                       visibility: CppVisibility::Public,
                     },
                     CppBaseSpecifier {
                       base_type: CppType {
                         indirection: CppTypeIndirection::None,
                         is_const: false,
                         is_const2: false,
                         base: CppTypeBase::Class(CppTypeClassBase {
                                                    name: "Base1".to_string(),
                                                    template_arguments: None,
                                                  }),
                       },
                       is_virtual: false,
                       visibility: CppVisibility::Public,
                     }]);
  } else {
    panic!("invalid type kind");
  }
}

#[test]
fn class_with_use() {
  let data = run_parser("class A { public: int m1(); };
  class B { public: double m1(); };
  class C : public A, public B {
    using B::m1;
  };");
  assert!(data.types.len() == 3);
  assert_eq!(data.types[2].name, "C");
  if let CppTypeKind::Class { ref using_directives, .. } = data.types[2].kind {
    assert_eq!(using_directives,
               &vec![CppClassUsingDirective {
                       class_name: "B".to_string(),
                       method_name: "m1".to_string(),
                     }]);
  } else {
    panic!("invalid type kind");
  }
}

#[test]
fn complex_const_types() {
  let data = run_parser("
    int f0();
    const int f1();
    int* f2();
    const int* f3();
    int* const f4();
    int** f5();
    int* const* f6();
    const int* const* f7();
    int const* const* f8();
    int const* const* const f9();
  ");
  let base = CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::Int);
  assert_eq!(data.methods.len(), 10);
  assert_eq!(&data.methods[0].return_type,
             &CppType {
                base: base.clone(),
                indirection: CppTypeIndirection::None,
                is_const: false,
                is_const2: false,
              });
  assert_eq!(&data.methods[1].return_type,
             &CppType {
                base: base.clone(),
                indirection: CppTypeIndirection::None,
                is_const: true,
                is_const2: false,
              });
  assert_eq!(&data.methods[2].return_type,
             &CppType {
                base: base.clone(),
                indirection: CppTypeIndirection::Ptr,
                is_const: false,
                is_const2: false,
              });
  assert_eq!(&data.methods[3].return_type,
             &CppType {
                base: base.clone(),
                indirection: CppTypeIndirection::Ptr,
                is_const: true,
                is_const2: false,
              });
  assert_eq!(&data.methods[4].return_type,
             &CppType {
                base: base.clone(),
                indirection: CppTypeIndirection::Ptr,
                is_const: false,
                is_const2: false,
              });
  assert_eq!(&data.methods[5].return_type,
             &CppType {
                base: base.clone(),
                indirection: CppTypeIndirection::PtrPtr,
                is_const: false,
                is_const2: false,
              });
  assert_eq!(&data.methods[6].return_type,
             &CppType {
                base: base.clone(),
                indirection: CppTypeIndirection::PtrPtr,
                is_const: false,
                is_const2: true,
              });
  assert_eq!(&data.methods[7].return_type,
             &CppType {
                base: base.clone(),
                indirection: CppTypeIndirection::PtrPtr,
                is_const: true,
                is_const2: true,
              });
  assert_eq!(&data.methods[8].return_type,
             &CppType {
                base: base.clone(),
                indirection: CppTypeIndirection::PtrPtr,
                is_const: true,
                is_const2: true,
              });
  assert_eq!(&data.methods[9].return_type,
             &CppType {
                base: base.clone(),
                indirection: CppTypeIndirection::PtrPtr,
                is_const: true,
                is_const2: true,
              });
}

#[test]
fn anon_enum() {
  let data = run_parser("class X {
    enum { v1, v2 } field;
  };");
  assert!(data.types.len() == 1);
  assert_eq!(data.types[0].name, "X");
  if let CppTypeKind::Class { ref fields, .. } = data.types[0].kind {
    assert!(fields.is_empty());
  } else {
    panic!("invalid type kind");
  }
}

#[test]
fn non_type_template_parameter() {
  let data = run_parser("\
  template<int> struct QAtomicOpsSupport { enum { IsSupported = 0 }; };
  template<> struct QAtomicOpsSupport<4> { enum { IsSupported = 1 }; };");
  assert!(data.types.is_empty());
}

#[test]
fn fixed_size_integers() {
  let data = run_parser("
  typedef unsigned long long int GLuint64;
  template<typename T> class QVector {};
  GLuint64 f1();
  QVector<GLuint64> f2();
  ");
  assert_eq!(data.methods.len(), 2);
  assert_eq!(&data.methods[0].name, "f1");
  let type1 = CppType {
    indirection: CppTypeIndirection::None,
    is_const: false,
    is_const2: false,
    base: CppTypeBase::SpecificNumeric(CppSpecificNumericType {
                                         name: "GLuint64".to_string(),
                                         bits: 64,
                                         kind: CppSpecificNumericTypeKind::Integer {
                                           is_signed: false,
                                         },
                                       }),
  };
  assert_eq!(&data.methods[0].return_type, &type1);

  assert_eq!(&data.methods[1].name, "f2");
  assert_eq!(&data.methods[1].return_type,
             &CppType {
                indirection: CppTypeIndirection::None,
                is_const: false,
                is_const2: false,
                base: CppTypeBase::Class(CppTypeClassBase {
                                           name: "QVector".to_string(),
                                           template_arguments: Some(vec![type1.clone()]),
                                         }),
              });

}


#[test]
fn template_class_with_base() {
  let data = run_parser("
  template<class T>
  class C1 {};

  template<class T>
  class C2: public C1<T> {};
  ");
  assert!(data.types.len() == 2);
  assert_eq!(data.types[0].name, "C1");
  if let CppTypeKind::Class {
           ref bases,
           ref fields,
           ref template_arguments,
           ..
         } = data.types[0].kind {
    assert_eq!(template_arguments,
               &Some(TemplateArgumentsDeclaration {
                       nested_level: 0,
                       names: vec!["T".to_string()],
                     }));
    assert!(bases.is_empty());
    assert!(fields.is_empty());
  } else {
    panic!("invalid type kind");
  }

  assert_eq!(data.types[1].name, "C2");
  if let CppTypeKind::Class {
           ref bases,
           ref fields,
           ref template_arguments,
           ..
         } = data.types[1].kind {
    assert_eq!(template_arguments,
               &Some(TemplateArgumentsDeclaration {
                       nested_level: 0,
                       names: vec!["T".to_string()],
                     }));
    assert_eq!(bases.len(), 1);
    assert!(fields.is_empty());
  } else {
    panic!("invalid type kind");
  }
}
