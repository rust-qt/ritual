use qt_core::core_application::CoreApplication;
use qt_core::message_logger::MessageLogger;
use qt_core::string::String;

fn main() {
    CoreApplication::create_and_exit(|_app| {
        MessageLogger::new()
            .debug(())
            .op_shl0(&String::from("Hello World!"));
        CoreApplication::exec()
    })
}
