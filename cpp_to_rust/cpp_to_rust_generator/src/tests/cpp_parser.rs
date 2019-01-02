use crate::common::file_utils::{create_dir, create_file, PathBufWithAdded};
use crate::cpp_data::*;
use crate::cpp_function::*;
use crate::cpp_operator::CppOperator;
use crate::cpp_type::*;

use crate::common::cpp_build_config::CppBuildPaths;
use crate::config::Config;
use crate::config::CrateProperties;
use crate::cpp_parser::cpp_parser_step;
use crate::processor;
use crate::workspace::Workspace;

struct ParserCppData {
    types: Vec<CppTypeData>,
    bases: Vec<CppBaseSpecifier>,
    fields: Vec<CppClassField>,
    methods: Vec<CppFunction>,
    enum_values: Vec<CppEnumValue>,
}

fn run_parser(code: &'static str) -> ParserCppData {
    let dir = tempdir::TempDir::new("test_cpp_parser_run").unwrap();

    let mut workspace = Workspace::new(dir.path().into()).unwrap();

    let include_dir = dir.path().with_added("include");
    create_dir(&include_dir).unwrap();
    let include_name = "myfakelib.h";
    let include_file_path = include_dir.with_added(&include_name);
    {
        let mut include_file = create_file(&include_file_path).unwrap();
        include_file.write(code).unwrap();
        include_file.write("\n").unwrap();
    }

    let mut paths = CppBuildPaths::new();
    paths.add_include_path(include_dir);

    let mut config = Config::new(CrateProperties::new("A", "0.0.0"));
    config.add_include_directive(include_name);
    config.set_cpp_build_paths(paths);

    processor::process(&mut workspace, &config, &[cpp_parser_step().name]).unwrap();

    let database = workspace.load_or_create_crate("A").unwrap();

    ParserCppData {
        types: database
            .items
            .iter()
            .filter_map(|item| item.cpp_data.as_type_ref())
            .cloned()
            .collect(),
        bases: database
            .items
            .iter()
            .filter_map(|item| item.cpp_data.as_base_ref())
            .cloned()
            .collect(),
        fields: database
            .items
            .iter()
            .filter_map(|item| item.cpp_data.as_field_ref())
            .cloned()
            .collect(),
        enum_values: database
            .items
            .iter()
            .filter_map(|item| item.cpp_data.as_enum_value_ref())
            .cloned()
            .collect(),
        methods: database
            .items
            .iter()
            .filter_map(|item| item.cpp_data.as_function_ref())
            .cloned()
            .collect(),
    }
}

#[test]
fn simple_func() {
    let data = run_parser("int func1(int x);");
    assert!(data.types.is_empty());
    assert!(data.methods.len() == 1);
    assert_eq!(
        data.methods[0],
        CppFunction {
            name: "func1".to_string(),
            member: None,
            operator: None,
            return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
            arguments: vec![CppFunctionArgument {
                name: "x".to_string(),
                argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
                has_default_value: false,
            }],
            doc: None,
            allows_variadic_arguments: false,
            template_arguments: None,
            declaration_code: Some("int func1 ( int x )".to_string()),
        }
    );
}

#[test]
fn simple_func_with_default_value() {
    let data = run_parser("bool func1(int x = 42) {\nreturn false;\n}");
    assert!(data.types.is_empty());
    assert!(data.methods.len() == 1);
    assert_eq!(
        data.methods[0],
        CppFunction {
            name: "func1".to_string(),
            member: None,
            operator: None,
            return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Bool),
            arguments: vec![CppFunctionArgument {
                name: "x".to_string(),
                argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
                has_default_value: true,
            }],
            doc: None,
            allows_variadic_arguments: false,
            template_arguments: None,
            declaration_code: Some("bool func1 ( int x = 42 )".to_string()),
        }
    );
}

#[test]
fn functions_with_class_arg() {
    let data = run_parser(
        "class Magic { public: int a, b; };
  bool func1(Magic x);
  bool func1(Magic* x);
  bool func2(const Magic&);",
    );
    assert_eq!(data.types.len(), 1);
    assert_eq!(data.types[0].name, "Magic");

    if let CppTypeDataKind::Class { ref type_base } = data.types[0].kind {
        assert!(type_base.template_arguments.is_none());
    } else {
        panic!("invalid type kind");
    }

    assert!(data.bases.is_empty());

    assert_eq!(data.fields.len(), 2);
    assert_eq!(data.fields[0].name, "a");
    assert_eq!(
        data.fields[0].field_type,
        CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
    );
    assert_eq!(data.fields[0].visibility, CppVisibility::Public);

    assert_eq!(data.fields[1].name, "b");
    assert_eq!(
        data.fields[1].field_type,
        CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
    );
    assert_eq!(data.fields[1].visibility, CppVisibility::Public);

    assert!(data.methods.len() == 3);
    assert_eq!(
        data.methods[0],
        CppFunction {
            name: "func1".to_string(),
            member: None,
            operator: None,
            return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Bool),
            arguments: vec![CppFunctionArgument {
                name: "x".to_string(),
                argument_type: CppType::Class(CppClassType {
                    name: "Magic".to_string(),
                    template_arguments: None,
                }),
                has_default_value: false,
            }],
            doc: None,
            allows_variadic_arguments: false,
            template_arguments: None,
            declaration_code: Some("bool func1 ( Magic x )".to_string()),
        }
    );
    assert_eq!(
        data.methods[1],
        CppFunction {
            name: "func1".to_string(),
            member: None,
            operator: None,
            return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Bool),
            arguments: vec![CppFunctionArgument {
                name: "x".to_string(),
                argument_type: CppType::new_pointer(
                    false,
                    CppType::Class(CppClassType {
                        name: "Magic".to_string(),
                        template_arguments: None,
                    })
                ),
                has_default_value: false,
            }],
            doc: None,
            allows_variadic_arguments: false,
            template_arguments: None,
            declaration_code: Some("bool func1 ( Magic * x )".to_string()),
        }
    );
    assert_eq!(
        data.methods[2],
        CppFunction {
            name: "func2".to_string(),
            member: None,
            operator: None,
            return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Bool),
            arguments: vec![CppFunctionArgument {
                name: "arg1".to_string(),
                argument_type: CppType::new_reference(
                    true,
                    CppType::Class(CppClassType {
                        name: "Magic".to_string(),
                        template_arguments: None,
                    })
                ),
                has_default_value: false,
            }],
            doc: None,
            allows_variadic_arguments: false,
            template_arguments: None,
            declaration_code: Some("bool func2 ( const Magic & )".to_string()),
        }
    );
}

#[test]
fn func_with_unknown_type() {
    let data = run_parser("class SomeClass; \n int func1(SomeClass* x);");
    assert!(data.types.is_empty());
    assert_eq!(data.methods.len(), 1);
}

#[test]
fn variadic_func() {
    let data = run_parser("int my_printf ( const char * format, ... );");
    assert!(data.types.is_empty());
    assert!(data.methods.len() == 1);
    assert_eq!(
        data.methods[0],
        CppFunction {
            name: "my_printf".to_string(),
            member: None,
            operator: None,
            return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
            arguments: vec![CppFunctionArgument {
                name: "format".to_string(),
                argument_type: CppType::new_pointer(
                    true,
                    CppType::BuiltInNumeric(CppBuiltInNumericType::Char)
                ),
                has_default_value: false,
            }],
            doc: None,
            allows_variadic_arguments: true,
            template_arguments: None,
            declaration_code: Some("int my_printf ( const char * format , ... )".to_string()),
        }
    );
}

#[test]
fn free_template_func() {
    let data = run_parser("template<typename T> T abs(T value) { return 2*value; }");
    assert!(data.types.is_empty());
    assert!(data.methods.len() == 1);
    assert_eq!(
        data.methods[0],
        CppFunction {
            name: "abs".to_string(),
            member: None,
            operator: None,
            return_type: CppType::TemplateParameter {
                nested_level: 0,
                index: 0,
                name: "T".into(),
            },
            arguments: vec![CppFunctionArgument {
                name: "value".to_string(),
                argument_type: CppType::TemplateParameter {
                    nested_level: 0,
                    index: 0,
                    name: "T".into(),
                },
                has_default_value: false,
            }],
            doc: None,
            allows_variadic_arguments: false,
            template_arguments: Some(vec![CppType::TemplateParameter {
                nested_level: 0,
                index: 0,
                name: "T".into(),
            }]),
            declaration_code: Some("template < typename T > T abs ( T value )".to_string()),
        }
    );
}

#[test]
fn free_func_operator_sub() {
    for code in &[
        "class C1 {}; \n C1 operator-(C1 a, C1 b);",
        "class C1 {}; \n C1 operator -(C1 a, C1 b);",
    ] {
        let data = run_parser(code);
        assert!(data.types.len() == 1);
        assert!(data.methods.len() == 1);
        assert_eq!(
            data.methods[0],
            CppFunction {
                name: "operator-".to_string(),
                member: None,
                operator: Some(CppOperator::Subtraction),
                return_type: CppType::Class(CppClassType {
                    name: "C1".to_string(),
                    template_arguments: None,
                }),
                arguments: vec![
                    CppFunctionArgument {
                        name: "a".to_string(),
                        argument_type: CppType::Class(CppClassType {
                            name: "C1".to_string(),
                            template_arguments: None,
                        }),
                        has_default_value: false,
                    },
                    CppFunctionArgument {
                        name: "b".to_string(),
                        argument_type: CppType::Class(CppClassType {
                            name: "C1".to_string(),
                            template_arguments: None,
                        }),
                        has_default_value: false,
                    },
                ],
                doc: None,
                allows_variadic_arguments: false,
                template_arguments: None,
                declaration_code: Some("C1 operator - ( C1 a , C1 b )".to_string()),
            }
        );
    }
}

#[test]
fn simple_class_method() {
    let data = run_parser(
        "class MyClass {
    public:
      int func1(int x);
    private:
      int m_x;
    };",
    );
    assert!(data.types.len() == 1);
    assert_eq!(data.types[0].name, "MyClass");
    if let CppTypeDataKind::Class { ref type_base } = data.types[0].kind {
        assert!(type_base.template_arguments.is_none());
    } else {
        panic!("invalid type kind");
    }

    assert!(data.bases.is_empty());
    assert!(data.fields.len() == 1);

    assert!(data.methods.len() == 1);
    assert_eq!(
        data.methods[0],
        CppFunction {
            name: "func1".to_string(),
            member: Some(CppFunctionMemberData {
                class_type: CppClassType {
                    name: "MyClass".to_string(),
                    template_arguments: None,
                },
                kind: CppFunctionKind::Regular,
                is_virtual: false,
                is_pure_virtual: false,
                is_const: false,
                is_static: false,
                visibility: CppVisibility::Public,
                is_signal: false,
                is_slot: false,
            }),
            operator: None,
            return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
            arguments: vec![CppFunctionArgument {
                name: "x".to_string(),
                argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
                has_default_value: false,
            }],
            doc: None,
            allows_variadic_arguments: false,
            template_arguments: None,
            declaration_code: Some("int func1 ( int x )".to_string()),
        }
    );
}

#[cfg_attr(feature = "clippy", allow(cyclomatic_complexity))]
#[test]
fn advanced_class_methods() {
    let data = run_parser(
        "class MyClass {
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
    };",
    );
    assert_eq!(data.methods.len(), 8);
    assert_eq!(data.methods[0].name, "MyClass");
    assert!(data.methods[0]
        .member
        .as_ref()
        .unwrap()
        .kind
        .is_constructor());
    assert_eq!(data.methods[0].arguments.len(), 3);
    assert_eq!(data.methods[0].return_type, CppType::Void);

    assert_eq!(data.methods[1].name, "~MyClass");
    assert!(data.methods[1]
        .member
        .as_ref()
        .unwrap()
        .kind
        .is_destructor());
    assert_eq!(data.methods[1].arguments.len(), 0);
    assert_eq!(data.methods[1].return_type, CppType::Void);

    assert_eq!(data.methods[2].name, "func1");
    assert!(data.methods[2].member.as_ref().unwrap().is_static);

    assert_eq!(data.methods[3].name, "func2");
    assert!(data.methods[3].member.as_ref().unwrap().is_virtual);
    assert!(!data.methods[3].member.as_ref().unwrap().is_pure_virtual);
    assert_eq!(
        data.methods[3].member.as_ref().unwrap().visibility,
        CppVisibility::Public
    );

    assert_eq!(data.methods[4].name, "func3");
    assert!(data.methods[4].member.as_ref().unwrap().is_virtual);
    assert!(data.methods[4].member.as_ref().unwrap().is_pure_virtual);
    assert_eq!(
        data.methods[4].member.as_ref().unwrap().visibility,
        CppVisibility::Protected
    );

    assert_eq!(data.methods[5].name, "func4");
    assert!(data.methods[5].member.as_ref().unwrap().is_const);

    assert_eq!(data.methods[6].name, "operator bool");
    assert!(data.methods[6].member.as_ref().unwrap().is_const);
    assert_eq!(
        data.methods[6].operator,
        Some(CppOperator::Conversion(CppType::BuiltInNumeric(
            CppBuiltInNumericType::Bool
        ),))
    );
    assert_eq!(
        data.methods[6].return_type,
        CppType::BuiltInNumeric(CppBuiltInNumericType::Bool),
    );

    assert_eq!(data.methods[7].name, "func6");
    assert_eq!(
        data.methods[7].template_arguments,
        Some(vec![
            CppType::TemplateParameter {
                nested_level: 0,
                index: 0,
                name: "K".into(),
            },
            CppType::TemplateParameter {
                nested_level: 0,
                index: 1,
                name: "V".into(),
            }
        ])
    );
    assert_eq!(data.methods[7].arguments.len(), 1);
    assert_eq!(
        data.methods[7].arguments[0].argument_type,
        CppType::TemplateParameter {
            nested_level: 0,
            index: 1,
            name: "V".into(),
        },
    );
}

#[test]
fn template_class_method() {
    let data = run_parser(
        "
  template<class T>
  class MyVector {
    public:
      class Iterator {};
      T get(int index);
      Iterator begin();
    };",
    );
    assert_eq!(data.types.len(), 1);
    assert_eq!(data.types[0].name, "MyVector");
    if let CppTypeDataKind::Class { ref type_base } = data.types[0].kind {
        assert_eq!(
            type_base.template_arguments,
            Some(vec![CppType::TemplateParameter {
                nested_level: 0,
                index: 0,
                name: "T".into(),
            }])
        );
    } else {
        panic!("invalid type kind");
    }
    assert!(data.bases.is_empty());
    assert!(data.fields.is_empty());
    assert_eq!(data.methods.len(), 2);
    assert_eq!(
        data.methods[0],
        CppFunction {
            name: "get".to_string(),
            member: Some(CppFunctionMemberData {
                class_type: CppClassType {
                    name: "MyVector".to_string(),
                    template_arguments: Some(vec![CppType::TemplateParameter {
                        nested_level: 0,
                        index: 0,
                        name: "T".into(),
                    }]),
                },
                kind: CppFunctionKind::Regular,
                is_virtual: false,
                is_pure_virtual: false,
                is_const: false,
                is_static: false,
                visibility: CppVisibility::Public,
                is_signal: false,
                is_slot: false,
            }),
            operator: None,
            return_type: CppType::TemplateParameter {
                nested_level: 0,
                index: 0,
                name: "T".into(),
            },
            arguments: vec![CppFunctionArgument {
                name: "index".to_string(),
                argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
                has_default_value: false,
            }],
            doc: None,
            allows_variadic_arguments: false,
            template_arguments: None,
            declaration_code: Some("T get ( int index )".to_string()),
        }
    );
}

#[test]
fn template_class_template_method() {
    let data = run_parser(
        "
  template<class T>
  class MyVector {
    public:
      template<typename F>
      F get_f();

      T get_t();
    };",
    );
    assert_eq!(data.methods[0].name, "get_f");
    assert_eq!(
        data.methods[0].template_arguments,
        Some(vec![CppType::TemplateParameter {
            nested_level: 1,
            index: 0,
            name: "F".into(),
        }])
    );
    assert_eq!(
        data.methods[0].return_type,
        CppType::TemplateParameter {
            nested_level: 1,
            index: 0,
            name: "F".into(),
        },
    );

    assert_eq!(data.methods[1].name, "get_t");
    assert_eq!(data.methods[1].template_arguments, None);
    assert_eq!(
        data.methods[1].return_type,
        CppType::TemplateParameter {
            nested_level: 0,
            index: 0,
            name: "T".into(),
        },
    );
}

#[test]
fn simple_enum() {
    let data = run_parser(
        "
  enum Enum1 {
    Good,
    Bad
  };",
    );
    assert_eq!(data.types.len(), 1);
    assert_eq!(data.types[0].name, "Enum1");
    assert_eq!(data.types[0].kind, CppTypeDataKind::Enum);
    assert_eq!(
        data.enum_values,
        vec![
            CppEnumValue {
                name: "Good".to_string(),
                value: 0,
                doc: None,
                enum_name: "Enum1".to_string(),
            },
            CppEnumValue {
                name: "Bad".to_string(),
                value: 1,
                doc: None,
                enum_name: "Enum1".to_string(),
            },
        ]
    );
}

#[test]
fn simple_enum2() {
    let data = run_parser(
        "
  namespace ns1 {
    enum Enum1 {
      Good = 1,
      Bad = 2,
      Questionable = Good | Bad
    };
  }",
    );
    assert_eq!(data.types.len(), 1);
    assert_eq!(data.types[0].name, "ns1::Enum1");
    assert_eq!(data.types[0].kind, CppTypeDataKind::Enum);
    assert_eq!(
        data.enum_values,
        vec![
            CppEnumValue {
                name: "Good".to_string(),
                value: 1,
                doc: None,
                enum_name: "ns1::Enum1".to_string(),
            },
            CppEnumValue {
                name: "Bad".to_string(),
                value: 2,
                doc: None,
                enum_name: "ns1::Enum1".to_string(),
            },
            CppEnumValue {
                name: "Questionable".to_string(),
                value: 3,
                doc: None,
                enum_name: "ns1::Enum1".to_string(),
            },
        ]
    );
}

#[test]
fn template_instantiation() {
    let data = run_parser(
        "
  template<typename T> class Vector {};
  class C1 {
  public:
    Vector<int> values();
  };
",
    );
    assert_eq!(data.methods.len(), 1);
    let int = CppType::BuiltInNumeric(CppBuiltInNumericType::Int);
    assert_eq!(
        data.methods[0].return_type,
        CppType::Class(CppClassType {
            name: "Vector".to_string(),
            template_arguments: Some(vec![int.clone()]),
        }),
    );
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
    assert_eq!(data.types[1].name, "Derived");
    assert_eq!(
        data.bases,
        vec![CppBaseSpecifier {
            base_class_type: CppClassType {
                name: "Base".to_string(),
                template_arguments: None,
            },
            derived_class_type: CppClassType {
                name: "Derived".to_string(),
                template_arguments: None,
            },
            is_virtual: false,
            visibility: CppVisibility::Public,
            base_index: 0,
        }]
    );
}

#[test]
fn derived_class_simple_private() {
    let data = run_parser("class Base {}; class Derived : Base {};");
    assert!(data.types.len() == 2);
    assert_eq!(data.types[0].name, "Base");
    assert_eq!(data.types[1].name, "Derived");
    assert_eq!(
        data.bases,
        vec![CppBaseSpecifier {
            base_class_type: CppClassType {
                name: "Base".to_string(),
                template_arguments: None,
            },
            derived_class_type: CppClassType {
                name: "Derived".to_string(),
                template_arguments: None,
            },
            is_virtual: false,
            visibility: CppVisibility::Private,
            base_index: 0,
        }]
    );
}

#[test]
fn derived_class_simple_virtual() {
    let data = run_parser("class Base {}; class Derived : public virtual Base {};");
    assert!(data.types.len() == 2);
    assert_eq!(data.types[0].name, "Base");
    assert_eq!(data.types[1].name, "Derived");
    assert_eq!(
        data.bases,
        vec![CppBaseSpecifier {
            base_class_type: CppClassType {
                name: "Base".to_string(),
                template_arguments: None,
            },
            derived_class_type: CppClassType {
                name: "Derived".to_string(),
                template_arguments: None,
            },
            is_virtual: true,
            visibility: CppVisibility::Public,
            base_index: 0,
        }]
    );
}

#[test]
fn derived_class_multiple() {
    let data = run_parser(
        "
    class Base1 {}; class Base2 {};
    class Derived : public Base2, public Base1 {};",
    );
    assert!(data.types.len() == 3);
    assert_eq!(data.types[0].name, "Base1");
    assert_eq!(data.types[1].name, "Base2");
    assert_eq!(data.types[2].name, "Derived");
    assert_eq!(
        data.bases,
        vec![
            CppBaseSpecifier {
                base_class_type: CppClassType {
                    name: "Base2".to_string(),
                    template_arguments: None,
                },
                derived_class_type: CppClassType {
                    name: "Derived".to_string(),
                    template_arguments: None,
                },
                is_virtual: false,
                visibility: CppVisibility::Public,
                base_index: 0,
            },
            CppBaseSpecifier {
                base_class_type: CppClassType {
                    name: "Base1".to_string(),
                    template_arguments: None,
                },
                derived_class_type: CppClassType {
                    name: "Derived".to_string(),
                    template_arguments: None,
                },
                is_virtual: false,
                visibility: CppVisibility::Public,
                base_index: 1,
            },
        ]
    );
}

#[test]
fn complex_const_types() {
    let data = run_parser(
        "
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
  ",
    );
    let base = CppType::BuiltInNumeric(CppBuiltInNumericType::Int);
    assert_eq!(data.methods.len(), 10);
    assert_eq!(&data.methods[0].return_type, &base);
    assert_eq!(&data.methods[1].return_type, &base);
    assert_eq!(
        &data.methods[2].return_type,
        &CppType::new_pointer(false, base.clone())
    );
    assert_eq!(
        &data.methods[3].return_type,
        &CppType::new_pointer(true, base.clone())
    );
    assert_eq!(
        &data.methods[4].return_type,
        &CppType::new_pointer(false, base.clone())
    );
    assert_eq!(
        &data.methods[5].return_type,
        &CppType::new_pointer(false, CppType::new_pointer(false, base.clone()))
    );
    assert_eq!(
        &data.methods[6].return_type,
        &CppType::new_pointer(true, CppType::new_pointer(false, base.clone()))
    );
    assert_eq!(
        &data.methods[7].return_type,
        &CppType::new_pointer(true, CppType::new_pointer(true, base.clone()))
    );
    assert_eq!(
        &data.methods[8].return_type,
        &CppType::new_pointer(true, CppType::new_pointer(true, base.clone()))
    );
    assert_eq!(
        &data.methods[9].return_type,
        &CppType::new_pointer(true, CppType::new_pointer(true, base.clone()))
    );
}

#[test]
fn anon_enum() {
    let data = run_parser(
        "class X {
    enum { v1, v2 } field;
  };",
    );
    assert!(data.types.len() == 1);
    assert_eq!(data.types[0].name, "X");
    assert!(data.fields.is_empty());
}

#[test]
fn non_type_template_parameter() {
    let data = run_parser(
        "\
  template<int> struct QAtomicOpsSupport { enum { IsSupported = 0 }; };
  template<> struct QAtomicOpsSupport<4> { enum { IsSupported = 1 }; };",
    );
    assert!(data.types.is_empty());
}

#[test]
fn fixed_size_integers() {
    let data = run_parser(
        "
  typedef unsigned long long int GLuint64;
  template<typename T> class QVector {};
  GLuint64 f1();
  QVector<GLuint64> f2();
  ",
    );
    assert_eq!(data.methods.len(), 2);
    assert_eq!(&data.methods[0].name, "f1");
    let type1 = CppType::SpecificNumeric(CppSpecificNumericType {
        name: "GLuint64".to_string(),
        bits: 64,
        kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
    });
    assert_eq!(&data.methods[0].return_type, &type1);

    assert_eq!(&data.methods[1].name, "f2");
    assert_eq!(
        &data.methods[1].return_type,
        &CppType::Class(CppClassType {
            name: "QVector".to_string(),
            template_arguments: Some(vec![type1.clone()]),
        }),
    );
}

#[test]
fn template_class_with_base() {
    let data = run_parser(
        "
  template<class T>
  class C1 {};

  template<class T>
  class C2: public C1<T> {};
  ",
    );
    assert!(data.types.len() == 2);
    assert_eq!(data.types[0].name, "C1");
    if let CppTypeDataKind::Class { ref type_base } = data.types[0].kind {
        assert_eq!(
            &type_base.template_arguments,
            &Some(vec![CppType::TemplateParameter {
                nested_level: 0,
                index: 0,
                name: "T".into(),
            }])
        );
    } else {
        panic!("invalid type kind");
    }

    assert_eq!(data.types[1].name, "C2");
    if let CppTypeDataKind::Class { ref type_base } = data.types[1].kind {
        assert_eq!(
            &type_base.template_arguments,
            &Some(vec![CppType::TemplateParameter {
                nested_level: 0,
                index: 0,
                name: "T".into(),
            }])
        );
    } else {
        panic!("invalid type kind");
    }
    assert_eq!(data.bases.len(), 1);
    assert!(data.fields.is_empty());
}
