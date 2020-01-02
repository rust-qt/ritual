use ritual::config::Config;
use ritual::cpp_checker::{PreliminaryTest, Snippet};
use ritual::cpp_data::{CppItem, CppPath, CppPathItem};
use ritual::cpp_ffi_data::CppFfiFunctionKind;
use ritual::cpp_function::{CppFunction, CppFunctionArgument};
use ritual::cpp_parser::CppParserOutput;
use ritual::cpp_template_instantiator::instantiate_function;
use ritual::cpp_type::{CppBuiltInNumericType, CppType};
use ritual::processor::ProcessorData;
use ritual::rust_info::{NameType, RustItem, RustPathScope};
use ritual::rust_type::{RustFinalType, RustPath, RustToFfiTypeConversion};
use ritual_common::errors::{bail, err_msg, Result};
use ritual_common::string_utils::CaseOperations;

/// QtCore specific configuration.
pub fn core_config(config: &mut Config) -> Result<()> {
    let crate_name = config.crate_properties().name().to_string();
    let crate_name2 = crate_name.clone();
    let namespace = CppPath::from_good_str("Qt");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str(&crate_name),
                prefix: None,
            }));
        }
        Ok(None)
    });

    config.set_cpp_parser_path_hook(|path| {
        let string = path.to_templateless_string();
        let blocked = &[
            // Qt internals, not intended for direct use
            "QtPrivate",
            "QAlgorithmsPrivate",
            "QtMetaTypePrivate",
            "QInternal",
            "qFlagLocation",
            "QArrayData",
            "QTypedArrayData",
            "QStaticByteArrayData",
            "QListData",
            "QObjectData",
            "QObjectUserData",
            "QObject::registerUserData",
            "QMapNodeBase",
            "QMapNode",
            "QMapDataBase",
            "QMapData",
            "QHashData",
            "QHashDummyValue",
            "QHashNode",
            "QContiguousCacheData",
            "QContiguousCacheTypedData",
            "QLinkedListData",
            "QLinkedListNode",
            "QThreadStorageData",
            "QVariant::Private",
            "QVariant::PrivateShared",
            "QByteArrayDataPtr",
            "QStringDataPtr",
            "QArrayDataPointer",
            "QArrayDataPointerRef",
            "QMetaTypeId",
            "QMetaTypeId2",
            "QVariantComparisonHelper",
            "QtStringBuilder",
            "QStringBuilder",
            "QStringBuilderCommon",
            "QStringBuilderBase",
            "QConcatenable",
            "QVariant::Handler",
            "QForeachContainer",
            "QPersistentModelIndex::internalId",
            "QPersistentModelIndex::internalPointer",
            "qMapLessThanKey",
            "qt_hash",
            "qt_qFindChild_helper",
            "qt_qFindChildren_helper",
            "qt_sharedpointer_cast_check",
            "qThreadStorage_deleteData",
            "qbswap_helper",
            // deprecated
            "qGreater",
            "qLess",
            "QString::Null",
            // undocumented, does nothing
            "qt_noop",
            "QNoDebug",
            // undocumented, unknown purpose
            "qTerminate",
            "qt_error_string",
            "QFutureInterfaceBase",
            "QFutureInterfaceBase",
            "Qt::Initialization",
            "QAbstractConcatenable",
            "QTextCodec::ConverterState",
            "QJsonValuePtr",
            "QJsonValueRefPtr",
            "QTypeInfo",
            "QTypeInfoQuery",
            "QTypeInfoMerger",
            "QtGlobalStatic",
            "_GUID",
            // for Q_ASSERT, Q_ASSERT_X macros, no need to access this from Rust
            "qt_assert",
            "qt_assert_x",
            // for Q_CHECK_PTR macro, no need to access this from Rust
            "qt_check_pointer",
            "q_check_ptr",
            // atomic operations, useless in Rust
            "QGenericAtomicOps",
            "QAtomicTraits",
            "QAtomicOps",
            "QBasicAtomicInteger",
            "QBasicAtomicPointer",
            "qAtomicAssign",
            "qAtomicDetach",
            "QAtomicAdditiveType",
            "QAtomicInt",
            "QAtomicPointer",
            "QAtomicInteger",
            // BE/LE integers, useless in Rust
            "QSpecialInteger",
            "QBigEndianStorageType",
            "QLittleEndianStorageType",
            // works on overloading, can't be useful in Rust
            "Qt::qt_getEnumName",
            "Qt::qt_getEnumMetaObject",
            // reimplemented in Rust
            "QFlag",
            "QIncompatibleFlag",
            // not useful in Rust
            "QtSharedPointer",
            "QSharedPointer",
            "QWeakPointer",
            "QEnableSharedFromThis",
            "QScopedArrayPointer",
            // throws exception, so useless here
            "qBadAlloc",
            // requires user class templates, so useless here
            "QSharedDataPointer",
            "QExplicitlySharedDataPointer",
            "QSharedData",
            "QScopeGuard",
            "QScopedValueRollback",
            "QScopedPointer",
            "QScopedPointerObjectDeleteLater",
            "QScopedPointerPodDeleter",
            "QScopedPointerDeleter",
            "QScopedPointerArrayDeleter",
            "QGenericArgument",
            "QGenericReturnArgument",
            "QNonConstOverload",
            "QConstOverload",
            // global functions that redirects to member functions
            "swap",
            // is not cross-platform and is deprecated anyway
            "QProcess::pid",
        ];
        if blocked.contains(&string.as_str()) {
            return Ok(false);
        }

        Ok(true)
    });
    // TODO: replace QVariant::Type with QMetaType::Type?

    let connect_path = CppPath::from_good_str("QObject::connect");
    let connection_to_bool_function = connection_to_bool_function();
    let connection_is_valid_path = RustPath::from_good_str(&format!(
        "{}::q_meta_object::Connection::is_valid",
        crate_name2
    ));
    let connection_is_valid_path2 = connection_is_valid_path.clone();
    let qmetamethod_ref_type =
        CppType::new_reference(true, CppType::Class(CppPath::from_good_str("QMetaMethod")));
    config.set_rust_path_hook(move |_path, name_type, data| {
        if let NameType::ApiFunction(function) = name_type {
            if let CppFfiFunctionKind::Function = &function.item.kind {
                let cpp_function = data
                    .db
                    .source_cpp_item(&function.id)?
                    .ok_or_else(|| err_msg("source cpp item not found"))?
                    .item
                    .as_function_ref()
                    .ok_or_else(|| err_msg("invalid source cpp item type"))?;

                if cpp_function.path == connect_path && cpp_function.arguments.len() >= 3 {
                    if !cpp_function.is_static_member() {
                        bail!("non-static QObject::connect is blacklisted");
                    }
                    let arg = &cpp_function.arguments[1].argument_type;
                    if arg == &qmetamethod_ref_type {
                        return Ok(Some(RustPath::from_good_str(&format!(
                            "{}::QObject::connect_by_meta_methods",
                            data.db.crate_name()
                        ))));
                    }
                }

                if cpp_function.is_same(&connection_to_bool_function) {
                    return Ok(Some(connection_is_valid_path.clone()));
                }

                let q_text_stream_functions = &[
                    "bin",
                    "bom",
                    "center",
                    "dec",
                    "endl",
                    "fixed",
                    "flush",
                    "forcepoint",
                    "forcesign",
                    "hex",
                    "left",
                    "lowercasebase",
                    "lowercasedigits",
                    "noforcepoint",
                    "noforcesign",
                    "noshowbase",
                    "oct",
                    "reset",
                    "right",
                    "scientific",
                    "showbase",
                    "uppercasebase",
                    "uppercasedigits",
                    "ws",
                    "qSetFieldWidth",
                    "qSetPadChar",
                    "qSetRealNumberPrecision",
                ];

                if cpp_function.path.items().len() == 1
                    && q_text_stream_functions.contains(&cpp_function.path.items()[0].name.as_str())
                {
                    let path = RustPath::from_good_str(data.db.crate_name())
                        .join("q_text_stream")
                        .join(cpp_function.path.items()[0].name.to_snake_case());

                    return Ok(Some(path));
                }
            }
        }

        Ok(None)
    });

    config.set_rust_item_hook(move |item, _data| {
        if item.path() == Some(&connection_is_valid_path2) {
            if let RustItem::Function(item) = item {
                let arg = item
                    .arguments
                    .get_mut(0)
                    .ok_or_else(|| err_msg("missing argument"))?;
                arg.argument_type = RustFinalType::new(
                    arg.argument_type.ffi_type().clone(),
                    RustToFfiTypeConversion::RefToPtr {
                        force_api_is_const: None,
                        lifetime: None,
                    },
                )?;
                arg.name = "self".into();
            } else {
                bail!("unexpected item type");
            }
        }
        Ok(())
    });

    config.set_ffi_generator_hook(|item| {
        if let CppItem::Function(function) = &item {
            if let Ok(class_type) = function.class_path() {
                let class_text = class_type.to_templateless_string();
                if class_text == "QFlags" {
                    return Ok(false);
                }
            }
            if function.is_operator() {
                if let CppType::Class(path) = &function.return_type {
                    if path.to_templateless_string() == "QFlags" {
                        return Ok(false);
                    }
                    if path.to_templateless_string() == "QDebug" && function.arguments.len() == 2 {
                        if let CppType::Class(path2) = &function.arguments[1].argument_type {
                            if path2.to_templateless_string() == "QFlags" {
                                return Ok(false);
                            }
                        }
                    }
                }
            }
        }
        Ok(true)
    });

    let tests = if config.crate_properties().name().starts_with("moqt") {
        vec![PreliminaryTest::new(
            "moqt_abs",
            true,
            Snippet::new_in_main("ritual_assert(moqt_abs(-2) == 2);", false),
        )]
    } else {
        let code1 = format!(
            "ritual_assert(QLibraryInfo::version().toString() == \"{}\");",
            config.cpp_lib_version().unwrap()
        );
        let test1 = PreliminaryTest::new("qt_version_fn", true, Snippet::new_in_main(code1, false));

        let code2 = format!(
            "ritual_assert(strcmp(QT_VERSION_STR, \"{}\") == 0);",
            config.cpp_lib_version().unwrap()
        );
        let test2 = PreliminaryTest::new(
            "qt_version_define",
            true,
            Snippet::new_in_main(code2, false),
        );
        let code3 = "
            class Class1 : public QObject {
            Q_OBJECT
            public:
                Class1() {
                    emit signal1();
                }
            signals:
                void signal1();
            };

            void x() {
                Class1 c;
            }
        ";
        let test3 =
            PreliminaryTest::new("class_with_signal", true, Snippet::new_global(code3, true));

        vec![test1, test2, test3]
    };
    config.add_cpp_checker_tests(tests);

    config.add_after_cpp_parser_hook(add_find_child_methods);
    config.add_after_cpp_parser_hook(add_connection_to_bool);

    // for moqt_core
    let namespace = CppPath::from_good_str("ignored_ns");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("moqt_core"),
                prefix: None,
            }));
        }
        Ok(None)
    });
    Ok(())
}

fn add_find_child_methods(data: &mut ProcessorData<'_>, output: &CppParserOutput) -> Result<()> {
    for item in &output.0 {
        let cpp_item = data.db.cpp_item(&item.id)?;
        let function = if let Some(f) = cpp_item.item.as_function_ref() {
            f
        } else {
            continue;
        };
        let path = function.path.to_templateless_string();
        if path == "QObject::findChild" || path == "QObject::findChildren" {
            let t = CppType::new_pointer(false, CppType::Class(CppPath::from_good_str("QObject")));
            let new_function = instantiate_function(function, 0, &[t])?;
            data.db
                .add_cpp_item(Some(item.id.clone()), CppItem::Function(new_function))?;
        }
    }
    Ok(())
}

fn add_connection_to_bool(data: &mut ProcessorData<'_>, _output: &CppParserOutput) -> Result<()> {
    // `QMetaObject::Connection::operator bool()` is a fake method, so we need to
    // explicitly add a conversion function to replace it.
    data.db
        .add_cpp_item(None, CppItem::Function(connection_to_bool_function()))?;
    Ok(())
}

fn connection_to_bool_function() -> CppFunction {
    CppFunction {
        path: CppPath::from_item(CppPathItem {
            name: "static_cast".into(),
            template_arguments: Some(vec![CppType::BuiltInNumeric(CppBuiltInNumericType::Bool)]),
        }),
        member: None,
        allows_variadic_arguments: false,
        arguments: vec![CppFunctionArgument {
            name: "connection".into(),
            has_default_value: false,
            argument_type: CppType::new_reference(
                true,
                CppType::Class(CppPath::from_good_str("QMetaObject::Connection")),
            ),
        }],
        cast: None,
        operator: None,
        declaration_code: None,
        return_type: CppType::BuiltInNumeric(CppBuiltInNumericType::Bool),
    }
}
