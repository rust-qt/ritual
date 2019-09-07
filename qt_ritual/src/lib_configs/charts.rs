use ritual::config::Config;
use ritual::cpp_data::CppPath;
use ritual::rust_info::RustPathScope;
use ritual::rust_type::RustPath;
use ritual_common::errors::Result;

/// QtCharts specific configuration.
pub fn charts_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("QtCharts");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("qt_charts"),
                prefix: None,
            }));
        }
        Ok(None)
    });
    Ok(())
}
