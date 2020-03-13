extern crate proc_macro;

use proc_macro::TokenStream;

mod slot;
mod ui_form;

#[proc_macro_attribute]
pub fn slot(attrs: TokenStream, input: TokenStream) -> TokenStream {
    crate::slot::slot(attrs, input)
}

#[proc_macro_attribute]
pub fn ui_form(attrs: TokenStream, input: TokenStream) -> TokenStream {
    crate::ui_form::ui_form(attrs, input)
}
