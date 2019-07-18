use ritual::config::Config;
use ritual_common::errors::Result;

/// QtWidgets specific configuration.
pub fn widgets_config(config: &mut Config) -> Result<()> {
    config.set_cpp_parser_path_hook(|path| {
        let string = path.to_templateless_string();
        let blocked = &[
            //internal
            "QWidgetData",
            "QWidgetItemV2",
            "QWidgetItemData",
        ];
        if blocked.contains(&string.as_str()) {
            return Ok(false);
        }
        Ok(true)
    });

    Ok(())
}
