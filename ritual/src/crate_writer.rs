use crate::cpp_code_generator;
use crate::cpp_code_generator::generate_cpp_type_size_requester;
use crate::processor::ProcessingStep;
use crate::processor::ProcessorData;
use crate::rust_code_generator;
use crate::versions;
use pathdiff::diff_paths;
use ritual_common::errors::{err_msg, Result};
use ritual_common::file_utils::copy_file;
use ritual_common::file_utils::copy_recursively;
use ritual_common::file_utils::create_dir;
use ritual_common::file_utils::create_dir_all;
use ritual_common::file_utils::create_file;
use ritual_common::file_utils::path_to_str;
use ritual_common::file_utils::read_dir;
use ritual_common::file_utils::remove_dir_all;
use ritual_common::file_utils::repo_crate_local_path;
use ritual_common::file_utils::save_json;
use ritual_common::file_utils::save_toml;
use ritual_common::toml;
use ritual_common::utils::run_command;
use ritual_common::utils::MapIfOk;
use ritual_common::BuildScriptData;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Merges `a` and `b` recursively. `b` take precedence over `a`.
fn recursive_merge_toml(a: toml::Value, b: toml::Value) -> toml::Value {
    if a.same_type(&b) {
        if let toml::Value::Array(mut a_array) = a {
            if let toml::Value::Array(mut b_array) = b {
                a_array.append(&mut b_array);
                toml::Value::Array(a_array)
            } else {
                unreachable!()
            }
        } else if let toml::Value::Table(mut a_table) = a {
            if let toml::Value::Table(b_table) = b {
                for (key, value) in b_table {
                    if let Some(old_value) = a_table.remove(&key) {
                        a_table.insert(key, recursive_merge_toml(old_value, value));
                    } else {
                        a_table.insert(key, value);
                    }
                }
                toml::Value::Table(a_table)
            } else {
                unreachable!()
            }
        } else {
            b
        }
    } else {
        b
    }
}

/// Generates `Cargo.toml` file and skeleton of the crate.
/// If a crate template was supplied, files from it are
/// copied to the output location.
fn generate_crate_template(data: &mut ProcessorData) -> Result<()> {
    let output_path = data
        .workspace
        .crate_path(data.config.crate_properties().name())?;

    let template_build_rs_path =
        data.config
            .crate_template_path()
            .as_ref()
            .and_then(|crate_template_path| {
                let template_build_rs_path = crate_template_path.join("build.rs");
                if template_build_rs_path.exists() {
                    Some(template_build_rs_path)
                } else {
                    None
                }
            });
    let output_build_rs_path = output_path.join("build.rs");
    if let Some(ref template_build_rs_path) = template_build_rs_path {
        copy_file(template_build_rs_path, output_build_rs_path)?;
    } else {
        let mut build_rs_file = create_file(&output_build_rs_path)?;
        write!(
            build_rs_file,
            "{}",
            include_str!("../templates/crate/build.rs")
        )?;
    }
    let cargo_toml_data = {
        let package = toml::value::Value::Table({
            let mut table = toml::value::Table::new();
            table.insert(
                "name".to_string(),
                toml::Value::String(data.config.crate_properties().name().into()),
            );
            table.insert(
                "version".to_string(),
                toml::Value::String(data.config.crate_properties().version().into()),
            );
            table.insert(
                "build".to_string(),
                toml::Value::String("build.rs".to_string()),
            );
            table.insert(
                "edition".to_string(),
                toml::Value::String("2018".to_string()),
            );
            table
        });
        let dep_value = |version: &str, local_path: Option<PathBuf>| -> Result<toml::Value> {
            Ok(
                if local_path.is_none() || !data.workspace.config().write_dependencies_local_paths {
                    toml::Value::String(version.to_string())
                } else {
                    toml::Value::Table({
                        let mut value = toml::value::Table::new();
                        value.insert(
                            "version".to_string(),
                            toml::Value::String(version.to_string()),
                        );
                        value.insert(
                            "path".to_string(),
                            toml::Value::String(
                                path_to_str(&local_path.expect("checked above"))?.to_string(),
                            ),
                        );
                        value
                    })
                },
            )
        };
        let dependencies = toml::Value::Table({
            let mut table = toml::value::Table::new();
            if !data
                .config
                .crate_properties()
                .should_remove_default_dependencies()
            {
                table.insert(
                    "cpp_utils".to_string(),
                    dep_value(
                        versions::CPP_UTILS_VERSION,
                        if data.workspace.config().write_dependencies_local_paths {
                            Some(repo_crate_local_path("cpp_utils")?)
                        } else {
                            None
                        },
                    )?,
                );
                for dep in data.dep_databases {
                    let relative_path =
                        diff_paths(&data.workspace.crate_path(&dep.crate_name)?, &output_path)
                            .ok_or_else(|| {
                                err_msg("failed to get relative path to the dependency")
                            })?;

                    table.insert(
                        dep.crate_name.clone(),
                        dep_value(&dep.crate_version, Some(relative_path))?,
                    );
                }
            }
            for dep in data.config.crate_properties().dependencies() {
                table.insert(
                    dep.name().to_string(),
                    dep_value(dep.version(), dep.local_path().cloned())?,
                );
            }
            table
        });
        let build_dependencies = toml::Value::Table({
            let mut table = toml::value::Table::new();
            if !data
                .config
                .crate_properties()
                .should_remove_default_build_dependencies()
            {
                table.insert(
                    "ritual_build".to_string(),
                    dep_value(
                        versions::RITUAL_BUILD_VERSION,
                        if data.workspace.config().write_dependencies_local_paths {
                            Some(repo_crate_local_path("ritual_build")?)
                        } else {
                            None
                        },
                    )?,
                );
            }
            for dep in data.config.crate_properties().build_dependencies() {
                table.insert(
                    dep.name().to_string(),
                    dep_value(dep.version(), dep.local_path().cloned())?,
                );
            }
            table
        });
        let mut table = toml::value::Table::new();
        table.insert("package".to_string(), package);
        table.insert("dependencies".to_string(), dependencies);
        table.insert("build-dependencies".to_string(), build_dependencies);
        recursive_merge_toml(
            toml::Value::Table(table),
            toml::Value::Table(data.config.crate_properties().custom_fields().clone()),
        )
    };
    save_toml(output_path.join("Cargo.toml"), &cargo_toml_data)?;

    if let Some(ref template_path) = data.config.crate_template_path() {
        for item in read_dir(template_path)? {
            let item = item?;
            let target = output_path.join(item.file_name());
            if target.exists() {
                remove_dir_all(&target)?;
            }
            copy_recursively(&item.path(), &target)?;
        }
    }
    if !output_path.join("src").exists() {
        create_dir_all(output_path.join("src"))?;
    }
    Ok(())
}

/// Generates main files and directories of the library.
fn generate_c_lib_template(
    lib_name: &str,
    lib_path: &Path,
    global_header_name: &str,
    include_directives: &[PathBuf],
) -> Result<()> {
    let name_upper = lib_name.to_uppercase();
    let cmakelists_path = lib_path.join("CMakeLists.txt");
    let mut cmakelists_file = create_file(&cmakelists_path)?;

    write!(
        cmakelists_file,
        include_str!("../templates/c_lib/CMakeLists.txt"),
        lib_name_lowercase = lib_name,
        lib_name_uppercase = name_upper
    )?;

    let include_directives_code = include_directives
        .map_if_ok(|d| -> Result<_> { Ok(format!("#include \"{}\"", path_to_str(d)?)) })?
        .join("\n");

    let global_header_path = lib_path.join(&global_header_name);
    let mut global_header_file = create_file(&global_header_path)?;
    write!(
        global_header_file,
        include_str!("../templates/c_lib/global.h"),
        include_directives_code = include_directives_code
    )?;
    Ok(())
}

fn run(data: &mut ProcessorData) -> Result<()> {
    generate_crate_template(data)?;
    data.workspace.update_cargo_toml()?;

    let output_path = data
        .workspace
        .crate_path(data.config.crate_properties().name())?;

    let c_lib_path = output_path.join("c_lib");
    if !c_lib_path.exists() {
        create_dir(&c_lib_path)?;
    }
    let c_lib_name = format!("{}_c", data.config.crate_properties().name());
    let global_header_name = format!("{}_global.h", c_lib_name);
    generate_c_lib_template(
        &c_lib_name,
        &c_lib_path,
        &global_header_name,
        data.config.include_directives(),
    )?;

    cpp_code_generator::generate_cpp_file(
        &data.current_database.cpp_items,
        &c_lib_path.join("file1.cpp"),
        &global_header_name,
    )?;

    let file = create_file(c_lib_path.join("sized_types.cxx"))?;
    generate_cpp_type_size_requester(
        &data.current_database.rust_database,
        data.config.include_directives(),
        file,
    )?;

    let rust_src_path = output_path.join("src");
    if rust_src_path.exists() {
        remove_dir_all(&rust_src_path)?;
    }
    create_dir_all(&rust_src_path)?;
    rust_code_generator::generate(
        data.config.crate_properties().name(),
        &data.current_database.rust_database,
        &rust_src_path,
        data.config.crate_template_path().map(|s| s.join("src")),
    )?;

    run_command(Command::new("cargo").arg("fmt").current_dir(&output_path))?;
    run_command(
        Command::new("rustfmt")
            .arg("src/ffi.in.rs")
            .current_dir(&output_path),
    )?;

    save_json(
        output_path.join("build_script_data.json"),
        &BuildScriptData {
            cpp_build_config: data.config.cpp_build_config().clone(),
            cpp_wrapper_lib_name: c_lib_name,
            cpp_lib_version: data.config.cpp_lib_version().map(|s| s.to_string()),
        },
    )?;
    Ok(())
}

pub fn crate_writer_step() -> ProcessingStep {
    // TODO: set dependencies
    ProcessingStep::new_const("crate_writer", run)
}
