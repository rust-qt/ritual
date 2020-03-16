use ritual::config::{Config, CrateDependencyKind, CrateDependencySource};
use ritual_common::errors::Result;
use ritual_common::file_utils::repo_dir_path;

/// QtUiTools specific configuration.
pub fn ui_tools_config(config: &mut Config) -> Result<()> {
    config.crate_properties_mut().add_dependency(
        "qt_macros",
        CrateDependencyKind::Normal,
        CrateDependencySource::Local {
            path: repo_dir_path("qt_macros")?,
        },
    )?;
    Ok(())
}
