include_generated!();

impl ::application::Application {
    pub fn create_and_exit<F: FnOnce(&mut ::application::Application) -> i32>(f: F) -> ! {
        let exit_code = {
            let mut args = ::qt_core::core_application::CoreApplicationArgs::from_real();
            let mut app = unsafe { ::application::Application::new(args.get()) };
            f(app.as_mut())
        };
        ::std::process::exit(exit_code)
    }
}
