use ::application::Application;
use ::qt_core::CoreApplicationArgs;

impl Application {
  pub fn create_and_exit<F: FnOnce(&mut Application) -> i32>(f: F) -> ! {
    let exit_code = {
      let mut args = CoreApplicationArgs::from_real();
      let mut app = Application::new(args.get());
      f(app.as_mut())
    };
    ::std::process::exit(exit_code)
  }
}
