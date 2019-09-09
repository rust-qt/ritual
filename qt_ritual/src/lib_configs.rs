//! Generator configurations specific for each Qt module.

use crate::detect_signals_and_slots::detect_signals_and_slots;
use crate::doc_parser::{parse_docs, set_crate_root_doc};
use crate::slot_wrappers::add_slot_wrappers;
use crate::versions;
use log::info;
use qt_ritual_common::{all_crate_names, get_full_build_config, lib_dependencies, lib_folder_name};
use ritual::config::CrateProperties;
use ritual::config::{Config, GlobalConfig};
use ritual_common::cpp_build_config::CppLibraryType;
use ritual_common::cpp_build_config::{CppBuildConfigData, CppBuildPaths};
use ritual_common::errors::{bail, format_err, Result, ResultExt};
use ritual_common::file_utils::repo_dir_path;
use ritual_common::target;
use ritual_common::toml;
use std::path::{Path, PathBuf};

mod _3d;
mod charts;
mod core;
mod gui;
mod widgets;

use self::_3d::{
    core_3d_config, extras_3d_config, input_3d_config, logic_3d_config, render_3d_config,
};
use self::{charts::charts_config, core::core_config, gui::gui_config, widgets::widgets_config};
use std::env;

pub const MOQT_INSTALL_DIR_ENV_VAR_NAME: &str = "MOQT_INSTALL_DIR";
pub const MOQT_TEMPLATE_DIR_ENV_VAR_NAME: &str = "MOQT_TEMPLATE_DIR";

fn empty_config(_config: &mut Config) -> Result<()> {
    Ok(())
}

/// Executes the generator for a single Qt module with given configuration.
pub fn create_config(crate_name: &str, qmake_path: Option<&str>) -> Result<Config> {
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
    let description = format!("Bindings for {} C++ library", lib_folder_name(crate_name));
    package_data.insert("description".to_string(), toml::Value::String(description));
    let doc_url = format!("https://rust-qt.github.io/rustdoc/qt/{}", &crate_name);
    package_data.insert("documentation".to_string(), toml::Value::String(doc_url));
    package_data.insert(
        "repository".to_string(),
        toml::Value::String("https://github.com/rust-qt/ritual".to_string()),
    );
    package_data.insert(
        "license".to_string(),
        toml::Value::String("MIT OR Apache-2.0".to_string()),
    );
    package_data.insert(
        "keywords".to_string(),
        toml::Value::Array(vec![
            toml::Value::String("gui".to_string()),
            toml::Value::String("ffi".to_string()),
            toml::Value::String("qt".to_string()),
            toml::Value::String("ritual".to_string()),
        ]),
    );
    package_data.insert(
        "categories".to_string(),
        toml::Value::Array(vec![
            toml::Value::String("external-ffi-bindings".to_string()),
            toml::Value::String("gui".to_string()),
        ]),
    );

    custom_fields.insert("package".to_string(), toml::Value::Table(package_data));
    crate_properties.set_custom_fields(custom_fields);
    let mut config = if crate_name.starts_with("moqt_") {
        let mut config = Config::new(crate_properties);
        let moqt_path =
            PathBuf::from(env::var(MOQT_INSTALL_DIR_ENV_VAR_NAME).with_context(|_| {
                format_err!("{} env var is missing", MOQT_INSTALL_DIR_ENV_VAR_NAME)
            })?);

        config.add_include_directive(format!("{}.h", crate_name));
        let include_path = moqt_path.join("include");
        if !include_path.exists() {
            bail!("Path does not exist: {}", include_path.display());
        }
        let lib_path = moqt_path.join("lib");
        if !lib_path.exists() {
            bail!("Path does not exist: {}", lib_path.display());
        }
        let sublib_include_path = include_path.join(crate_name);
        if !sublib_include_path.exists() {
            bail!("Path does not exist: {}", sublib_include_path.display());
        }
        {
            let mut paths = CppBuildPaths::new();
            paths.add_include_path(&sublib_include_path);
            paths.add_lib_path(&lib_path);

            for &lib in lib_dependencies(crate_name)? {
                let dep_include_path = include_path.join(lib);
                if !dep_include_path.exists() {
                    bail!("Path does not exist: {}", dep_include_path.display());
                }
                paths.add_include_path(&dep_include_path);
            }
            config.set_cpp_build_paths(paths);
        }
        config.add_target_include_path(&sublib_include_path);

        {
            let mut data = CppBuildConfigData::new();
            data.add_linked_lib(crate_name);
            for &lib in lib_dependencies(crate_name)? {
                data.add_linked_lib(lib);
            }
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

        let template_path =
            PathBuf::from(env::var(MOQT_TEMPLATE_DIR_ENV_VAR_NAME).with_context(|_| {
                format_err!("{} env var is missing", MOQT_TEMPLATE_DIR_ENV_VAR_NAME)
            })?);
        config.set_crate_template_path(template_path.join(&crate_name));

        let steps = config.processing_steps_mut();
        let crate_name_clone = crate_name.to_string();
        steps.add_after(&["cpp_parser"], "qt_doc_parser", move |data| {
            parse_docs(data, &crate_name_clone, &Path::new("."))
        })?;

        config
    } else {
        crate_properties.remove_default_build_dependencies();
        crate_properties.add_build_dependency(
            "qt_ritual_build",
            versions::QT_RITUAL_BUILD_VERSION,
            Some(repo_dir_path("qt_ritual_build")?),
        );

        let mut config = Config::new(crate_properties);

        let qt_config = get_full_build_config(crate_name, qmake_path)?;
        config.set_cpp_build_config(qt_config.cpp_build_config);
        config.set_cpp_build_paths(qt_config.cpp_build_paths);

        config.add_target_include_path(&qt_config.installation_data.lib_include_path);
        config.set_cpp_lib_version(qt_config.installation_data.qt_version.as_str());
        // TODO: does parsing work on MacOS without adding "-F"?

        config.add_include_directive(&lib_folder_name(crate_name));

        // DEBUG!
        //config.add_include_directive("QObject");
        //config.add_include_directive("QMetaObject");

        // TODO: allow to override parser flags
        config.add_cpp_parser_arguments(vec!["-fPIC", "-fcxx-exceptions"]);

        if target::current_env() == target::Env::Msvc {
            config.add_cpp_parser_argument("-std=c++14");
        } else {
            config.add_cpp_parser_argument("-std=gnu++11");
        }
        //config.add_cpp_parser_blocked_name(CppName::from_one_part("qt_check_for_QGADGET_macro"));

        let steps = config.processing_steps_mut();
        let crate_name_clone = crate_name.to_string();
        let docs_path = qt_config.installation_data.docs_path.clone();

        steps.add_after(&["cpp_parser"], "qt_doc_parser", move |data| {
            parse_docs(data, &crate_name_clone, &docs_path)
        })?;

        config
            .set_crate_template_path(repo_dir_path("qt_ritual/crate_templates")?.join(&crate_name));

        config
    };

    config.add_after_cpp_parser_hook(detect_signals_and_slots);

    let steps = config.processing_steps_mut();
    for cpp_parser_stage in &["cpp_parser", "cpp_parser_stage2"] {
        steps.add_after(&[cpp_parser_stage], "add_slot_wrappers", add_slot_wrappers)?;
    }

    steps.add_after(
        &["rust_generator"],
        "set_crate_root_doc",
        set_crate_root_doc,
    )?;

    let lib_config = match crate_name {
        "qt_core" => core_config,
        "qt_gui" => gui_config,
        "qt_widgets" => widgets_config,
        "qt_3d_core" => core_3d_config,
        "qt_3d_render" => render_3d_config,
        "qt_3d_input" => input_3d_config,
        "qt_3d_logic" => logic_3d_config,
        "qt_3d_extras" => extras_3d_config,
        "qt_ui_tools" => empty_config,
        "qt_charts" => charts_config,
        "qt_qml" => empty_config,
        "moqt_core" => core_config,
        "moqt_gui" => gui_config,
        _ => bail!("Unknown crate name: {}", crate_name),
    };
    lib_config(&mut config)?;

    config.set_dependent_cpp_crates(
        lib_dependencies(crate_name)?
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    Ok(config)
}

pub fn global_config() -> GlobalConfig {
    let mut config = GlobalConfig::new();
    config.set_all_crate_names(all_crate_names().iter().map(|s| s.to_string()).collect());
    config.set_create_config_hook(|crate_name| create_config(crate_name, None));
    config
}
