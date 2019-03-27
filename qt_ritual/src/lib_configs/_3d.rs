use ritual::config::Config;
use ritual::cpp_data::CppPath;
use ritual::rust_info::RustPathScope;
use ritual::rust_type::RustPath;
use ritual_common::errors::Result;

/// Qt3DCore specific configuration.
pub fn core_3d_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("Qt3DCore");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("qt_3d_core"),
                prefix: None,
            }));
        }
        Ok(None)
    });
    Ok(())
}

/// Qt3DRender specific configuration.
pub fn render_3d_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("Qt3DRender");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("qt_3d_render"),
                prefix: None,
            }));
        }
        Ok(None)
    });
    // TODO: hopefully we don't need to block these anymore
    /*
    config.add_cpp_parser_blocked_names(vec![
      "Qt3DRender::QTexture1D",
      "Qt3DRender::QTexture1DArray",
      "Qt3DRender::QTexture2D",
      "Qt3DRender::QTexture2DArray",
      "Qt3DRender::QTexture3D",
      "Qt3DRender::QTextureCubeMap",
      "Qt3DRender::QTextureCubeMapArray",
      "Qt3DRender::QTexture2DMultisample",
      "Qt3DRender::QTexture2DMultisampleArray",
      "Qt3DRender::QTextureRectangle",
      "Qt3DRender::QTextureBuffer",
      "Qt3DRender::QRenderCapture",
      "Qt3DRender::QRenderCaptureReply",
      "Qt3DRender::QSortCriterion",
      "Qt3DRender::QSpotLight::attenuation",
    ]);*/
    Ok(())
}

/// Qt3DInput specific configuration.
pub fn input_3d_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("Qt3DInput");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("qt_3d_input"),
                prefix: None,
            }));
        }
        Ok(None)
    });
    // TODO: hopefully we don't need to block these anymore
    //config.add_cpp_parser_blocked_names(vec!["Qt3DInput::QWheelEvent"]);
    Ok(())
}

/// Qt3DLogic specific configuration.
pub fn logic_3d_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("Qt3DLogic");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("qt_3d_logic"),
                prefix: None,
            }));
        }
        Ok(None)
    });
    Ok(())
}

/// Qt3DExtras specific configuration.
pub fn extras_3d_config(config: &mut Config) -> Result<()> {
    let namespace = CppPath::from_good_str("Qt3DExtras");
    config.set_rust_path_scope_hook(move |path| {
        if path == &namespace {
            return Ok(Some(RustPathScope {
                path: RustPath::from_good_str("qt_3d_extras"),
                prefix: None,
            }));
        }
        Ok(None)
    });
    Ok(())
}
