use proc_macro::TokenStream;
use quote::quote;
use syn::{export::Span, parse_macro_input, Ident, ItemFn};

pub fn slot(attrs: TokenStream, input: TokenStream) -> TokenStream {
    let slot_type = parse_macro_input!(attrs as Ident);
    let input = parse_macro_input!(input as ItemFn);

    let args = input.sig.inputs.len();
    let arg_names = (0..(args - 1))
        .map(|i| Ident::new(&format!("arg{}", i), Span::call_site()))
        .collect::<Vec<_>>();

    let fn_name = &input.sig.ident;
    let slot_name = Ident::new(&format!("slot_{}", fn_name.to_string()), Span::call_site());
    let vis = &input.vis;

    let expanded = quote! {
        #input

        #vis unsafe fn #slot_name(self: &std::rc::Rc<Self>) -> qt_core::QBox<#slot_type> {
            let this = Rc::clone(&self);
            #slot_type::new(self.main_widget(), move |#(#arg_names),*| {
                this.#fn_name(#(#arg_names),*);
            })
        }
    };

    TokenStream::from(expanded)
}
