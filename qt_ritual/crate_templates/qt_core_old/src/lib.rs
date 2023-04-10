use proc_macro_hack::proc_macro_hack;

mod connect;
mod impl_arguments_compatible;
mod impl_ptr_ops;
mod impl_q_byte_array;
mod impl_q_core_application;
mod impl_q_object;
mod impl_q_string;
mod q_box;
mod q_debug_shim;
mod q_flags;
mod q_message_logger_macros;
mod q_ptr;

pub use crate::connect::{ArgumentsCompatible, AsReceiver, Receiver, Signal};
pub use crate::impl_q_core_application::QCoreApplicationArgs;
pub use crate::impl_q_object::FindChildError;
pub use crate::impl_q_string::qs;
pub use crate::q_box::QBox;
pub use crate::q_debug_shim::{qdbg, QDebugShim};
pub use crate::q_flags::QFlags;
pub use crate::q_ptr::QPtr;

pub use qt_macros::slot;

/// Initializes Qt resources specified by the `.qrc` file with the specified base name.
///
/// This macro should be used in combination with `qt_ritual_build::add_resources`.
/// Call `add_resources` in the build script of your crate, then call the macro like this:
/// ```ignore
/// QGuiApplication::init(|_| unsafe {
///     q_init_resource!("resources");
///     //...
/// })
/// ```
/// The argument must be equal to the base name
/// ([file stem](https://doc.rust-lang.org/std/path/struct.Path.html#method.file_stem))
/// of the `.qrc` file. Special characters (such as '-')
/// have to be replaced by the underscore character (`'_'`).
///
/// This macro is semantically equivalent to the
/// [Q_INIT_RESOURCE](https://doc.qt.io/qt-5/qdir.html#Q_INIT_RESOURCE) C++ macro.
///
/// [C++ documentation](https://doc.qt.io/qt-5/qdir.html#Q_INIT_RESOURCE):
/// <div style='border: 1px solid #5CFF95; background: #D6FFE4; padding: 16px;'>
/// <p>Initializes the resources specified by the .qrc file with the specified base name.
/// Normally, when resources are built as part of the application, the resources are loaded
/// automatically at startup. The <code>Q_INIT_RESOURCE()</code> macro is necessary
/// on some platforms for resources stored in a static library.</p>
/// <p>For example, if your application's resources are listed in a file called
/// <code>myapp.qrc</code>, you can ensure that the resources are initialized at startup
/// by adding this line to your <code>main()</code> function:</p>
/// <pre>
/// Q_INIT_RESOURCE(myapp);
/// </pre>
/// </div>
#[proc_macro_hack]
pub use qt_macros::q_init_resource;
