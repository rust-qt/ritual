pub mod flags;
pub mod connections;
mod extra_impls;

include!(concat!(env!("OUT_DIR"), "/lib.in.rs"));

//TODO: move to core_application mod

pub struct CoreApplicationArgs {
  _values: Vec<Vec<u8>>,
  argc: Box<::libc::c_int>,
  argv: Vec<*mut ::libc::c_char>,
}
impl CoreApplicationArgs {
  pub fn from(mut args: Vec<Vec<u8>>) -> CoreApplicationArgs {
    for arg in &mut args {
      if !arg.ends_with(&[0]) {
        arg.push(0);
      }
    }
    CoreApplicationArgs {
      argc: Box::new(args.len() as ::libc::c_int),
      argv: args.iter_mut().map(|x| x.as_mut_ptr() as *mut ::libc::c_char).collect(),
      _values: args,
    }
  }
  pub fn empty() -> CoreApplicationArgs {
    CoreApplicationArgs::from(Vec::new())
  }
  pub fn get(&mut self) -> (&mut ::libc::c_int, *mut *mut ::libc::c_char, cpp_utils::AsBox) {
    (self.argc.as_mut(), self.argv.as_mut_ptr(), cpp_utils::AsBox)
  }

  #[cfg(unix)]
  pub fn from_real() -> CoreApplicationArgs {
    use std::os::unix::ffi::OsStringExt;
    let args = std::env::args_os().map(|arg| arg.into_vec()).collect();
    CoreApplicationArgs::from(args)
  }
  #[cfg(windows)]
  pub fn from_real() -> CoreApplicationArgs {
    // Qt doesn't use argc and argv on Windows anyway
    // TODO: check this
    CoreApplicationArgs::empty()
  }
}

impl ::core_application::CoreApplication {
  pub fn create_and_exit<F: FnOnce(&mut ::core_application::CoreApplication) -> i32>(f: F) -> ! {
    let exit_code = {
      let mut args = CoreApplicationArgs::from_real();
      let mut app = ::core_application::CoreApplication::new(args.get());
      f(app.as_mut())
    };
    std::process::exit(exit_code)
  }
}


// TODO: the same for QGuiApplication

//pub trait ClosureAsSlot {
//  type Slot;
//  fn as_slot(self) -> Self::Slot;
//}
//
//pub fn slot<C: ClosureAsSlot>(c: C) -> C::Slot {
//  c.as_slot()
//}

//impl<'a, T: FnMut(&::variant::Variant) + 'a> ClosureAsSlot for T {
//  type Slot = SlotVariant<'a>;
//  fn as_slot(self) -> SlotVariant<'a> {
//    SlotVariant::new(self)
//  }
//}
