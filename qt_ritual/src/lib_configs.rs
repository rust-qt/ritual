//! Generator configurations specific for each Qt module.

use crate::detect_signal_argument_types::detect_signal_argument_types;
use crate::detect_signals_and_slots::detect_signals_and_slots;
use crate::doc_parser::parse_docs;
use crate::fix_header_names::fix_header_names;
use crate::lib_configs;
use crate::versions;
use log::info;
use qt_ritual_common::{get_full_build_config, lib_dependencies, lib_folder_name};
use ritual::config::Config;
use ritual::config::CrateProperties;
use ritual::cpp_data::CppPath;
use ritual::processor::ProcessingStep;
use ritual_common::cpp_build_config::CppLibraryType;
use ritual_common::cpp_build_config::{CppBuildConfigData, CppBuildPaths};
use ritual_common::errors::{bail, Result, ResultExt};
use ritual_common::file_utils::repo_crate_local_path;
use ritual_common::target;
use ritual_common::toml;
use std::path::PathBuf;

/*
/// Helper method to blacklist all methods of `QList<T>` template instantiation that
/// don't work if `T` doesn't have `operator==`. `types` is list of such `T` types.
fn exclude_qlist_eq_based_methods<S: AsRef<str>, I: IntoIterator<Item = S>>(
  config: &mut Config,
  types: I,
) {
  let types: Vec<String> = types.into_iter().map(|x| x.as_ref().to_string()).collect();
  config.add_cpp_ffi_generator_filter(move |method| {
    if let Some(ref info) = method.class_membership {
      if info.class_type.name == "QList" {
        let args = info
          .class_type
          .template_arguments
          .as_ref()
          .with_context(|| "failed to get QList args")?;
        let arg = args.get(0).with_context(|| "failed to get QList arg")?;
        let arg_text = arg.to_cpp_pseudo_code();
        if types.iter().any(|x| x == &arg_text) {
          match method.name.as_ref() {
            "operator==" | "operator!=" | "indexOf" | "lastIndexOf" | "contains" | "startsWith"
            | "endsWith" | "removeOne" | "removeAll" | "value" | "toVector" | "toSet" => {
              return Ok(false)
            }
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
  });
}

/// Helper method to blacklist all methods of `QVector<T>` template instantiation that
/// don't work if `T` doesn't have `operator==`. `types` is list of such `T` types.
fn exclude_qvector_eq_based_methods<S: AsRef<str>, I: IntoIterator<Item = S>>(
  config: &mut Config,
  types: I,
) {
  let types: Vec<String> = types.into_iter().map(|x| x.as_ref().to_string()).collect();
  config.add_cpp_ffi_generator_filter(move |method| {
    if let Some(ref info) = method.class_membership {
      if info.class_type.name == "QVector" {
        let args = info
          .class_type
          .template_arguments
          .as_ref()
          .with_context(|| "failed to get QVector args")?;
        let arg = args.get(0).with_context(|| "failed to get QVector arg")?;
        let arg_text = arg.to_cpp_pseudo_code();
        if types.iter().any(|x| x == &arg_text) {
          match method.name.as_ref() {
            "operator==" | "operator!=" | "indexOf" | "lastIndexOf" | "contains" | "startsWith"
            | "endsWith" | "removeOne" | "removeAll" | "toList" => return Ok(false),
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
  });
}

/// List of QtCore identifiers that should be blacklisted.
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
}
*/
/// QtCore specific configuration.
pub fn core(config: &mut Config) -> Result<()> {
    // TODO: replace QVariant::Type with QMetaType::Type?
    //config.add_cpp_parser_blocked_names(core_cpp_parser_blocked_names());
    //config.add_cpp_parser_blocked_names(vec!["QtMetaTypePrivate", "QtPrivate"]);

    // TODO: the following items should be conditionally available on Windows;
    /*config.add_cpp_parser_blocked_names(vec![
      "QWinEventNotifier",
      "QProcess::CreateProcessArguments",
      "QProcess::nativeArguments",
      "QProcess::setNativeArguments",
      "QProcess::createProcessArgumentsModifier",
      "QProcess::setCreateProcessArgumentsModifier",
      "QAbstractEventDispatcher::registerEventNotifier",
      "QAbstractEventDispatcher::unregisterEventNotifier",
    ]);*/

    // QProcess::pid returns different types on different platforms,
    // but this method is obsolete anyway
    config.add_cpp_parser_blocked_names(vec![CppPath::from_str_unchecked("QProcess::pid")]);
    /*
    exclude_qvector_eq_based_methods(config, &["QStaticPlugin", "QTimeZone::OffsetData"]);
    exclude_qlist_eq_based_methods(
      config,
      &["QAbstractEventDispatcher::TimerInfo", "QCommandLineOption"],
    );

    config.set_types_allocation_place(
      CppTypeAllocationPlace::Stack,
      vec![
        "QAssociativeIterable",
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
        "QString",
      ],
    );

    config.add_cpp_ffi_generator_filter(|method| {
      if let Some(ref info) = method.class_membership {
        if info.class_type.to_cpp_pseudo_code() == "QFuture<void>" {
          // template partial specialization removes these methods
          match method.name.as_ref() {
            "operator void" | "isResultReadyAt" | "result" | "resultAt" | "results" => {
              return Ok(false)
            }
            _ => {}
          }
        }
        if info.class_type.to_cpp_pseudo_code() == "QFutureIterator<void>" {
          // template partial specialization removes these methods
          match method.name.as_ref() {
            "QFutureIterator" | "operator=" => return Ok(false),
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
            "registerConverterFunction" | "unregisterConverterFunction" => {
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
      if &method.name == "qHash" && method.class_membership.is_none()
        && (method.arguments.len() == 1 || method.arguments.len() == 2)
        && &method.arguments[0].argument_type == &long_double
      {
        return Ok(false); // produces error on MacOS
      }
      Ok(true)
    });*/
    Ok(())
}

/// QtGui specific configuration.
pub fn gui(_config: &mut Config) -> Result<()> {
    /*
      config.add_cpp_parser_blocked_names(vec![
        "QAbstractOpenGLFunctionsPrivate",
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
        "QTextFrameLayoutData",
      ]);
      exclude_qvector_eq_based_methods(
        config,
        &[
          "QTextLayout::FormatRange",
          "QAbstractTextDocumentLayout::Selection",
        ],
      );
      exclude_qlist_eq_based_methods(
        config,
        &[
          "QInputMethodEvent::Attribute",
          "QTextLayout::FormatRange",
          "QTouchEvent::TouchPoint",
        ],
      );
      config.add_cpp_ffi_generator_filter(|method| {
        if let Some(ref info) = method.class_membership {
          match info.class_type.to_cpp_pseudo_code().as_ref() {
            "QQueue<QInputMethodEvent::Attribute>"
            | "QQueue<QTextLayout::FormatRange>"
            | "QQueue<QTouchEvent::TouchPoint>" => match method.name.as_ref() {
              "operator==" | "operator!=" => return Ok(false),
              _ => {}
            },
            "QStack<QInputMethodEvent::Attribute>" | "QStack<QTextLayout::FormatRange>" => {
              match method.name.as_ref() {
                "operator==" | "operator!=" | "fromList" => return Ok(false),
                _ => {}
              }
            }
            "QOpenGLVersionFunctionsStorage" => match method.name.as_ref() {
              "QOpenGLVersionFunctionsStorage" | "~QOpenGLVersionFunctionsStorage" | "backend" => {
                return Ok(false)
              }
              _ => {}
            },
            _ => {}
          }
          if info.class_type.name.starts_with("QOpenGLFunctions_")
            && (info.class_type.name.ends_with("_CoreBackend")
              | info.class_type.name.ends_with("_CoreBackend::Functions")
              | info.class_type.name.ends_with("_DeprecatedBackend")
              | info
                .class_type
                .name
                .ends_with("_DeprecatedBackend::Functions"))
          {
            return Ok(false);
          }
        }
        Ok(true)
      });
    */
    Ok(())
}

/// QtWidgets specific configuration.
pub fn widgets(_config: &mut Config) -> Result<()> {
    /*
    config.add_cpp_parser_blocked_names(vec!["QWidgetData", "QWidgetItemV2"]);

    // TODO: Mac specific:
    config.add_cpp_parser_blocked_names(vec!["QMacCocoaViewContainer", "QMacNativeWidget"]);

    exclude_qlist_eq_based_methods(
      config,
      &["QTableWidgetSelectionRange", "QTextEdit::ExtraSelection"],
    );
    config.add_cpp_ffi_generator_filter(|method| {
      if let Some(ref info) = method.class_membership {
        match info.class_type.to_cpp_pseudo_code().as_ref() {
          "QQueue<QTableWidgetSelectionRange>" | "QQueue<QTextEdit::ExtraSelection>" => {
            match method.name.as_ref() {
              "operator==" | "operator!=" => return Ok(false),
              _ => {}
            }
          }
          _ => {}
        }
      }
      Ok(true)
    });*/
    Ok(())
}

/// Qt3DCore specific configuration.
pub fn core_3d(config: &mut Config) -> Result<()> {
    config.add_cpp_filtered_namespace(CppPath::from_str_unchecked("Qt3DCore"));
    //exclude_qvector_eq_based_methods(config, &["Qt3DCore::QNodeIdTypePair"]);
    Ok(())
}

/// Qt3DRender specific configuration.
pub fn render_3d(config: &mut Config) -> Result<()> {
    config.add_cpp_filtered_namespace(CppPath::from_str_unchecked("Qt3DRender"));
    /*
    config.add_cpp_parser_blocked_names(vec![
      "Qt3DRender::QTexture1D",
      "Qt3DRender::QTexture1DArray",
      "Qt3DRender::QTexture2D",
      "Qt3DRender::QTexture2DArray",
      "Qt3DRender::QTexture3D",
      "Qt3DRender::QTextureCubeMap",
      "Qt3DRender::QTextureCubeMapArray",
      "Qt3DRender::QTexture2DMultisample",
      "Qt3DRender::QTexture2DMultisampleArray",
      "Qt3DRender::QTextureRectangle",
      "Qt3DRender::QTextureBuffer",
      "Qt3DRender::QRenderCapture",
      "Qt3DRender::QRenderCaptureReply",
      "Qt3DRender::QSortCriterion",
    ]);
    config.add_cpp_ffi_generator_filter(|method| {
      if let Some(ref info) = method.class_membership {
        match info.class_type.to_cpp_pseudo_code().as_ref() {
          "Qt3DRender::QSpotLight" => match method.name.as_ref() {
            "attenuation" => return Ok(false),
            _ => {}
          },

          "Qt3DRender::QGraphicsApiFilter" => match method.name.as_ref() {
            "operator==" | "operator!=" => return Ok(false),
            _ => {}
          },

          _ => {}
        }
      }
      if method.short_text().contains("QGraphicsApiFilter") {
        println!("TEST {:?}", method);
      }
      if method.name == "Qt3DRender::operator==" || method.name == "Qt3DRender::operator!=" {
        if method.arguments.len() == 2 {
          if let CppTypeBase::Class(ref base) = method.arguments[0].argument_type.base {
            if &base.name == "Qt3DRender::QGraphicsApiFilter" {
              return Ok(false);
            }
          }
        }
      }
      Ok(true)
    });*/
    Ok(())
}

/// Qt3DInput specific configuration.
pub fn input_3d(config: &mut Config) -> Result<()> {
    config.add_cpp_filtered_namespace(CppPath::from_str_unchecked("Qt3DInput"));
    //config.add_cpp_parser_blocked_names(vec!["Qt3DInput::QWheelEvent"]);
    Ok(())
}

/// Qt3DLogic specific configuration.
pub fn logic_3d(config: &mut Config) -> Result<()> {
    config.add_cpp_filtered_namespace(CppPath::from_str_unchecked("Qt3DLogic"));
    Ok(())
}

/// Qt3DExtras specific configuration.
pub fn extras_3d(config: &mut Config) -> Result<()> {
    config.add_cpp_filtered_namespace(CppPath::from_str_unchecked("Qt3DExtras"));
    Ok(())
}

/// Executes the generator for a single Qt module with given configuration.
pub fn make_config(crate_name: &str) -> Result<Config> {
    info!("Preparing generator config for crate: {}", crate_name);
    let mut crate_properties = CrateProperties::new(crate_name, versions::QT_OUTPUT_CRATES_VERSION);
    let mut custom_fields = toml::value::Table::new();
    let mut package_data = toml::value::Table::new();
    package_data.insert(
        "authors".to_string(),
        toml::Value::Array(vec![toml::Value::String(
            "Pavel Strakhov <ri@idzaaus.org>".to_string(),
        )]),
    );
    let description = format!(
        "Bindings for {} C++ library (generated automatically with cpp_to_rust project)",
        lib_folder_name(crate_name)
    );
    package_data.insert("description".to_string(), toml::Value::String(description));
    let doc_url = format!("https://rust-qt.github.io/rustdoc/qt/{}", &crate_name);
    package_data.insert("documentation".to_string(), toml::Value::String(doc_url));
    package_data.insert(
        "repository".to_string(),
        toml::Value::String("https://github.com/rust-qt/cpp_to_rust".to_string()),
    );
    package_data.insert(
        "license".to_string(),
        toml::Value::String("MIT".to_string()),
    );

    custom_fields.insert("package".to_string(), toml::Value::Table(package_data));
    crate_properties.set_custom_fields(custom_fields);
    let mut config = if crate_name.starts_with("moqt_") {
        let mut config = Config::new(crate_properties);
        let moqt_path = PathBuf::from(
            ::std::env::var("MOQT_PATH").with_context(|_| "MOQT_PATH env var is missing")?,
        );

        config.add_include_directive(format!("{}.h", crate_name));
        let moqt_sublib_path = moqt_path.join(crate_name);
        if !moqt_sublib_path.exists() {
            bail!("Path does not exist: {}", moqt_sublib_path.display());
        }
        let include_path = moqt_sublib_path.join("include");
        if !include_path.exists() {
            bail!("Path does not exist: {}", include_path.display());
        }
        let lib_path = moqt_sublib_path.join("lib");
        if !lib_path.exists() {
            bail!("Path does not exist: {}", lib_path.display());
        }
        {
            let mut paths = CppBuildPaths::new();
            paths.add_include_path(&include_path);
            paths.add_lib_path(&lib_path);
            config.set_cpp_build_paths(paths);
        }
        config.add_target_include_path(&include_path);

        {
            let mut data = CppBuildConfigData::new();
            data.add_linked_lib(crate_name.replace("_", ""));
            data.set_library_type(CppLibraryType::Shared);
            config
                .cpp_build_config_mut()
                .add(target::Condition::True, data);
        }
        {
            let mut data = CppBuildConfigData::new();
            data.add_compiler_flag("-fPIC");
            data.add_compiler_flag("-std=gnu++11");
            config
                .cpp_build_config_mut()
                .add(target::Condition::Env(target::Env::Msvc).negate(), data);
        }
        if target::current_env() == target::Env::Msvc {
            config.add_cpp_parser_argument("-std=c++14");
        } else {
            config.add_cpp_parser_argument("-std=gnu++11");
        }
        //    let cpp_config_data = CppBuildConfigData {
        //      linked_libs: vec![crate_name.to_string()],
        //      linked_frameworks: Vec::new(),
        //
        //    }
        //...
        config
    } else {
        crate_properties.remove_default_build_dependencies();
        crate_properties.add_build_dependency(
            "qt_build_tools",
            versions::QT_BUILD_TOOLS_VERSION,
            Some(repo_crate_local_path("qt_generator/qt_build_tools")?),
        );

        let mut config = Config::new(crate_properties);

        let qt_config = get_full_build_config(crate_name)?;
        config.set_cpp_build_config(qt_config.cpp_build_config);
        config.set_cpp_build_paths(qt_config.cpp_build_paths);

        config.add_target_include_path(&qt_config.installation_data.lib_include_path);
        config.set_cpp_lib_version(qt_config.installation_data.qt_version.as_str());
        // TODO: does parsing work on MacOS without adding "-F"?

        config.add_include_directive(&lib_folder_name(crate_name));
        // TODO: allow to override parser flags
        config.add_cpp_parser_arguments(vec!["-fPIC", "-fcxx-exceptions"]);

        if target::current_env() == target::Env::Msvc {
            config.add_cpp_parser_argument("-std=c++14");
        } else {
            config.add_cpp_parser_argument("-std=gnu++11");
        }
        //config.add_cpp_parser_blocked_name(CppName::from_one_part("qt_check_for_QGADGET_macro"));

        let lib_include_path = qt_config.installation_data.lib_include_path.clone();

        config.add_custom_processing_step(ProcessingStep::new(
            "qt_fix_header_names",
            vec!["cpp_parser".to_string()],
            move |data| fix_header_names(&mut data.current_database.items, &lib_include_path),
        ));

        config.add_custom_processing_step(ProcessingStep::new(
            "qt_detect_signals_and_slots",
            vec!["cpp_parser".to_string()],
            detect_signals_and_slots,
        ));
        config.add_custom_processing_step(ProcessingStep::new(
            "detect_signal_argument_types",
            vec![
                "cpp_parser".to_string(),
                "qt_detect_signals_and_slots".to_string(),
            ],
            detect_signal_argument_types,
        ));

        let crate_name_clone = crate_name.to_string();
        let docs_path = qt_config.installation_data.docs_path.clone();
        config.add_custom_processing_step(ProcessingStep::new(
            "qt_doc_parser",
            vec!["cpp_parser".to_string()],
            move |data| parse_docs(data, &crate_name_clone, &docs_path),
        ));

        config
    };

    config.set_crate_template_path(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("crate_templates")
            .join(&crate_name),
    );
    match crate_name {
        "qt_core" => lib_configs::core(&mut config)?,
        "qt_gui" => lib_configs::gui(&mut config)?,
        "qt_widgets" => lib_configs::widgets(&mut config)?,
        "qt_3d_core" => lib_configs::core_3d(&mut config)?,
        "qt_3d_render" => lib_configs::render_3d(&mut config)?,
        "qt_3d_input" => lib_configs::input_3d(&mut config)?,
        "qt_3d_logic" => lib_configs::logic_3d(&mut config)?,
        "qt_3d_extras" => lib_configs::extras_3d(&mut config)?,
        "qt_ui_tools" => {}
        "moqt_core" => {}
        _ => bail!("Unknown crate name: {}", crate_name),
    }

    config.set_dependent_cpp_crates(
        lib_dependencies(crate_name)?
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    Ok(config)
}
