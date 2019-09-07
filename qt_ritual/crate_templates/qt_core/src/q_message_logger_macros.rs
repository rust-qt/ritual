#[macro_export]
macro_rules! q_message_logger {
    () => {
        $crate::QMessageLogger::new_3a(
            ::std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes()),
            line!() as i32,
            ::std::ffi::CStr::from_bytes_with_nul_unchecked(
                concat!(module_path!(), "\0").as_bytes(),
            ),
        )
    };
}

#[macro_export]
macro_rules! q_debug {
    () => {
        &$crate::q_message_logger!().debug_0a()
    };
}

#[macro_export]
macro_rules! q_info {
    () => {
        &$crate::q_message_logger!().info_0a()
    };
}

#[macro_export]
macro_rules! q_warning {
    () => {
        &$crate::q_message_logger!().warning_0a()
    };
}

#[macro_export]
macro_rules! q_critical {
    () => {
        &$crate::q_message_logger!().critical_0a()
    };
}
