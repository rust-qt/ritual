use cpp_to_rust_common::errors::{Result};
use cpp_to_rust_common::log;
use cpp_to_rust_common::utils::is_msvc;
use cpp_to_rust_common::file_utils::PathBufWithAdded;
use cpp_to_rust_generator::config::Config;
use cpp_to_rust_common::cpp_build_config::CppBuildConfigData;
use cpp_to_rust_common::target;
use qt_generator_common::qmake_query::{get_installation_data, real_lib_name, lib_folder_name};
use std::path::PathBuf;


use doc_parser::DocParser;
use fix_header_names::fix_header_names;
use cpp_to_rust_generator::cpp_method::CppMethod;
use cpp_to_rust_generator::cpp_data::CppTypeKind;
use cpp_to_rust_generator::config::{CrateProperties, is_completed};
use doc_decoder::decode_doc;
use lib_configs;

pub fn exec_all(libs: Vec<String>, cache_dir: PathBuf, output_dir: PathBuf) -> Result<()> {
  let crate_templates_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).with_added("crate_templates");
  for sublib_name in libs {
    let lib_cache_dir = cache_dir.with_added(&sublib_name);
    let lib_crate_templates_path = crate_templates_path.with_added(&sublib_name);
    let lib_output_dir = output_dir.with_added(format!("qt_{}", sublib_name));


    let dependency_names = match sublib_name.as_str() {
      "core" => vec![],
      "gui" => vec!["core"],
      "widgets" => vec!["core", "gui"],
      _ => return Err(format!("Unknown lib name: {}", sublib_name).into()),
    };
    let mut dependency_paths = Vec::new();
    for dep in dependency_names {
      let path = cache_dir.with_added(dep);
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
    exec(&sublib_name, lib_cache_dir, lib_output_dir, lib_crate_templates_path, dependency_paths)?;
  }
  Ok(())
}

fn exec(sublib_name: &str,
        cache_dir: PathBuf,
        output_dir: PathBuf,
        crate_templates_path: PathBuf,
        dependency_paths: Vec<PathBuf>) -> Result<()> {
  if is_completed(&cache_dir) {
    return Ok(());
  }
  log::info(format!("Processing library: {}", sublib_name));
  let qt_lib_name = real_lib_name(sublib_name);
  let mut crate_properties = CrateProperties::new(format!("qt_{}", sublib_name), "0.1.5");
  crate_properties.add_author("Pavel Strakhov <ri@idzaaus.org>");
  crate_properties.set_links_attribute(qt_lib_name.clone());
  crate_properties.remove_default_build_dependencies();
  crate_properties.add_build_dependency("qt_build_tools", "0.1");
  let mut config = Config::new(&output_dir, &cache_dir, crate_properties);
  let installation_data = get_installation_data(sublib_name)?;
  config.add_include_path(&installation_data.root_include_path);
  config.add_include_path(&installation_data.lib_include_path);
  config.add_target_include_path(&installation_data.lib_include_path);
  // TODO: does parsing work on MacOS without adding "-F"?

  config.add_include_directive(&lib_folder_name(sublib_name));
  config.add_cpp_data_filter(Box::new(move |cpp_data| {
    fix_header_names(cpp_data, &installation_data.lib_include_path)
  }));
  config.add_cpp_parser_flags(vec!["-fPIC", "-fcxx-exceptions"]);
  {
    let mut data = CppBuildConfigData::new();
    data.add_compiler_flags(vec!["-fPIC", "-std=gnu++11"]);
    config.cpp_build_config_mut().add(target::Condition::Env(target::Env::Msvc).negate(), data);
  }

  if is_msvc() {
    config.add_cpp_parser_flag("-std=c++14");
  } else {
    config.add_cpp_parser_flag("-std=gnu++11");
  }
  config.add_cpp_parser_blocked_name("qt_check_for_QGADGET_macro");
  let sublib_name_clone = sublib_name.to_string();
  config.add_cpp_data_filter(Box::new(move |cpp_data| {
    match decode_doc(&sublib_name_clone) {
      Ok(doc_data) => {
        let mut parser = DocParser::new(doc_data);
        find_methods_docs(&mut cpp_data.methods, &mut parser)?;
        for type1 in &mut cpp_data.types {
          match parser.doc_for_type(&type1.name) {
            Ok(doc) => {
              log::info(format!("Found doc for type: {}", type1.name));
              type1.doc = Some(doc.0);
              if let CppTypeKind::Enum { ref mut values } = type1.kind {
                for value in values {
                  if let Some(r) = doc.1.iter().find(|x| x.name == value.name) {
                    value.doc = Some(r.html.clone());
                  } else {
                    log::warning(format!("Not found doc for enum variant: {}::{}",
                                         type1.name,
                                         value.name));
                  }
                }
              }
            }
            Err(err) => {
              log::warning(format!("Not found doc for type: {}: {}", type1.name, err));
            }
          }
        }
      }
      Err(err) => {
        log::warning(format!("Failed to get Qt documentation: {}", err));
        err.discard_expected();
      }
    }
    Ok(())
  }));


  config.set_crate_template_path(crate_templates_path);
  match sublib_name {
    "core" => lib_configs::core(&mut config)?,
    "gui" => lib_configs::gui(&mut config)?,
    "widgets" => lib_configs::widgets(&mut config)?,
    _ => return Err(format!("Unknown lib name: {}", sublib_name).into()),
  }

  config.set_dependency_paths(dependency_paths);
  config.exec()?;
  Ok(())
}

fn find_methods_docs(cpp_methods: &mut [CppMethod], data: &mut DocParser) -> Result<()> {
  for cpp_method in cpp_methods {
    if let Some(ref declaration_code) = cpp_method.declaration_code {
      match data.doc_for_method(&cpp_method.doc_id(),
                                declaration_code,
                                &cpp_method.short_text()) {
        Ok(doc) => cpp_method.doc = Some(doc),
        Err(msg) => {
          log::warning(format!("Failed to get documentation for method: {}: {}",
                               &cpp_method.short_text(),
                               msg));
        }
      }
    }
  }
  Ok(())
}

