use ritual::cli;
use ritual::config::{Config, CrateProperties, GlobalConfig};
use ritual::cpp_data::CppPath;
use ritual::cpp_type::{CppBuiltInNumericType, CppType};
use ritual::rust_info::{NameType, RustPathScope};
use ritual::rust_type::RustPath;
use ritual_common::cpp_build_config::{CppBuildConfigData, CppLibraryType};
use ritual_common::errors::{bail, err_msg, FancyUnwrap, Result, ResultExt};
use ritual_common::file_utils::repo_dir_path;
use ritual_common::string_utils::CaseOperations;
use ritual_common::{target, toml};
use std::env;
use std::path::PathBuf;

mod after_cpp_parser;

pub const CPP_STD_VERSION: &str = "0.1.1";
pub const STD_HEADERS_PATH_ENV_VAR_NAME: &str = "RITUAL_STD_HEADERS";

fn create_config() -> Result<Config> {
    let mut crate_properties = CrateProperties::new("cpp_std", CPP_STD_VERSION);
    let mut custom_fields = toml::value::Table::new();
    let mut package_data = toml::value::Table::new();
    package_data.insert(
        "authors".to_string(),
        toml::Value::Array(vec![toml::Value::String(
            "Pavel Strakhov <ri@idzaaus.org>".to_string(),
        )]),
    );
    package_data.insert(
        "description".to_string(),
        toml::Value::String("Bindings for C++ standard library".into()),
    );
    // TODO: doc url
    //package_data.insert("documentation".to_string(), toml::Value::String(doc_url));
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
            toml::Value::String("ffi".to_string()),
            toml::Value::String("ritual".to_string()),
        ]),
    );
    package_data.insert(
        "categories".to_string(),
        toml::Value::Array(vec![toml::Value::String(
            "external-ffi-bindings".to_string(),
        )]),
    );

    custom_fields.insert("package".to_string(), toml::Value::Table(package_data));
    crate_properties.set_custom_fields(custom_fields);

    let mut config = Config::new(crate_properties);
    config.set_cpp_lib_version("11");

    config.set_crate_template_path(repo_dir_path("ritual/src/bin/std_ritual/crate_template")?);

    /* TODO:  "array", "bitset", "deque", "forward_list", "list", "map", "queue", "set", "stack",
        "unordered_map", "unordered_set", "ios", "istream", "iostream", "fstream", "sstream",
        "atomic", "condition_variable", "future", "mutex", "thread", "algorithm", "chrono",
        "codecvt", "complex", "exception", "functional", "initializer_list", "iterator", "limits",
         "locale", "memory", "new", "numeric", "random", "ratio", "regex", "stdexcept",
         "system_error", "tuple", "typeindex", "typeinfo", "type_traits", "utility", "valarray",
    */
    let headers = ["string", "vector"];

    for header in &headers[..] {
        config.add_include_directive(header);
    }

    let include_path = PathBuf::from(
        env::var(STD_HEADERS_PATH_ENV_VAR_NAME)
            .with_context(|_| format!("missing env var: {}", STD_HEADERS_PATH_ENV_VAR_NAME))?,
    );
    if !include_path.exists() {
        bail!("std headers path doesn't exist: {}", include_path.display());
    }
    config.add_target_include_path(include_path);

    config.set_cpp_parser_path_hook(|path| {
        if path.items().iter().any(|item| item.name.starts_with('_')) {
            return Ok(false);
        }

        /*
                let string = path.to_templateless_string();
                let blocked = &[
                    // not in C++ standard
                    "__gnu_cxx",
                ];
                if blocked.contains(&string.as_str()) {
                    return Ok(false);
                }
        */

        Ok(true)
    });

    config.add_after_cpp_parser_hook(after_cpp_parser::hook);

    let namespace = CppPath::from_good_str("std");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("cpp_std"),
                prefix: None,
            }));
        }
        Ok(None)
    });

    config.set_rust_path_hook(move |path, name_type, _data| {
        if path.to_templateless_string() == "std::basic_string" {
            let args = path
                .last()
                .template_arguments
                .as_ref()
                .ok_or_else(|| err_msg("std::basic_string must have template arguments"))?;
            let arg = args
                .get(0)
                .ok_or_else(|| err_msg("std::basic_string: first argument is missing"))?;

            let rust_name = match arg {
                CppType::BuiltInNumeric(CppBuiltInNumericType::Char) => "String",
                CppType::BuiltInNumeric(CppBuiltInNumericType::Char16) => "U16String",
                CppType::BuiltInNumeric(CppBuiltInNumericType::Char32) => "U32String",
                CppType::BuiltInNumeric(CppBuiltInNumericType::WChar) => "WString",
                _ => {
                    return Ok(None);
                }
            };

            let name = match name_type {
                NameType::Type { .. } => format!("cpp_std::{}", rust_name),
                NameType::Module { .. } => format!("cpp_std::{}", rust_name.to_snake_case()),
                _ => bail!("unexpected name type for std::basic_string"),
            };
            return Ok(Some(RustPath::from_good_str(&name)));
        }
        Ok(None)
    });

    config.add_cpp_parser_argument("-std=c++11");
    let mut data = CppBuildConfigData::new();
    data.set_library_type(CppLibraryType::Static);
    config
        .cpp_build_config_mut()
        .add(target::Condition::True, data);
    Ok(config)
}

fn main() {
    let mut config = GlobalConfig::new();
    config.set_all_crate_names(vec!["cpp_std".into()]);
    config.set_create_config_hook(|_crate_name| create_config());

    cli::run_from_args(config).fancy_unwrap();
}
