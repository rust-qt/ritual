use itertools::Itertools;
use ritual::config::{Config, CrateDependencyKind, CrateDependencySource};
use ritual::cpp_checker::{PreliminaryTest, Snippet};
use ritual::cpp_data::{CppItem, CppPath, CppPathItem, CppTypeDeclaration, CppTypeDeclarationKind};
use ritual::cpp_ffi_data::CppFfiFunctionKind;
use ritual::cpp_function::{CppFunction, CppFunctionArgument};
use ritual::cpp_template_instantiator::instantiate_function;
use ritual::cpp_type::{CppBuiltInNumericType, CppType};
use ritual::processor::ProcessorData;
use ritual::rust_info::{NameType, RustItem, RustPathScope};
use ritual::rust_type::{RustFinalType, RustPath, RustToFfiTypeConversion};
use ritual::rustifier::Rustifier;
use ritual_common::errors::{bail, err_msg, Result};
use ritual_common::file_utils::repo_dir_path;
use ritual_common::string_utils::CaseOperations;

/// QtCore specific configuration.
pub fn core_config(config: &mut Config) -> Result<()> {
    config.crate_properties_mut().add_dependency(
        "qt_macros",
        CrateDependencyKind::Normal,
        CrateDependencySource::Local {
            path: repo_dir_path("qt_macros")?,
        },
    )?;
    config.crate_properties_mut().add_dependency(
        "proc-macro-hack",
        CrateDependencyKind::Normal,
        CrateDependencySource::CratesIo {
            version: "0.5.11".into(),
        },
    )?;

    let crate_name = config.crate_properties().name().to_string();
    let crate_name2 = crate_name.clone();

    let qt_namespace = CppPath::from_good_str("Qt");
    let moqt_ignored_namespace = CppPath::from_good_str("ignored_ns");
    config.set_rust_path_scope_hook(move |path| {
        if path == &qt_namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str(&crate_name),
                prefix: None,
            }));
        }
        if path == &moqt_ignored_namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("moqt_core"),
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
                    RustToFfiTypeConversion::RefToPtr { lifetime: None },
                )?;
                arg.name = "self".into();
            } else {
                bail!("unexpected item type");
            }
        }
        Ok(())
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

    config.processing_steps_mut().add_after(
        &["cpp_parser"],
        "add_extra_cpp_items",
        add_extra_cpp_items,
    )?;

    config.set_rustifier_hook(rustify);

    Ok(())
}

fn add_extra_cpp_items(data: &mut ProcessorData<'_>) -> Result<()> {
    add_find_child_methods(data)?;
    add_connection_to_bool(data)?;
    add_qpointer(data)?;
    Ok(())
}

fn add_find_child_methods(data: &mut ProcessorData<'_>) -> Result<()> {
    for id in data.db.cpp_item_ids().collect_vec() {
        let cpp_item = data.db.cpp_item(&id)?;
        let function = if let Some(f) = cpp_item.item.as_function_ref() {
            f
        } else {
            continue;
        };
        let path = function.path.to_templateless_string();
        if path == "QObject::findChild" || path == "QObject::findChildren" {
            let t = CppType::new_pointer(false, CppType::Class(CppPath::from_good_str("QObject")));
            let new_function = instantiate_function(function, 0, &[t])?;
            data.add_cpp_item(Some(id), CppItem::Function(new_function))?;
        }
    }
    Ok(())
}

fn add_connection_to_bool(data: &mut ProcessorData<'_>) -> Result<()> {
    // `QMetaObject::Connection::operator bool()` is a fake method, so we need to
    // explicitly add a conversion function to replace it.
    data.add_cpp_item(None, CppItem::Function(connection_to_bool_function()))?;
    Ok(())
}

fn add_qpointer(data: &mut ProcessorData<'_>) -> Result<()> {
    for id in data.db.cpp_item_ids().collect_vec() {
        let cpp_item = data.db.cpp_item(&id)?;
        let t = if let Some(f) = cpp_item.item.as_type_ref() {
            f
        } else {
            continue;
        };
        let is_qpointer_t = t.path.to_templateless_string() == "QPointer"
            && t.path
                .last()
                .template_arguments
                .as_ref()
                .map_or(false, |args| {
                    args.get(0).map_or(false, |arg| arg.is_template_parameter())
                });
        if is_qpointer_t {
            data.add_cpp_item(
                Some(id),
                CppItem::Type(CppTypeDeclaration {
                    path: CppPath::from_item(CppPathItem {
                        name: "QPointer".into(),
                        template_arguments: Some(vec![CppType::Class(CppPath::from_good_str(
                            "QObject",
                        ))]),
                    }),
                    kind: CppTypeDeclarationKind::Class,
                }),
            )?;
        }
    }
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

pub fn rustify(r: &mut Rustifier<'_>) -> Result<()> {
    r.add_cpp_code(include_str!(
        "../../crate_templates/common/QObjectLifetimeChecker.h"
    ));
    r.add_cpp_code(include_str!(
        "../../crate_templates/common/QObjectLifetimeChecker.cpp"
    ));
    rustify_qobject(r, "QObject")?;
    rustify_qobject(r, "QTimer")?;
    include_moc(r)?;
    Ok(())
}

pub fn add_lifetime_checker_header(r: &mut Rustifier<'_>) -> Result<()> {
    r.add_cpp_code("#ifndef Q_MOC_RUN");
    r.add_cpp_code(include_str!(
        "../../crate_templates/common/QObjectLifetimeChecker.h"
    ));
    r.add_cpp_code("#endif");
    r.add_cpp_code("extern QObjectLifetimeChecker* QOBJECT_LIFETIME_CHECKER;");
    Ok(())
}

pub fn rustify_qobject(r: &mut Rustifier<'_>, cpp_type: &str) -> Result<()> {
    let rust_name = cpp_type;
    r.add_rust_lib_code(&format!(
        "
        pub struct {rust_name}(::std::num::NonZeroUsize);
    "
    ));
    Ok(())
}
pub fn include_moc(rustifier: &mut Rustifier<'_>) -> Result<()> {
    rustifier.add_cpp_code(r#"#include "file1.moc""#);
    Ok(())
}
