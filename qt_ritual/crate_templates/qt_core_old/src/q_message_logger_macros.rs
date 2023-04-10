/// Creates a `QMessageLogger` with attached information about current file, line, and module.
#[macro_export]
macro_rules! q_message_logger {
    () => {
        $crate::QMessageLogger::new_3a(
            ::std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes())
                .as_ptr(),
            line!() as i32,
            ::std::ffi::CStr::from_bytes_with_nul_unchecked(
                concat!(module_path!(), "\0").as_bytes(),
            )
            .as_ptr(),
        )
    };
}

/// Creates a `QDebug` that logs a debug message
/// with attached information about current file, line, and module.
///
/// This is similar to `qDebug()` C++ macro.
#[macro_export]
macro_rules! q_debug {
    () => {
        &$crate::q_message_logger!().debug_0a()
    };
}

/// Creates a `QDebug` that logs an informational message
/// with attached information about current file, line, and module.
///
/// This is similar to `qInfo()` C++ macro.
#[macro_export]
macro_rules! q_info {
    () => {
        &$crate::q_message_logger!().info_0a()
    };
}

/// Creates a `QDebug` that logs a warning message
/// with attached information about current file, line, and module.
///
/// This is similar to `qWarning()` C++ macro.
#[macro_export]
macro_rules! q_warning {
    () => {
        &$crate::q_message_logger!().warning_0a()
    };
}

/// Creates a `QDebug` that logs a critical message
/// with attached information about current file, line, and module.
///
/// This is similar to `qCritical()` C++ macro.
#[macro_export]
macro_rules! q_critical {
    () => {
        &$crate::q_message_logger!().critical_0a()
    };
}
