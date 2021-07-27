use proc_macro::TokenStream;
use syn::{DeriveInput, parse::Parser, parse_macro_input};
use quote::quote;

#[proc_macro_derive(Version)]
pub fn version(input: TokenStream) -> TokenStream {
    let struct_fields = quote! {
        pub version: Option<u32>
    };

    let DeriveInput { ident, data, .. } = &mut parse_macro_input!(input);

    if let syn::Data::Struct(s) = data {
        if let syn::Fields::Named(fields) = &mut s.fields {
            fields.named.push(syn::Field::parse_named.parse(struct_fields.into()).unwrap())
        }
    }

    let output = quote! {
        impl #ident {
            fn get_version(&self) -> Option<u32> {
                self.version
            }

            fn set_version(&mut self, version: Option<u32>) {
                self.version = version
            }
        }
    };

    output.into()
}