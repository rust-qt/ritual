//! Functions for setting configuration and executing the generator.

use cpp_to_rust_generator::common::errors::Result;
use cpp_to_rust_generator::common::{log, toml};
use cpp_to_rust_generator::common::file_utils::{PathBufWithAdded, repo_crate_local_path};
use cpp_to_rust_generator::config::{Config, CacheUsage, DebugLoggingConfig};
use cpp_to_rust_generator::cpp_data::CppVisibility;
use cpp_to_rust_generator::common::cpp_build_config::{CppBuildConfigData, CppLibraryType};
use cpp_to_rust_generator::common::target;
use qt_generator_common::{get_installation_data, lib_folder_name, lib_dependencies};
use std::path::PathBuf;
use versions;

use doc_parser::DocParser;
use fix_header_names::fix_header_names;
use cpp_to_rust_generator::cpp_method::CppMethod;
use cpp_to_rust_generator::cpp_data::CppTypeKind;
use cpp_to_rust_generator::config::{CrateProperties, is_completed};
use doc_decoder::DocData;
use lib_configs;

/// Options passed to `exec_all`,
/// as in `cpp_to_rust_generator::config::Config`.
pub struct ExecConfig {
  pub write_dependencies_local_paths: bool,
  pub cache_usage: CacheUsage,
  pub write_cache: bool,
  pub debug_logging_config: DebugLoggingConfig,
  pub quiet_mode: bool,
}

/// Executes generator for `libs` with given configuration.
pub fn exec_all(libs: Vec<String>,
                cache_dir: PathBuf,
                output_dir: PathBuf,
                config: ExecConfig)
                -> Result<()> {
  if config.quiet_mode {
    let mut logger = log::default_logger();

    logger.set_category_settings(log::Status,
                                 log::LoggerSettings {
                                   file_path: None,
                                   write_to_stderr: false,
                                 });
  }

  let crate_templates_path =
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).with_added("crate_templates");
  let final_libs = if libs.iter().any(|x| x == "all") {
    vec!["core".to_string(),
         "gui".to_string(),
         "widgets".to_string(),
         "3d_core".to_string(),
         "3d_render".to_string(),
         "3d_input".to_string(),
         "3d_logic".to_string(),
         "3d_extras".to_string(),
         "ui_tools".to_string()]
  } else {
    libs
  };
  for sublib_name in final_libs {
    let lib_cache_dir = cache_dir.with_added(format!("qt_{}", sublib_name));
    let lib_crate_templates_path = crate_templates_path.with_added(&sublib_name);
    let lib_output_dir = output_dir.with_added(format!("qt_{}", sublib_name));

    let mut dependency_paths = Vec::new();
    for dep in lib_dependencies(&sublib_name)? {
      let path = cache_dir.with_added(format!("qt_{}", dep));
      if !is_completed(&path) {
        return Err(format!("\"{}\" depends on \"{}\" but processing \
          in \"{}\" directory is not completed.",
                           sublib_name,
                           dep,
                           path.display())
                       .into());
      }
      dependency_paths.push(path);
    }
    exec(&sublib_name,
         lib_cache_dir,
         lib_output_dir,
         lib_crate_templates_path,
         dependency_paths,
         &config)?;
  }
  Ok(())
}

/// Executes the generator for a single Qt module with given configuration.
fn exec(sublib_name: &str,
        cache_dir: PathBuf,
        output_dir: PathBuf,
        crate_templates_path: PathBuf,
        dependency_paths: Vec<PathBuf>,
        exec_config: &ExecConfig)
        -> Result<()> {
  if is_completed(&cache_dir) && exec_config.cache_usage.can_skip_all() {
    log::status("No processing! cpp_to_rust uses previous results.");
    log::status("Run with -C0 to force full processing.");
    return Ok(());
  }
  log::status(format!("Processing library: {}", sublib_name));
  let crate_name = format!("qt_{}", sublib_name);
  let mut crate_properties = CrateProperties::new(crate_name.clone(),
                                                  versions::QT_OUTPUT_CRATES_VERSION);
  let mut custom_fields = toml::Table::new();
  let mut package_data = toml::Table::new();
  package_data.insert("authors".to_string(),
                      toml::Value::Array(vec![toml::Value::String("Pavel Strakhov <ri@idzaaus.org>"
                                                                    .to_string())]));
  let description = format!("Bindings for {} C++ library (generated automatically with cpp_to_rust project)",
                            lib_folder_name(sublib_name));
  package_data.insert("description".to_string(), toml::Value::String(description));
  let doc_url = format!("https://rust-qt.github.io/rustdoc/qt/{}", &crate_name);
  package_data.insert("documentation".to_string(), toml::Value::String(doc_url));
  package_data.insert("repository".to_string(),
                      toml::Value::String("https://github.com/rust-qt/cpp_to_rust".to_string()));
  package_data.insert("license".to_string(),
                      toml::Value::String("MIT".to_string()));

  custom_fields.insert("package".to_string(), toml::Value::Table(package_data));
  crate_properties.set_custom_fields(custom_fields);
  crate_properties.remove_default_build_dependencies();
  let qt_build_tools_path = if exec_config.write_dependencies_local_paths {
    Some(repo_crate_local_path("qt_generator/qt_build_tools")?)
  } else {
    None
  };
  crate_properties.add_build_dependency("qt_build_tools",
                                        versions::QT_BUILD_TOOLS_VERSION,
                                        qt_build_tools_path);
  let mut config = Config::new(&output_dir, &cache_dir, crate_properties);
  let installation_data = get_installation_data(sublib_name)?;
  config.add_include_path(&installation_data.root_include_path);
  config.add_include_path(&installation_data.lib_include_path);
  for dep in lib_dependencies(&sublib_name)? {
    let dep_data = get_installation_data(dep)?;
    config.add_include_path(&dep_data.lib_include_path);
  }
  config.add_target_include_path(&installation_data.lib_include_path);
  config.set_cache_usage(exec_config.cache_usage.clone());
  config.set_write_dependencies_local_paths(exec_config.write_dependencies_local_paths);
  config.set_write_cache(exec_config.write_cache);
  config.set_quiet_mode(exec_config.quiet_mode);
  config.set_debug_logging_config(exec_config.debug_logging_config.clone());
  config.set_cpp_lib_version(installation_data.qt_version.as_str());
  if exec_config.write_dependencies_local_paths {
    log::status("Output Cargo.toml file will contain local paths of used dependencies \
               (use --no-local-paths to disable).");
  } else {
    log::status("Local paths will not be written to the output crate. Make sure all dependencies \
               are published before trying to compile the crate.");
  }
  // TODO: does parsing work on MacOS without adding "-F"?

  config.add_include_directive(&lib_folder_name(sublib_name));
  let lib_include_path = installation_data.lib_include_path.clone();
  config.add_cpp_data_filter(move |cpp_data| fix_header_names(cpp_data, &lib_include_path));
  config.add_cpp_parser_arguments(vec!["-fPIC", "-fcxx-exceptions"]);
  {
    let mut data = CppBuildConfigData::new();
    data.add_compiler_flag("-std=gnu++11");
    config
      .cpp_build_config_mut()
      .add(target::Condition::Env(target::Env::Msvc).negate(), data);
  }
  {
    let mut data = CppBuildConfigData::new();
    data.add_compiler_flag("-fPIC");
    // msvc and mingw don't need this
    config
      .cpp_build_config_mut()
      .add(target::Condition::OS(target::OS::Windows).negate(), data);
  }
  {
    let mut data = CppBuildConfigData::new();
    data.set_library_type(CppLibraryType::Shared);
    config
      .cpp_build_config_mut()
      .add(target::Condition::Env(target::Env::Msvc), data);
  }

  if target::current_env() == target::Env::Msvc {
    config.add_cpp_parser_argument("-std=c++14");
  } else {
    config.add_cpp_parser_argument("-std=gnu++11");
  }
  config.add_cpp_parser_blocked_name("qt_check_for_QGADGET_macro");
  let sublib_name_clone = sublib_name.to_string();
  let docs_path = installation_data.docs_path.clone();

  config.add_cpp_data_filter(move |cpp_data| {
    match DocData::new(&sublib_name_clone, &docs_path) {
      Ok(doc_data) => {
        let mut parser = DocParser::new(doc_data);
        find_methods_docs(&mut cpp_data.methods, &mut parser)?;
        for type1 in &mut cpp_data.types {
          match parser.doc_for_type(&type1.name) {
            Ok(doc) => {
              // log::debug(format!("Found doc for type: {}", type1.name));
              type1.doc = Some(doc.0);
              if let CppTypeKind::Enum { ref mut values } = type1.kind {
                let enum_namespace = if let Some(index) = type1.name.rfind("::") {
                  type1.name[0..index + 2].to_string()
                } else {
                  String::new()
                };
                for value in values {
                  if let Some(r) = doc.1.iter().find(|x| x.name == value.name) {
                    value.doc = Some(r.html.clone());

                    // let full_name = format!("{}::{}", enum_namespace, &value.name);
                    // println!("full name: {}", full_name);
                    parser.mark_enum_variant_used(&format!("{}{}", enum_namespace, &value.name));

                  } else {
                    let type_name = &type1.name;
                    log::llog(log::DebugQtDoc, || {
                      format!("Not found doc for enum variant: {}::{}",
                              type_name,
                              &value.name)
                    });
                  }
                }
              }
            }
            Err(err) => {
              log::llog(log::DebugQtDoc,
                        || format!("Not found doc for type: {}: {}", type1.name, err));
            }
          }
        }
        parser.report_unused_anchors();
      }
      Err(err) => {
        log::error(format!("Failed to get Qt documentation: {}", err));
        err.discard_expected();
      }
    }
    Ok(())
  });

  config.set_crate_template_path(crate_templates_path);
  match sublib_name {
    "core" => lib_configs::core(&mut config)?,
    "gui" => lib_configs::gui(&mut config)?,
    "widgets" => lib_configs::widgets(&mut config)?,
    "3d_core" => lib_configs::core_3d(&mut config)?,
    "3d_render" => lib_configs::render_3d(&mut config)?,
    "3d_input" => lib_configs::input_3d(&mut config)?,
    "3d_logic" => lib_configs::logic_3d(&mut config)?,
    "3d_extras" => lib_configs::extras_3d(&mut config)?,
    "ui_tools" => {}
    _ => return Err(format!("Unknown lib name: {}", sublib_name).into()),
  }

  config.set_dependency_cache_paths(dependency_paths);
  config.exec()?;
  Ok(())
}

/// Adds documentation from `data` to `cpp_methods`.
fn find_methods_docs(cpp_methods: &mut [CppMethod], data: &mut DocParser) -> Result<()> {
  for cpp_method in cpp_methods {
    if let Some(ref info) = cpp_method.class_membership {
      if info.visibility == CppVisibility::Private {
        continue;
      }
    }
    if let Some(ref declaration_code) = cpp_method.declaration_code {
      match data.doc_for_method(&cpp_method.doc_id(),
                                declaration_code,
                                &cpp_method.short_text()) {
        Ok(doc) => cpp_method.doc = Some(doc),
        Err(msg) => {
          if cpp_method.class_membership.is_some() &&
             (&cpp_method.name == "tr" || &cpp_method.name == "trUtf8" ||
              &cpp_method.name == "metaObject") {
            // no error message
          } else {
            log::llog(log::DebugQtDoc, || {
              format!("Failed to get documentation for method: {}: {}",
                      &cpp_method.short_text(),
                      msg)
            });
          }
        }
      }
    }
  }
  Ok(())
}
