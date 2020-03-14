//! Macros for Qt.
//!
//! This crate is part of the [ritual](https://github.com/rust-qt/ritual) project.

#![deny(missing_docs)]

// TODO: this will be unneeded soon.
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_hack::proc_macro_hack;

mod q_init_resource;
mod slot;
mod ui_form;

/// Generates a method that returns a slot object bound to `self`.
///
/// # Usage
///
/// This attribute may be used on methods:
///
/// ```ignore
/// impl TodoWidget {
///     #[slot(SlotNoArgs)]
///     unsafe fn on_add_clicked(self: &Rc<Self>) {
///         //...
///     }
/// }
/// ```
///
/// The type of slot wrapper (e.g. `SlotNoArgs`) must be specified as an argument to the attribute.
/// This type must be in scope.
///
/// The macro generates another method that is called `slot_{original_name}` and can be used for
/// making a connection like this:
/// ```ignore
/// self.form.add.clicked().connect(&self.slot_on_add_clicked());
/// ```
///
/// The method accepts a `&Rc<Self>` and returns a `QBox<Slot>`, where `Slot` is the slot wrapper
/// type passed to the attribute. The slot wrapper retains a weak reference to `Self`, so it
/// doesn't prevent deletion of the object.
///
/// Note that each invokation of the slot getter will create a new object.
///
/// # Requirements
/// - Target method must have `self: &Rc<Self>` argument.
/// - The rest of the arguments must correspond to arguments expected by the specified
/// slot wrapper type.
/// - `Self` must implement `StaticUpcast<QObject>`. Created slots will use the result of this
/// conversion as the parent object.
#[proc_macro_attribute]
pub fn slot(attrs: TokenStream, input: TokenStream) -> TokenStream {
    crate::slot::slot(attrs, input)
}

/// Generates code for loading an UI file.
///
/// # Usage
///
/// This attribute should be used on structs:
/// ```ignore
/// #[ui_form("../ui/form.ui")]
/// #[derive(Debug)]
/// struct Form {
///     widget: QBox<QWidget>,
///     add: QPtr<QPushButton>,
///     //...
/// }
/// ```
///
/// Specify path to the UI file as an argument of the attribute. The path must be relative to
/// the current file. Content of the UI file will be embedded into the executable.
///
/// The macro will generate the function `fn load() -> Self`.
///
/// # Requirements
///
/// - The struct must contain named fields.
/// - The first argument must have `QBox<QWidget>` type. This argument will contain the main widget.
/// - Each of the following arguments must have a name corresponding to `objectName` of an object
/// in the UI file. The type of the field must be `QPtr<T>`, where `T` must correspond to the type
/// of the object.
///
/// The `load()` function will panic if the UI file is invalid or if a name or type of any field
/// doesn't match the objects in the UI file.
#[proc_macro_attribute]
pub fn ui_form(attrs: TokenStream, input: TokenStream) -> TokenStream {
    crate::ui_form::ui_form(attrs, input)
}

/// TODO
#[proc_macro_hack]
pub fn q_init_resource(input: TokenStream) -> TokenStream {
    crate::q_init_resource::q_init_resource(input)
}
