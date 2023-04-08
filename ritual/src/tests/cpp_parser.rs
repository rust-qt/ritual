use crate::config::{Config, CrateProperties};
use crate::cpp_data::*;
use crate::cpp_function::*;
use crate::cpp_operator::CppOperator;
use crate::cpp_type::*;
use crate::processor;
use crate::workspace::Workspace;
use ritual_common::cpp_build_config::CppBuildPaths;
use ritual_common::file_utils::create_dir;
use ritual_common::file_utils::create_file;
use std::io::Write;

struct ParserCppData {
    types: Vec<CppTypeDeclaration>,
    bases: Vec<CppBaseSpecifier>,
    fields: Vec<CppClassField>,
    methods: Vec<CppFunction>,
    enum_values: Vec<CppEnumValue>,
    namespaces: Vec<CppPath>,
}

fn run_parser(code: &'static str) -> ParserCppData {
    let dir = tempdir::TempDir::new("test_cpp_parser_run").unwrap();

    let mut workspace = Workspace::new(dir.path().into()).unwrap();

    let include_dir = dir.path().join("include");
    create_dir(&include_dir).unwrap();
    let include_name = "myfakelib.h";
    let include_file_path = include_dir.join(include_name);
    {
        let mut include_file = create_file(&include_file_path).unwrap();
        writeln!(include_file, "{}", code).unwrap();
    }

    let mut paths = CppBuildPaths::new();
    paths.add_include_path(include_dir);

    let mut config = Config::new(CrateProperties::new("A", "0.0.0"));
    config.add_include_directive(include_name);
    config.set_cpp_build_paths(paths);
    config.add_target_include_path(include_file_path);

    processor::process(&mut workspace, &config, &["cpp_parser".into()], None).unwrap();

    let database = workspace
        .get_database_client("A", &[], true, false)
        .unwrap();

    ParserCppData {
        types: database
            .cpp_items()
            .filter_map(|item| item.item.as_type_ref())
            .cloned()
            .collect(),
        bases: database
            .cpp_items()
            .filter_map(|item| item.item.as_base_ref())
            .cloned()
            .collect(),
        fields: database
            .cpp_items()
            .filter_map(|item| item.item.as_field_ref())
            .cloned()
            .collect(),
        enum_values: database
            .cpp_items()
            .filter_map(|item| item.item.as_enum_value_ref())
            .cloned()
            .collect(),
        methods: database
            .cpp_items()
            .filter_map(|item| item.item.as_function_ref())
            .cloned()
            .collect(),
        namespaces: database
            .cpp_items()
            .filter_map(|item| item.item.as_namespace_ref())
            .map(|ns| ns.path.clone())
            .collect(),
    }
}

#[test]
fn simple_func() {
    let data = run_parser("int func1(int x);");
    assert!(data.types.is_empty());
    assert_eq!(data.methods.len(), 1);
    assert_eq!(
        data.methods[0],
        CppFunction {
            path: CppPath::from_good_str("func1"),
            member: None,
            operator: None,
            return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
            arguments: vec![CppFunctionArgument {
                name: "x".to_string(),
                argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
                has_default_value: false,
            }],
            allows_variadic_arguments: false,
            cast: None,
            declaration_code: Some("int func1 ( int x )".to_string()),
        }
    );
}

#[test]
fn simple_func_with_default_value() {
    let data = run_parser(
        "
        bool func1(int x = 42) {
            return false;
        }
        ",
    );
    assert!(data.types.is_empty());
    assert_eq!(data.methods.len(), 1);
    assert_eq!(
        data.methods[0],
        CppFunction {
            path: CppPath::from_good_str("func1"),
            member: None,
            operator: None,
            return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Bool),
            arguments: vec![CppFunctionArgument {
                name: "x".to_string(),
                argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
                has_default_value: true,
            }],
            allows_variadic_arguments: false,
            cast: None,
            declaration_code: Some("bool func1 ( int x = 42 )".to_string()),
        }
    );
}

#[test]
fn functions_with_class_arg() {
    let data = run_parser(
        "
        class Magic {
        public:
            int a, b;
            static int c;
        };
        bool func1(Magic x);
        bool func1(Magic* x);
        bool func2(const Magic&);
        ",
    );
    assert_eq!(data.types.len(), 1);
    assert_eq!(data.types[0].path, CppPath::from_good_str("Magic"));
    assert!(data.types[0].kind.is_class());

    assert!(data.bases.is_empty());

    assert_eq!(data.fields.len(), 3);
    assert_eq!(data.fields[0].path, CppPath::from_good_str("Magic::a"));
    assert_eq!(
        data.fields[0].field_type,
        CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
    );
    assert_eq!(data.fields[0].visibility, CppVisibility::Public);
    assert!(!data.fields[0].is_static);

    assert_eq!(data.fields[1].path, CppPath::from_good_str("Magic::b"));
    assert_eq!(
        data.fields[1].field_type,
        CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
    );
    assert_eq!(data.fields[1].visibility, CppVisibility::Public);
    assert!(!data.fields[1].is_static);

    assert_eq!(data.fields[2].path, CppPath::from_good_str("Magic::c"));
    assert_eq!(
        data.fields[2].field_type,
        CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
    );
    assert_eq!(data.fields[2].visibility, CppVisibility::Public);
    assert!(data.fields[2].is_static);

    assert_eq!(data.methods.len(), 3);
    assert_eq!(
        data.methods[0],
        CppFunction {
            path: CppPath::from_good_str("func1"),
            member: None,
            operator: None,
            return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Bool),
            arguments: vec![CppFunctionArgument {
                name: "x".to_string(),
                argument_type: CppType::Class(CppPath::from_good_str("Magic")),
                has_default_value: false,
            }],
            allows_variadic_arguments: false,
            cast: None,
            declaration_code: Some("bool func1 ( Magic x )".to_string()),
        }
    );
    assert_eq!(
        data.methods[1],
        CppFunction {
            path: CppPath::from_good_str("func1"),
            member: None,
            operator: None,
            return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Bool),
            arguments: vec![CppFunctionArgument {
                name: "x".to_string(),
                argument_type: CppType::new_pointer(
                    false,
                    CppType::Class(CppPath::from_good_str("Magic"))
                ),
                has_default_value: false,
            }],
            allows_variadic_arguments: false,
            cast: None,
            declaration_code: Some("bool func1 ( Magic * x )".to_string()),
        }
    );
    assert_eq!(
        data.methods[2],
        CppFunction {
            path: CppPath::from_good_str("func2"),
            member: None,
            operator: None,
            return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Bool),
            arguments: vec![CppFunctionArgument {
                name: "arg1".to_string(),
                argument_type: CppType::new_reference(
                    true,
                    CppType::Class(CppPath::from_good_str("Magic"))
                ),
                has_default_value: false,
            }],
            allows_variadic_arguments: false,
            cast: None,
            declaration_code: Some("bool func2 ( const Magic & )".to_string()),
        }
    );
}

#[test]
fn free_operator() {
    let data = run_parser(
        "
        class QPoint {
            friend QPoint operator-(const QPoint& one, const QPoint& other);
        };
        QPoint operator-(const QPoint& one, const QPoint& other);
        ",
    );

    assert_eq!(data.methods.len(), 1);
}

#[test]
fn func_with_unknown_type() {
    let data = run_parser(
        "
        class SomeClass;
        int func1(SomeClass* x);
        ",
    );
    assert!(data.types.is_empty());
    assert_eq!(data.methods.len(), 1);
}

#[test]
fn variadic_func() {
    let data = run_parser("int my_printf ( const char * format, ... );");
    assert_eq!(data.methods.len(), 1);
    assert!(data.types.is_empty());
    assert_eq!(
        data.methods[0],
        CppFunction {
            path: CppPath::from_good_str("my_printf"),
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
            allows_variadic_arguments: true,
            cast: None,
            declaration_code: Some("int my_printf ( const char * format , ... )".to_string()),
        }
    );
}

#[test]
fn free_template_func() {
    let data = run_parser(
        "
        template<typename T>
        T abs(T value) {
            return 2 * value;
        }
        ",
    );
    assert!(data.types.is_empty());
    assert_eq!(data.methods.len(), 1);
    let abs_item = CppPathItem {
        name: "abs".into(),
        template_arguments: Some(vec![CppType::TemplateParameter(CppTemplateParameter {
            nested_level: 0,
            index: 0,
            name: "T".into(),
        })]),
    };
    assert_eq!(
        data.methods[0],
        CppFunction {
            path: CppPath::from_item(abs_item),
            member: None,
            operator: None,
            return_type: CppType::TemplateParameter(CppTemplateParameter {
                nested_level: 0,
                index: 0,
                name: "T".into(),
            }),
            arguments: vec![CppFunctionArgument {
                name: "value".to_string(),
                argument_type: CppType::TemplateParameter(CppTemplateParameter {
                    nested_level: 0,
                    index: 0,
                    name: "T".into(),
                }),
                has_default_value: false,
            }],
            allows_variadic_arguments: false,
            cast: None,
            declaration_code: Some("template < typename T > T abs ( T value )".to_string()),
        }
    );
}

#[test]
fn free_func_operator_sub() {
    for code in &[
        "
        class C1 {};
        C1 operator-(C1 a, C1 b);
        ",
        "\
         class C1 {};\
         C1 operator -(C1 a, C1 b);\
         ",
    ] {
        let data = run_parser(code);
        assert_eq!(data.types.len(), 1);
        assert_eq!(data.methods.len(), 1);
        assert_eq!(
            data.methods[0],
            CppFunction {
                path: CppPath::from_good_str("operator-"),
                member: None,
                operator: Some(CppOperator::Subtraction),
                return_type: CppType::Class(CppPath::from_good_str("C1")),
                arguments: vec![
                    CppFunctionArgument {
                        name: "a".to_string(),
                        argument_type: CppType::Class(CppPath::from_good_str("C1")),
                        has_default_value: false,
                    },
                    CppFunctionArgument {
                        name: "b".to_string(),
                        argument_type: CppType::Class(CppPath::from_good_str("C1")),
                        has_default_value: false,
                    },
                ],
                allows_variadic_arguments: false,
                cast: None,
                declaration_code: Some("C1 operator - ( C1 a , C1 b )".to_string()),
            }
        );
    }
}

#[test]
fn simple_class_method() {
    let data = run_parser(
        "
        class MyClass {
        public:
            int func1(int x);
        private:
            int m_x;
        };
        ",
    );
    assert_eq!(data.types.len(), 1);
    assert_eq!(data.types[0].path, CppPath::from_good_str("MyClass"));
    assert!(data.types[0].kind.is_class());

    assert!(data.bases.is_empty());
    assert_eq!(data.fields.len(), 1);

    assert_eq!(data.methods.len(), 1);
    assert_eq!(
        data.methods[0],
        CppFunction {
            path: CppPath::from_good_str("MyClass::func1"),
            member: Some(CppFunctionMemberData {
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
            allows_variadic_arguments: false,
            cast: None,
            declaration_code: Some("int func1 ( int x )".to_string()),
        }
    );
}

#[test]
fn advanced_class_methods() {
    let data = run_parser(
        "
        class MyClass {
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
        };
        ",
    );
    assert_eq!(data.methods.len(), 8);
    assert_eq!(
        data.methods[0].path,
        CppPath::from_good_str("MyClass::MyClass")
    );
    assert!(data.methods[0]
        .member
        .as_ref()
        .unwrap()
        .kind
        .is_constructor());
    assert_eq!(data.methods[0].arguments.len(), 3);
    assert_eq!(data.methods[0].return_type, CppType::Void);

    assert_eq!(
        data.methods[1].path,
        CppPath::from_good_str("MyClass::~MyClass")
    );
    assert!(data.methods[1]
        .member
        .as_ref()
        .unwrap()
        .kind
        .is_destructor());
    assert_eq!(data.methods[1].arguments.len(), 0);
    assert_eq!(data.methods[1].return_type, CppType::Void);

    assert_eq!(
        data.methods[2].path,
        CppPath::from_good_str("MyClass::func1")
    );
    assert!(data.methods[2].member.as_ref().unwrap().is_static);

    assert_eq!(
        data.methods[3].path,
        CppPath::from_good_str("MyClass::func2")
    );
    assert!(data.methods[3].member.as_ref().unwrap().is_virtual);
    assert!(!data.methods[3].member.as_ref().unwrap().is_pure_virtual);
    assert_eq!(
        data.methods[3].member.as_ref().unwrap().visibility,
        CppVisibility::Public
    );

    assert_eq!(
        data.methods[4].path,
        CppPath::from_good_str("MyClass::func3")
    );
    assert!(data.methods[4].member.as_ref().unwrap().is_virtual);
    assert!(data.methods[4].member.as_ref().unwrap().is_pure_virtual);
    assert_eq!(
        data.methods[4].member.as_ref().unwrap().visibility,
        CppVisibility::Protected
    );

    assert_eq!(
        data.methods[5].path,
        CppPath::from_good_str("MyClass::func4")
    );
    assert!(data.methods[5].member.as_ref().unwrap().is_const);

    assert_eq!(
        data.methods[6].path,
        CppPath::from_good_str("MyClass::operator bool")
    );
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

    let func6_item = CppPathItem {
        name: "func6".to_string(),
        template_arguments: Some(vec![
            CppType::TemplateParameter(CppTemplateParameter {
                nested_level: 0,
                index: 0,
                name: "K".into(),
            }),
            CppType::TemplateParameter(CppTemplateParameter {
                nested_level: 0,
                index: 1,
                name: "V".into(),
            }),
        ]),
    };
    assert_eq!(
        data.methods[7].path,
        CppPath::from_items(vec![CppPathItem::from_good_str("MyClass"), func6_item])
    );
    assert_eq!(data.methods[7].arguments.len(), 1);
    assert_eq!(
        data.methods[7].arguments[0].argument_type,
        CppType::TemplateParameter(CppTemplateParameter {
            nested_level: 0,
            index: 1,
            name: "V".into(),
        }),
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
        };
        ",
    );
    assert_eq!(data.types.len(), 2);
    let my_vector_item = CppPathItem {
        name: "MyVector".into(),
        template_arguments: Some(vec![CppType::TemplateParameter(CppTemplateParameter {
            nested_level: 0,
            index: 0,
            name: "T".into(),
        })]),
    };
    let my_vector_path = CppPath::from_item(my_vector_item.clone());
    assert_eq!(data.types[0].path, my_vector_path);
    assert!(data.types[0].kind.is_class());

    assert_eq!(
        data.types[1].path,
        my_vector_path.join(CppPathItem::from_good_str("Iterator"))
    );
    assert!(data.types[1].kind.is_class());

    assert!(data.bases.is_empty());
    assert!(data.fields.is_empty());
    assert_eq!(data.methods.len(), 2);
    assert_eq!(
        data.methods[0],
        CppFunction {
            path: my_vector_path.join(CppPathItem::from_good_str("get")),
            member: Some(CppFunctionMemberData {
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
            return_type: CppType::TemplateParameter(CppTemplateParameter {
                nested_level: 0,
                index: 0,
                name: "T".into(),
            }),
            arguments: vec![CppFunctionArgument {
                name: "index".to_string(),
                argument_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Int),
                has_default_value: false,
            }],
            allows_variadic_arguments: false,
            cast: None,
            declaration_code: Some("T get ( int index )".to_string()),
        }
    );
    assert_eq!(
        data.methods[1].path,
        my_vector_path.join(CppPathItem::from_good_str("begin"))
    );
    assert_eq!(
        data.methods[1].return_type,
        CppType::Class(CppPath::from_items(vec![
            my_vector_item,
            CppPathItem::from_good_str("Iterator")
        ]))
    );
    assert!(data.namespaces.is_empty());
}

#[test]
fn template_parameter_level() {
    let data = run_parser(
        "
        template<class U>
        class X {
            template<class T>
            class MyVector {
            public:
                T get(int index);
            };
        };
        ",
    );
    assert_eq!(
        data.methods[0].return_type,
        CppType::TemplateParameter(CppTemplateParameter {
            nested_level: 1,
            index: 0,
            name: "T".into(),
        })
    );
}

#[test]
fn template_parameter_level2() {
    let data = run_parser(
        "
        template<class U>
        class X {
            class Y {
                template<class T>
                class MyVector {
                public:
                    T get(int index);
                };
            };
        };
        ",
    );
    assert_eq!(
        data.methods[0].return_type,
        CppType::TemplateParameter(CppTemplateParameter {
            nested_level: 1,
            index: 0,
            name: "T".into(),
        })
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
        };
        ",
    );
    let vector_item = CppPathItem {
        name: "MyVector".to_string(),
        template_arguments: Some(vec![CppType::TemplateParameter(CppTemplateParameter {
            nested_level: 0,
            index: 0,
            name: "T".into(),
        })]),
    };
    assert_eq!(
        data.methods[0].path,
        CppPath::from_items(vec![
            vector_item.clone(),
            CppPathItem {
                name: "get_f".into(),
                template_arguments: Some(vec![CppType::TemplateParameter(CppTemplateParameter {
                    nested_level: 1,
                    index: 0,
                    name: "F".into(),
                })])
            }
        ])
    );
    assert_eq!(
        data.methods[0].return_type,
        CppType::TemplateParameter(CppTemplateParameter {
            nested_level: 1,
            index: 0,
            name: "F".into(),
        }),
    );

    assert_eq!(
        data.methods[1].path,
        CppPath::from_items(vec![vector_item, CppPathItem::from_good_str("get_t")])
    );
    assert_eq!(
        data.methods[1].return_type,
        CppType::TemplateParameter(CppTemplateParameter {
            nested_level: 0,
            index: 0,
            name: "T".into(),
        }),
    );
}

#[test]
fn simple_enum() {
    let data = run_parser(
        "
        enum Enum1 {
            Good,
            Bad
        };
        ",
    );
    assert_eq!(data.types.len(), 1);
    assert_eq!(data.types[0].path, CppPath::from_good_str("Enum1"));
    assert_eq!(data.types[0].kind, CppTypeDeclarationKind::Enum);
    assert_eq!(
        data.enum_values,
        vec![
            CppEnumValue {
                value: 0,
                path: CppPath::from_good_str("Enum1::Good"),
            },
            CppEnumValue {
                value: 1,
                path: CppPath::from_good_str("Enum1::Bad"),
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
        }
        ",
    );
    assert_eq!(data.types.len(), 1);
    assert_eq!(data.types[0].path, CppPath::from_good_str("ns1::Enum1"));
    assert_eq!(data.types[0].kind, CppTypeDeclarationKind::Enum);
    assert_eq!(
        data.enum_values,
        vec![
            CppEnumValue {
                value: 1,
                path: CppPath::from_good_str("ns1::Enum1::Good"),
            },
            CppEnumValue {
                value: 2,
                path: CppPath::from_good_str("ns1::Enum1::Bad"),
            },
            CppEnumValue {
                value: 3,
                path: CppPath::from_good_str("ns1::Enum1::Questionable"),
            },
        ]
    );
    assert_eq!(data.namespaces, vec![CppPath::from_good_str("ns1")]);
}

#[test]
fn template_instantiation() {
    let data = run_parser(
        "
        template<typename T>
        class Vector {};
        class C1 {
            public:
            Vector<int> values();
        };
        ",
    );
    assert_eq!(data.methods.len(), 1);
    let int = CppType::BuiltInNumeric(CppBuiltInNumericType::Int);
    let vector_int_item = CppPathItem {
        name: "Vector".to_string(),
        template_arguments: Some(vec![int]),
    };
    assert_eq!(
        data.methods[0].return_type,
        CppType::Class(CppPath::from_item(vector_int_item)),
    );
}

#[test]
fn derived_class_simple() {
    let data = run_parser("class Base {}; class Derived : public Base {};");
    assert_eq!(data.types.len(), 2);
    assert_eq!(data.types[0].path, CppPath::from_good_str("Base"));
    assert_eq!(data.types[1].path, CppPath::from_good_str("Derived"));
    assert_eq!(
        data.bases,
        vec![CppBaseSpecifier {
            base_class_type: CppPath::from_good_str("Base"),
            derived_class_type: CppPath::from_good_str("Derived"),
            is_virtual: false,
            visibility: CppVisibility::Public,
            base_index: 0,
        }]
    );
}

#[test]
fn derived_class_simple_private() {
    let data = run_parser("class Base {}; class Derived : Base {};");
    assert_eq!(data.types.len(), 2);
    assert_eq!(data.types[0].path, CppPath::from_good_str("Base"));
    assert_eq!(data.types[1].path, CppPath::from_good_str("Derived"));
    assert_eq!(
        data.bases,
        vec![CppBaseSpecifier {
            base_class_type: CppPath::from_good_str("Base"),
            derived_class_type: CppPath::from_good_str("Derived"),
            is_virtual: false,
            visibility: CppVisibility::Private,
            base_index: 0,
        }]
    );
}

#[test]
fn derived_class_simple_virtual() {
    let data = run_parser("class Base {}; class Derived : public virtual Base {};");
    assert_eq!(data.types.len(), 2);
    assert_eq!(data.types[0].path, CppPath::from_good_str("Base"));
    assert_eq!(data.types[1].path, CppPath::from_good_str("Derived"));
    assert_eq!(
        data.bases,
        vec![CppBaseSpecifier {
            base_class_type: CppPath::from_good_str("Base"),
            derived_class_type: CppPath::from_good_str("Derived"),
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
        class Base1 {};
        class Base2 {};
        class Derived : public Base2, public Base1 {};
        ",
    );
    assert_eq!(data.types.len(), 3);
    assert_eq!(data.types[0].path, CppPath::from_good_str("Base1"));
    assert_eq!(data.types[1].path, CppPath::from_good_str("Base2"));
    assert_eq!(data.types[2].path, CppPath::from_good_str("Derived"));
    assert_eq!(
        data.bases,
        vec![
            CppBaseSpecifier {
                base_class_type: CppPath::from_good_str("Base2"),
                derived_class_type: CppPath::from_good_str("Derived"),
                is_virtual: false,
                visibility: CppVisibility::Public,
                base_index: 0,
            },
            CppBaseSpecifier {
                base_class_type: CppPath::from_good_str("Base1"),
                derived_class_type: CppPath::from_good_str("Derived"),
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
        &CppType::new_pointer(true, CppType::new_pointer(true, base))
    );
}

#[test]
fn anon_enum() {
    let data = run_parser(
        "
        class X {
            enum { v1, v2 } field;
        };
        ",
    );
    assert_eq!(data.types.len(), 1);
    assert_eq!(data.types[0].path, CppPath::from_good_str("X"));
    assert!(data.fields.is_empty());
}

#[test]
fn non_type_template_parameter() {
    let data = run_parser(
        "
        template<int>
        struct QAtomicOpsSupport {
            enum { IsSupported = 0 };
        };
        template<>
        struct QAtomicOpsSupport<4> {
            enum { IsSupported = 1 };
        };
        ",
    );
    assert!(data.types.is_empty());
}

#[test]
fn template_specialization() {
    let data = run_parser(
        "
        template<class T>
        class QVector {};

        template <typename T>
        class QFutureInterface {
            void reportResults(const QVector<T> &value);
        };

        template <>
        class QFutureInterface<void> {
            void reportResults(const QVector<void> &value);
        };
        ",
    );
    assert_eq!(data.types.len(), 2);
    println!("{:?}", data.methods);
    assert_eq!(data.methods.len(), 1);
}

#[test]
fn fixed_size_integers() {
    let data = run_parser(
        "
        typedef unsigned long long int GLuint64;
        template<typename T>
        class QVector {};
        GLuint64 f1();
        QVector<GLuint64> f2();
        ",
    );
    assert_eq!(data.methods.len(), 2);
    assert_eq!(&data.methods[0].path, &CppPath::from_good_str("f1"));
    let type1 = CppType::SpecificNumeric(CppSpecificNumericType {
        path: CppPath::from_good_str("GLuint64"),
        bits: 64,
        kind: CppSpecificNumericTypeKind::Integer { is_signed: false },
    });
    assert_eq!(&data.methods[0].return_type, &type1);

    assert_eq!(&data.methods[1].path, &CppPath::from_good_str("f2"));

    let vector_gluint64_item = CppPathItem {
        name: "QVector".to_string(),
        template_arguments: Some(vec![type1]),
    };
    assert_eq!(
        &data.methods[1].return_type,
        &CppType::Class(CppPath::from_item(vector_gluint64_item)),
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
    assert_eq!(data.types.len(), 2);
    let c1_item = CppPathItem {
        name: "C1".to_string(),
        template_arguments: Some(vec![CppType::TemplateParameter(CppTemplateParameter {
            nested_level: 0,
            index: 0,
            name: "T".into(),
        })]),
    };
    assert_eq!(data.types[0].path, CppPath::from_item(c1_item));
    assert!(data.types[0].kind.is_class());

    let c2_item = CppPathItem {
        name: "C2".to_string(),
        template_arguments: Some(vec![CppType::TemplateParameter(CppTemplateParameter {
            nested_level: 0,
            index: 0,
            name: "T".into(),
        })]),
    };
    assert_eq!(data.types[1].path, CppPath::from_item(c2_item));
    assert!(data.types[1].kind.is_class());
    assert_eq!(data.bases.len(), 1);
    assert!(data.fields.is_empty());
}

#[test]
fn namespaces() {
    let data = run_parser(
        "
        namespace a {
            class X {};
            namespace b {
                class Y {};
            }
            namespace c {
                void z() {}
            }
        }
        namespace a {
            class Z {};
        }
        namespace a::b::c {
            void x() {}
        }
        ",
    );
    assert_eq!(data.namespaces.len(), 4);
    assert!(data.namespaces.contains(&CppPath::from_good_str("a")));
    assert!(data.namespaces.contains(&CppPath::from_good_str("a::b")));
    assert!(data.namespaces.contains(&CppPath::from_good_str("a::b::c")));
    assert!(data.namespaces.contains(&CppPath::from_good_str("a::c")));
}

#[test]
fn empty_namespace() {
    let data = run_parser(
        "
        namespace a {
            namespace b {
                class Y {};
            }
        }
        ",
    );
    assert_eq!(data.namespaces.len(), 2);
    assert!(data.namespaces.contains(&CppPath::from_good_str("a")));
    assert!(data.namespaces.contains(&CppPath::from_good_str("a::b")));
}

#[test]
fn function_pointer() {
    let data = run_parser("void set(void (*func)(void*), void* data);");
    assert_eq!(data.methods.len(), 1);
    let function = &data.methods[0];
    assert_eq!(function.arguments.len(), 2);

    let arg0 = &function.arguments[0];
    assert_eq!(arg0.name, "func");
    assert_eq!(
        arg0.argument_type,
        CppType::FunctionPointer(CppFunctionPointerType {
            arguments: vec![CppType::new_pointer(false, CppType::Void)],
            return_type: Box::new(CppType::Void),
            allows_variadic_arguments: false,
        })
    );
    assert!(!arg0.has_default_value);

    let arg1 = &function.arguments[1];
    assert_eq!(arg1.name, "data");
    assert_eq!(
        arg1.argument_type,
        CppType::new_pointer(false, CppType::Void)
    );
    assert!(!arg1.has_default_value);
}

#[test]
fn template_conversion_operator() {
    let data = run_parser(
        "
        template<typename T>
        class Vec {};

        class A {
        public:
            operator Vec<int>() {}
        };
        ",
    );
    assert_eq!(data.methods.len(), 1);

    let vec_item = CppPathItem {
        name: "Vec".to_string(),
        template_arguments: Some(vec![CppType::BuiltInNumeric(CppBuiltInNumericType::Int)]),
    };
    let vec_type = CppType::Class(CppPath::from_item(vec_item));

    let func = &data.methods[0];
    assert_eq!(
        func.operator,
        Some(CppOperator::Conversion(vec_type.clone()))
    );
    assert_eq!(func.return_type, vec_type);
}

#[test]
fn elaborated_enum_type() {
    let data = run_parser(
        "
        class A {
        public:
            enum E { E1, E2, };
        };
        A::E fn1();
        ",
    );

    assert_eq!(data.methods.len(), 1);
    let func = &data.methods[0];
    assert_eq!(
        func.return_type,
        CppType::Enum {
            path: CppPath::from_good_str("A::E")
        }
    );
}
