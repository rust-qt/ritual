use ritual::config::Config;
use ritual::cpp_data::{CppItem, CppPath};
use ritual::cpp_ffi_data::CppFfiFunctionKind;
use ritual::cpp_type::CppType;
use ritual::rust_info::{NameType, RustPathScope};
use ritual::rust_type::RustPath;
use ritual_common::errors::{bail, Result};

/// QtCore specific configuration.
pub fn core_config(config: &mut Config) -> Result<()> {
    let crate_name = config.crate_properties().name().to_string();
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
        ];
        if blocked.contains(&string.as_str()) {
            return Ok(false);
        }

        Ok(true)
    });
    // TODO: replace QVariant::Type with QMetaType::Type?

    let connect_path = CppPath::from_good_str("QObject::connect");
    let qmetamethod_ref_type =
        CppType::new_reference(true, CppType::Class(CppPath::from_good_str("QMetaMethod")));
    config.set_rust_path_hook(move |_path, name_type, data| {
        if let NameType::ApiFunction(function) = name_type {
            if let CppFfiFunctionKind::Function { cpp_function, .. } = &function.kind {
                if cpp_function.path == connect_path && cpp_function.arguments.len() >= 3 {
                    if !cpp_function.is_static_member() {
                        bail!("non-static QObject::connect is blacklisted");
                    }
                    let arg = &cpp_function.arguments[1].argument_type;
                    if arg == &qmetamethod_ref_type {
                        return Ok(Some(RustPath::from_good_str(&format!(
                            "{}::QObject::connect_by_meta_methods",
                            data.current_database.crate_name()
                        ))));
                    }
                }
            }
        }
        Ok(None)
    });

    config.set_ffi_generator_hook(|item| {
        if let CppItem::Function(function) = &item.item {
            if let Ok(class_type) = function.class_type() {
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
                }
            }
        }
        Ok(true)
    });
    Ok(())
}
