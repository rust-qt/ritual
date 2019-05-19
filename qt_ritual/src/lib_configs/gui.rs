use ritual::config::Config;
use ritual_common::errors::Result;

/// QtGui specific configuration.
#[allow(clippy::collapsible_if)]
pub fn gui_config(config: &mut Config) -> Result<()> {
    config.set_cpp_parser_path_hook(|path| {
        let string = path.to_templateless_string();
        let blocked = &[
            "QAbstractOpenGLFunctionsPrivate",
            "QOpenGLFunctionsPrivate",
            "QOpenGLExtraFunctionsPrivate",
            "QBrushData",
            "QAccessible::ActivationObserver",
            "QAccessibleImageInterface",
            "QAccessibleBridge",
            "QAccessibleBridgePlugin",
            "QAccessibleApplication",
            "QOpenGLVersionStatus",
            "QOpenGLVersionFunctionsBackend",
            "QOpenGLVersionFunctionsStorage",
            "QOpenGLTexture::TextureFormatClass",
            "QTextFrameLayoutData",
        ];
        if blocked.contains(&string.as_str()) {
            return Ok(false);
        }
        if string.starts_with("QOpenGLFunctions_") {
            if string.ends_with("_CoreBackend") | string.ends_with("_DeprecatedBackend") {
                return Ok(false);
            }
        }
        Ok(true)
    });
    Ok(())
}
