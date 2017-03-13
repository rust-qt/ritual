use cpp_to_rust_common::errors::{Result, ChainErr};
use cpp_to_rust_generator::config::{Config, CppTypeAllocationPlace};
use cpp_to_rust_generator::cpp_type::{CppType, CppTypeBase, CppBuiltInNumericType, CppTypeIndirection};

fn exclude_qlist_eq_based_methods<S: AsRef<str>, I: IntoIterator<Item = S>>(config: &mut Config,
                                                                            types: I) {
  let types: Vec<String> = types.into_iter().map(|x| x.as_ref().to_string()).collect();
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
  let types: Vec<String> = types.into_iter().map(|x| x.as_ref().to_string()).collect();
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
pub fn core(config: &mut Config) -> Result<()> {
  config.add_cpp_parser_blocked_names(vec![
    "_GUID", "QAbstractConcatenable", "QArrayData", "QArrayDataPointer", "QArrayDataPointerRef",
    "QAtomicAdditiveType", "QAtomicInt", "QAtomicInteger", "QAtomicOps", "QAtomicPointer",
    "QBasicAtomicInteger", "QBasicAtomicInteger", "QBasicAtomicPointer", "QByteArrayDataPtr",
    "QConcatenable", "QConstOverload", "QContiguousCacheData", "QContiguousCacheTypedData",
    "QEnableSharedFromThis", "QException", "QFlag", "QForeachContainer", "QGenericAtomicOps",
    "qGreater", "QHashData",
    "QHashDummyValue", "QHashNode", "QHashNode", "QIncompatibleFlag", "QInternal", "QJsonValuePtr",
    "QJsonValueRefPtr", "qLess", "QLinkedListData", "QLinkedListNode", "QListData", "QMapData",
    "QMapDataBase", "QMapNode",
    "QMapNodeBase", "QMessageLogContext::copy", "QMetaTypeId", "QMetaTypeId2", "QNoDebug",
    "QNonConstOverload", "QObjectData", "QObjectUserData", "QObjectUserData", "QScopedPointerPodDeleter",
    "QString::Null",
    "QString::vasprintf", "QString::vsprintf", "QStringDataPtr", "Qt::Initialization",
    "qt_check_for_QGADGET_macro", "QtGlobalStatic", "QThreadStorageData", "QtMetaTypePrivate",
    "QtPrivate", "QtSharedPointer", "QtStringBuilder", "QTypedArrayData", "QTypeInfo", "QTypeInfoMerger",
    "QTypeInfoQuery", "QVariant::Handler", "QVariant::Private", "QVariant::PrivateShared",
    "QVariantComparisonHelper", "qvsnprintf", "QUnhandledException"
  ]);

  // TODO: the following items should be conditionally available on Windows
  config.add_cpp_parser_blocked_names(vec!["QWinEventNotifier",
                                           "QProcess::CreateProcessArguments",
                                           "QProcess::nativeArguments",
                                           "QProcess::setNativeArguments",
                                           "QProcess::createProcessArgumentsModifier",
                                           "QProcess::setCreateProcessArgumentsModifier"]);

  // QProcess::pid returns different types on different platforms,
  // but this method is obsolete anyway
  config.add_cpp_parser_blocked_names(vec!["QProcess::pid"]);

  exclude_qvector_eq_based_methods(config, &["QStaticPlugin", "QTimeZone::OffsetData"]);
  exclude_qlist_eq_based_methods(config,
                                 &["QAbstractEventDispatcher::TimerInfo", "QCommandLineOption"]);

  config.set_types_allocation_place(CppTypeAllocationPlace::Stack, vec![
    "QAssociativeIterable", "QByteArray", "QChar", "QItemSelection", "QJsonArray",
    "QJsonObject", "QJsonParseError",
    "QJsonValue", "QJsonValueRef", "QList", "QLoggingCategory", "QMultiHash",
    "QPointF", "QRegularExpressionMatch", "QResource", "QSequentialIterable", "QString"]);

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
                                           "QOpenGLExtraFunctionsPrivate"]);
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
          info.class_type.name.ends_with("_CoreBackend::Functions") |
          info.class_type.name.ends_with("_DeprecatedBackend") |
          info.class_type.name.ends_with("_DeprecatedBackend::Functions")) {
        return Ok(false);
      }
    }
    Ok(true)
  }));

  Ok(())
}
pub fn widgets(config: &mut Config) -> Result<()> {
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
