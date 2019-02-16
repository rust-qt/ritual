include_generated!();

impl ::gui_application::GuiApplication {
    pub fn create_and_exit<F: FnOnce(&mut ::gui_application::GuiApplication) -> i32>(f: F) -> ! {
        let exit_code = {
            let mut args = ::qt_core::core_application::CoreApplicationArgs::from_real();
            let mut app = unsafe { ::gui_application::GuiApplication::new(args.get()) };
            f(app.as_mut())
        };
        ::std::process::exit(exit_code)
    }
}
