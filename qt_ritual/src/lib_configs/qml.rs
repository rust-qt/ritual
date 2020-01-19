use ritual::config::Config;
use ritual_common::errors::Result;

pub fn qml_config(config: &mut Config) -> Result<()> {
    config.set_cpp_parser_path_hook(|path| {
        let string = path.to_templateless_string();
        let blocked = &[
            // Internal, undocumented.
            "QQmlPrivate",
            "qmlRegisterBaseTypes",
            "QJSEngine::handle",
        ];
        if blocked.contains(&string.as_str()) {
            return Ok(false);
        }

        Ok(true)
    });

    Ok(())
}
