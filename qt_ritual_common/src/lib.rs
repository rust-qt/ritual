//! Common utilities for the generator and the build script for Qt crates.
//!
//!
//! See [README](https://github.com/rust-qt/cpp_to_rust)
//! for more information.

use log::debug;
use ritual_common::cpp_build_config::{
    CppBuildConfig, CppBuildConfigData, CppBuildPaths, CppLibraryType,
};
use ritual_common::cpp_lib_builder::CMakeVar;
use ritual_common::errors::{bail, Result};
use ritual_common::string_utils::CaseOperations;
use ritual_common::target;
use ritual_common::utils::get_command_output;
use std::path::PathBuf;
use std::process::Command;

/// Makes a query to `qmake`.
fn run_qmake_string_query(property: &str, qmake_path: Option<&str>) -> Result<String> {
    let command = qmake_path.unwrap_or("qmake");
    let result = get_command_output(Command::new(command).arg("-query").arg(property))?;
    Ok(result.trim().to_string())
}

/// Makes a query to `qmake` and interprets its output as a path.
fn run_qmake_query(property: &str, qmake_path: Option<&str>) -> Result<PathBuf> {
    Ok(PathBuf::from(run_qmake_string_query(property, qmake_path)?))
}

/// Properties of a Qt installation
pub struct InstallationData {
    /// Qt version.
    pub qt_version: String,
    /// Path to the parent include directory of the installation.
    pub root_include_path: PathBuf,
    /// Path to the include directory of the library that is being processed.
    /// This is a direct subdirectory of `root_include_path`.
    pub lib_include_path: PathBuf,
    /// Path to the directory containing library files for the linker.
    pub lib_path: PathBuf,
    /// Path to the directory containing Qt documentation files.
    pub docs_path: PathBuf,
    /// If true, this Qt library was built as a MacOS framework.
    pub is_framework: bool,
}

/// Detects properties of current Qt installation using `qmake` command line utility.
pub fn get_installation_data(
    crate_name: &str,
    qmake_path: Option<&str>,
) -> Result<InstallationData> {
    let qt_version = run_qmake_string_query("QT_VERSION", qmake_path)?;
    debug!("QT_VERSION = \"{}\"", qt_version);

    let qt_version_parsed = semver::Version::parse(&qt_version)?;
    if qt_version_parsed.major != 5 {
        bail!(
            "only Qt 5 is supported! \"{}\" is not supported",
            qt_version
        );
    }

    debug!("Detecting Qt directories");

    let root_include_path = run_qmake_query("QT_INSTALL_HEADERS", qmake_path)?;
    debug!("QT_INSTALL_HEADERS = \"{}\"", root_include_path.display());
    let lib_path = run_qmake_query("QT_INSTALL_LIBS", qmake_path)?;
    debug!("QT_INSTALL_LIBS = \"{}\"", lib_path.display());
    let docs_path = run_qmake_query("QT_INSTALL_DOCS", qmake_path)?;
    debug!("QT_INSTALL_DOCS = \"{}\"", docs_path.display());
    let folder_name = lib_folder_name(crate_name);
    let dir = root_include_path.join(&folder_name);
    if dir.exists() {
        Ok(InstallationData {
            root_include_path,
            lib_path,
            docs_path,
            lib_include_path: dir,
            is_framework: false,
            qt_version,
        })
    } else {
        let dir2 = lib_path.join(format!("{}.framework/Headers", folder_name));
        if dir2.exists() {
            Ok(InstallationData {
                root_include_path,
                lib_path,
                docs_path,
                lib_include_path: dir2,
                is_framework: true,
                qt_version,
            })
        } else {
            bail!(
                "extra header dir not found (tried: {}, {})",
                dir.display(),
                dir2.display()
            );
        }
    }
}

pub struct FullBuildConfig {
    pub installation_data: InstallationData,
    pub cpp_build_config: CppBuildConfig,
    pub cpp_build_paths: CppBuildPaths,
}

pub fn get_full_build_config(
    crate_name: &str,
    qmake_path: Option<&str>,
) -> Result<FullBuildConfig> {
    let installation_data = get_installation_data(crate_name, qmake_path)?;
    let mut cpp_build_paths = CppBuildPaths::new();
    let mut cpp_build_config_data = CppBuildConfigData::new();

    let mut apply_installation_data = |name: &str, data: &InstallationData| {
        cpp_build_paths.add_include_path(&data.root_include_path);
        cpp_build_paths.add_include_path(&data.lib_include_path);
        if data.is_framework {
            cpp_build_paths.add_framework_path(&data.lib_path);
            cpp_build_config_data.add_linked_framework(framework_name(name));
        } else {
            cpp_build_paths.add_lib_path(&data.lib_path);
            cpp_build_config_data.add_linked_lib(real_lib_name(name));
        }
        cpp_build_config_data.add_cmake_var(CMakeVar::new("RITUAL_QT", "1"));
    };

    apply_installation_data(crate_name, &installation_data);
    for dep in lib_dependencies(crate_name)? {
        let dep_data = get_installation_data(dep, qmake_path)?;
        apply_installation_data(dep, &dep_data);
    }

    let mut cpp_build_config = CppBuildConfig::new();
    cpp_build_config.add(target::Condition::True, cpp_build_config_data);
    {
        let mut data = CppBuildConfigData::new();
        data.add_compiler_flag("-std=gnu++11");
        cpp_build_config.add(target::Condition::Env(target::Env::Msvc).negate(), data);
    }
    {
        let mut data = CppBuildConfigData::new();
        data.add_compiler_flag("-fPIC");
        // msvc and mingw don't need this
        cpp_build_config.add(target::Condition::OS(target::OS::Windows).negate(), data);
    }
    {
        let mut data = CppBuildConfigData::new();
        data.set_library_type(CppLibraryType::Shared);
        cpp_build_config.add(target::Condition::Env(target::Env::Msvc), data);
    }
    {
        let mut data = CppBuildConfigData::new();
        data.set_library_type(CppLibraryType::Static);
        cpp_build_config.add(target::Condition::Env(target::Env::Msvc).negate(), data);
    }
    Ok(FullBuildConfig {
        installation_data,
        cpp_build_config,
        cpp_build_paths,
    })
}

/// Returns library name of the specified module as
/// should be passed to the linker, e.g. `"Qt5Core"`.
pub fn real_lib_name(crate_name: &str) -> String {
    let sublib_name = crate_name.replace("qt_", "");
    let sublib_name_capitalized = sublib_name.to_class_case();
    format!("Qt5{}", sublib_name_capitalized)
}

/// Returns name of the module's include directory, e.g. `"QtCore"`.
pub fn lib_folder_name(crate_name: &str) -> String {
    let sublib_name = crate_name.replace("qt_", "");
    let sublib_name_capitalized = sublib_name.to_class_case();
    format!("Qt{}", sublib_name_capitalized)
}

/// Returns MacOS framework name of the specified module as
/// should be passed to the linker, e.g. `"QtCore"`.
pub fn framework_name(crate_name: &str) -> String {
    let sublib_name = crate_name.replace("qt_", "");
    let sublib_name_capitalized = sublib_name.to_class_case();
    format!("Qt{}", sublib_name_capitalized)
}

pub fn all_crate_names() -> &'static [&'static str] {
    &[
        "qt_core",
        "qt_gui",
        "qt_widgets",
        "qt_ui_tools",
        "qt_3d_core",
        "qt_3d_render",
        "qt_3d_input",
        "qt_3d_logic",
        "qt_3d_extras",
    ]
}

/// Returns list of modules this module depends on.
pub fn lib_dependencies(crate_name: &str) -> Result<&'static [&'static str]> {
    Ok(match crate_name {
        "qt_core" => &[],
        "qt_gui" => &["qt_core"],
        "qt_widgets" => &["qt_core", "qt_gui"],
        "qt_3d_core" => &["qt_core", "qt_gui"],
        "qt_3d_render" | "qt_3d_input" | "qt_3d_logic" => &["qt_core", "qt_gui", "qt_3d_core"],
        "qt_3d_extras" => &[
            "qt_core",
            "qt_gui",
            "qt_3d_core",
            "qt_3d_render",
            "qt_3d_input",
            "qt_3d_logic",
        ],
        "qt_ui_tools" => &["qt_core", "qt_gui", "qt_widgets"],
        "moqt_core" => &[],
        "moqt_gui" => &["moqt_core"],
        _ => bail!("Unknown crate name: {}", crate_name),
    })
}
