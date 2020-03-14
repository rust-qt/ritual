use proc_macro::TokenStream;
use quote::quote;
use syn::{export::Span, parse_macro_input, Ident, LitStr};

pub fn q_init_resource(input: TokenStream) -> TokenStream {
    let resource_name = parse_macro_input!(input as LitStr);
    let fn_name = Ident::new(
        &format!("ritual_init_resource_{}", resource_name.value()),
        Span::call_site(),
    );
    let expanded = quote! {
        {
            extern "C" {
                fn #fn_name();
            }
            unsafe {
                #fn_name();
            }
        }
    };

    TokenStream::from(expanded)
}
