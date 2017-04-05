use cpp_to_rust_common::errors::{Result, ChainErr};
use cpp_to_rust_generator::config::{Config, CppTypeAllocationPlace};
use cpp_to_rust_generator::cpp_type::{CppType, CppTypeBase, CppBuiltInNumericType,
                                      CppTypeIndirection};

fn exclude_qlist_eq_based_methods<S: AsRef<str>, I: IntoIterator<Item = S>>(config: &mut Config,
                                                                            types: I) {
  let types: Vec<String> = types
    .into_iter()
    .map(|x| x.as_ref().to_string())
    .collect();
  config.add_cpp_ffi_generator_filter(Box::new(move |method| {
    if let Some(ref info) = method.class_membership {
      if info.class_type.name == "QList" {
        let args = info.class_type
          .template_arguments
          .as_ref()
          .chain_err(|| "failed to get QList args")?;
        let arg = args.get(0).chain_err(|| "failed to get QList arg")?;
        let arg_text = arg.to_cpp_pseudo_code();
        if types.iter().any(|x| x == &arg_text) {
          match method.name.as_ref() {
            "operator==" | "operator!=" | "indexOf" | "lastIndexOf" | "contains" |
            "startsWith" | "endsWith" | "removeOne" | "removeAll" | "value" | "toVector" |
            "toSet" => return Ok(false),
            "count" => {
              if method.arguments.len() == 1 {
                return Ok(false);
              }
            }
            _ => {}
          }
        }
      }
    }
    Ok(true)
  }));
}

fn exclude_qvector_eq_based_methods<S: AsRef<str>, I: IntoIterator<Item = S>>(config: &mut Config,
                                                                              types: I) {
  let types: Vec<String> = types
    .into_iter()
    .map(|x| x.as_ref().to_string())
    .collect();
  config.add_cpp_ffi_generator_filter(Box::new(move |method| {
    if let Some(ref info) = method.class_membership {
      if info.class_type.name == "QVector" {
        let args = info.class_type
          .template_arguments
          .as_ref()
          .chain_err(|| "failed to get QVector args")?;
        let arg = args.get(0).chain_err(|| "failed to get QVector arg")?;
        let arg_text = arg.to_cpp_pseudo_code();
        if types.iter().any(|x| x == &arg_text) {
          match method.name.as_ref() {
            "operator==" | "operator!=" | "indexOf" | "lastIndexOf" | "contains" |
            "startsWith" | "endsWith" | "removeOne" | "removeAll" | "toList" => return Ok(false),
            "count" => {
              if method.arguments.len() == 1 {
                return Ok(false);
              }
            }
            _ => {}
          }
        }
      }
    }
    Ok(true)
  }));
}

#[cfg_attr(rustfmt, rustfmt_skip)]
fn core_cpp_parser_blocked_names() -> Vec<&'static str> {
  vec![
    "QAbstractConcatenable", "QAlgorithmsPrivate", "QArrayData",
    "QArrayDataPointer", "QArrayDataPointerRef", "QAtomicAdditiveType",
    "QAtomicInt", "QAtomicInteger", "QAtomicOps", "QAtomicPointer",
    "QBasicAtomicInteger", "QBasicAtomicInteger", "QBasicAtomicPointer",
    "QBitArray::detach", "QBitArray::isDetached", "QByteArray::detach",
    "QByteArray::isDetached", "QByteArray::isSharedWith", "QByteArrayDataPtr",
    "QConcatenable", "QConstOverload", "QContiguousCache::detach",
    "QContiguousCache::isDetached", "QContiguousCache::setSharable",
    "QContiguousCacheData", "QContiguousCacheTypedData", "QEnableSharedFromThis",
    "QException", "QFlag", "QForeachContainer", "QGenericAtomicOps",
    "QHash::detach", "QHash::isDetached", "QHash::setSharable", "QHashData",
    "QHashDummyValue", "QHashNode", "QHashNode", "QIncompatibleFlag", "QInternal",
    "QJsonValuePtr", "QJsonValueRefPtr", "QLinkedList::detach",
    "QLinkedList::isDetached", "QLinkedList::isSharedWith",
    "QLinkedList::setSharable", "QLinkedListData", "QLinkedListNode",
    "QList::detach", "QList::detachShared", "QList::isDetached",
    "QList::isSharedWith", "QList::setSharable", "QListData", "QMap::detach",
    "QMap::isDetached", "QMap::isSharedWith", "QMap::setSharable", "QMapData",
    "QMapDataBase", "QMapNode", "QMapNodeBase", "QMessageLogContext::copy",
    "QMetaObject::Connection::isConnected_helper", "QMetaTypeId", "QMetaTypeId2",
    "QNoDebug", "QNonConstOverload", "QObject::registerUserData", "QObjectData",
    "QObjectUserData", "QObjectUserData", "QPersistentModelIndex::internalId",
    "QPersistentModelIndex::internalPointer", "QScopedPointerArrayDeleter",
    "QScopedPointerDeleter", "QScopedPointerObjectDeleteLater",
    "QScopedPointerPodDeleter", "QSet::detach", "QSet::isDetached",
    "QSet::setSharable", "QString::Null", "QString::detach",
    "QString::isDetached", "QString::isSharedWith", "QString::isSimpleText",
    "QString::vasprintf", "QString::vsprintf", "QStringDataPtr",
    "QThreadStorageData", "QTypeInfo", "QTypeInfoMerger", "QTypeInfoQuery",
    "QTypedArrayData", "QUnhandledException", "QUrl::detach", "QUrl::isDetached",
    "QUrlQuery::isDetached", "QVariant::Handler", "QVariant::Private",
    "QVariant::PrivateShared", "QVariant::constData", "QVariant::data",
    "QVariant::detach", "QVariant::isDetached", "QVariantComparisonHelper",
    "QVector::detach", "QVector::isDetached", "QVector::isSharedWith",
    "QVector::setSharable", "Qt::Initialization", "QtGlobalStatic",
    "QtMetaTypePrivate", "QtPrivate", "QtSharedPointer", "QtStringBuilder",
    "_GUID", "qBadAlloc", "qErrnoWarning", "qFlagLocation", "qGreater", "qLess",
    "qMapLessThanKey", "qSharedBuild", "qYouForgotTheQ_OBJECT_Macro",
    "qbswap_helper", "qobject_interface_iid", "qt_QMetaEnum_debugOperator",
    "qt_QMetaEnum_flagDebugOperator", "qt_assert", "qt_assert_x",
    "qt_check_for_QGADGET_macro", "qt_check_for_QOBJECT_macro",
    "qt_check_pointer", "qt_hash", "qt_message_output", "qt_metacall",
    "qt_metacast", "qt_noop", "qt_qFindChild_helper", "qt_qFindChildren_helper",
    "qt_sharedpointer_cast_check", "qvsnprintf", "std",
    "qThreadStorage_deleteData", "QStringBuilderCommon", "QStringBuilderBase", "QStringBuilder",
    "QFutureInterfaceBase", "QFutureInterface", "QFutureWatcherBase", "QFutureWatcher"
  ]
  //next: QAbstractItemModel
}

pub fn core(config: &mut Config) -> Result<()> {
  config.add_cpp_parser_blocked_names(core_cpp_parser_blocked_names());

  // TODO: the following items should be conditionally available on Windows;
  config.add_cpp_parser_blocked_names(vec!["QWinEventNotifier",
                                           "QProcess::CreateProcessArguments",
                                           "QProcess::nativeArguments",
                                           "QProcess::setNativeArguments",
                                           "QProcess::createProcessArgumentsModifier",
                                           "QProcess::setCreateProcessArgumentsModifier",
                                           "QAbstractEventDispatcher::registerEventNotifier",
                                           "QAbstractEventDispatcher::unregisterEventNotifier"]);

  // QProcess::pid returns different types on different platforms,
  // but this method is obsolete anyway
  config.add_cpp_parser_blocked_names(vec!["QProcess::pid"]);

  exclude_qvector_eq_based_methods(config, &["QStaticPlugin", "QTimeZone::OffsetData"]);
  exclude_qlist_eq_based_methods(config,
                                 &["QAbstractEventDispatcher::TimerInfo", "QCommandLineOption"]);

  config.set_types_allocation_place(CppTypeAllocationPlace::Stack,
                                    vec!["QAssociativeIterable",
                                         "QByteArray",
                                         "QChar",
                                         "QItemSelection",
                                         "QJsonArray",
                                         "QJsonObject",
                                         "QJsonParseError",
                                         "QJsonValue",
                                         "QJsonValueRef",
                                         "QList",
                                         "QLoggingCategory",
                                         "QMultiHash",
                                         "QPointF",
                                         "QRegularExpressionMatch",
                                         "QResource",
                                         "QSequentialIterable",
                                         "QString"]);

  config.add_cpp_ffi_generator_filter(Box::new(|method| {
    if let Some(ref info) = method.class_membership {
      if info.class_type.to_cpp_pseudo_code() == "QFuture<void>" {
        // template partial specialization removes these methods
        match method.name.as_ref() {
          "operator void" |
          "isResultReadyAt" |
          "result" |
          "resultAt" |
          "results" => return Ok(false),
          _ => {}
        }
      }
      if info.class_type.to_cpp_pseudo_code() == "QFutureIterator<void>" {
        // template partial specialization removes these methods
        match method.name.as_ref() {
          "QFutureIterator" |
          "operator=" => return Ok(false),
          _ => {}
        }
      }
      if info.class_type.name == "QString" {
        match method.name.as_ref() {
          "toLatin1" | "toUtf8" | "toLocal8Bit" => {
            // MacOS has non-const duplicates of these methods,
            // and that would alter Rust names of these methods
            if !info.is_const {
              return Ok(false);
            }
          }
          _ => {}
        }
      }
      if info.class_type.name == "QMetaType" {
        match method.name.as_ref() {
          "registerConverterFunction" |
          "unregisterConverterFunction" => {
            // only public on msvc for some technical reason
            return Ok(false);
          }
          _ => {}
        }
      }
      if info.class_type.name == "QVariant" {
        match method.name.as_ref() {
          "create" | "cmp" | "compare" | "convert" => {
            // only public on msvc for some technical reason
            return Ok(false);
          }
          _ => {}
        }
      }
    }
    let long_double = CppType {
      indirection: CppTypeIndirection::None,
      is_const: false,
      is_const2: false,
      base: CppTypeBase::BuiltInNumeric(CppBuiltInNumericType::LongDouble),
    };
    if &method.name == "qHash" && method.class_membership.is_none() &&
       (method.arguments.len() == 1 || method.arguments.len() == 2) &&
       &method.arguments[0].argument_type == &long_double {
      return Ok(false); // produces error on MacOS
    }
    Ok(true)
  }));
  Ok(())
}

pub fn gui(config: &mut Config) -> Result<()> {
  config.add_cpp_parser_blocked_names(vec!["QAbstractOpenGLFunctionsPrivate",
                                           "QOpenGLFunctionsPrivate",
                                           "QOpenGLExtraFunctionsPrivate",
                                           "QKeySequence::isDetached",
                                           "QBrushData",
                                           "QAccessible::ActivationObserver",
                                           "QAccessibleImageInterface",
                                           "QAccessibleBridge",
                                           "QAccessibleBridgePlugin",
                                           "QAccessibleApplication",
                                           "QOpenGLVersionStatus",
                                           "QOpenGLVersionFunctionsBackend",
                                           "QOpenGLVersionFunctionsStorage",
                                           "QOpenGLTexture::TextureFormatClass",
                                           "QTextFrameLayoutData"]);
  exclude_qvector_eq_based_methods(config,
                                   &["QTextLayout::FormatRange",
                                     "QAbstractTextDocumentLayout::Selection"]);
  exclude_qlist_eq_based_methods(config,
                                 &["QInputMethodEvent::Attribute",
                                   "QTextLayout::FormatRange",
                                   "QTouchEvent::TouchPoint"]);
  config.add_cpp_ffi_generator_filter(Box::new(|method| {
    if let Some(ref info) = method.class_membership {
      match info.class_type.to_cpp_pseudo_code().as_ref() {
        "QQueue<QInputMethodEvent::Attribute>" |
        "QQueue<QTextLayout::FormatRange>" |
        "QQueue<QTouchEvent::TouchPoint>" => {
          match method.name.as_ref() {
            "operator==" | "operator!=" => return Ok(false),
            _ => {}
          }
        }
        "QStack<QInputMethodEvent::Attribute>" |
        "QStack<QTextLayout::FormatRange>" => {
          match method.name.as_ref() {
            "operator==" | "operator!=" | "fromList" => return Ok(false),
            _ => {}
          }
        }
        "QOpenGLVersionFunctionsStorage" => {
          match method.name.as_ref() {
            "QOpenGLVersionFunctionsStorage" |
            "~QOpenGLVersionFunctionsStorage" |
            "backend" => return Ok(false),
            _ => {}
          }
        }
        _ => {}
      }
      if info.class_type.name.starts_with("QOpenGLFunctions_") &&
         (info.class_type.name.ends_with("_CoreBackend") |
          info
            .class_type
            .name
            .ends_with("_CoreBackend::Functions") |
          info.class_type.name.ends_with("_DeprecatedBackend") |
          info
            .class_type
            .name
            .ends_with("_DeprecatedBackend::Functions")) {
        return Ok(false);
      }
    }
    Ok(true)
  }));

  Ok(())
}
pub fn widgets(config: &mut Config) -> Result<()> {
  config.add_cpp_parser_blocked_names(vec!["QWidgetData", "QWidgetItemV2"]);

  // TODO: Mac specific:
  config.add_cpp_parser_blocked_names(vec!["QMacCocoaViewContainer", "QMacNativeWidget"]);

  exclude_qlist_eq_based_methods(config,
                                 &["QTableWidgetSelectionRange", "QTextEdit::ExtraSelection"]);
  config.add_cpp_ffi_generator_filter(Box::new(|method| {
    if let Some(ref info) = method.class_membership {
      match info.class_type.to_cpp_pseudo_code().as_ref() {
        "QQueue<QTableWidgetSelectionRange>" |
        "QQueue<QTextEdit::ExtraSelection>" => {
          match method.name.as_ref() {
            "operator==" | "operator!=" => return Ok(false),
            _ => {}
          }
        }
        _ => {}
      }
    }
    Ok(true)
  }));
  Ok(())
}
