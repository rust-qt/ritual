use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, ItemStruct};

pub fn ui_form(attrs: TokenStream, input: TokenStream) -> TokenStream {
    let ui_file_path = parse_macro_input!(attrs as Literal);
    let input = parse_macro_input!(input as ItemStruct);

    let struct_name = &input.ident;

    let mut fields = input
        .fields
        .iter()
        .map(|f| f.ident.clone().expect("unnamed fields are not supported"))
        .collect::<Vec<_>>();
    let first_field = fields.remove(0);
    let field_names = fields
        .iter()
        .map(|ident| ident.to_token_stream().to_string())
        .collect::<Vec<_>>();

    let expanded = quote! {
        #input

        impl #struct_name {
            pub fn load() -> Self {
                unsafe {
                    let loader = ::qt_ui_tools::QUiLoader::new_0a();
                    let widget = loader.load_bytes(include_bytes!(#ui_file_path));
                    assert!(!widget.is_null(), "invalid ui file");

                    Self {
                        #(
                            #fields: widget.find_child(#field_names).unwrap(),
                        )*
                        #first_field: widget,
                    }
                }
            }
        }
    };

    TokenStream::from(expanded)
}
