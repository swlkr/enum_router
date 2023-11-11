use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{parse::Parse, parse_macro_input, Data, DeriveInput, LitStr, Result};

#[proc_macro_derive(Routes, attributes(route))]
pub fn routes(s: TokenStream) -> TokenStream {
    let input = parse_macro_input!(s as DeriveInput);
    match routes_macro(input) {
        Ok(s) => s.to_token_stream().into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn routes_macro(input: DeriveInput) -> Result<TokenStream2> {
    let enum_name = input.ident;
    let Data::Enum(enum_data) = input.data else {
        panic!("Only enums are supported");
    };

    let variants = enum_data.variants;
    let parts = variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            let attr = variant
                .attrs
                .iter()
                .filter_map(|attr| attr.parse_args::<Attr>().ok())
                .last()
                .expect("#[route] attr required");
            let uri = attr.0;

            (ident, uri)
        })
        .collect::<Vec<_>>();

    let route_to_str = parts
        .iter()
        .map(|(ident, uri)| quote! { #enum_name::#ident => #uri })
        .collect::<Vec<_>>();

    let str_to_route = parts
        .iter()
        .map(|(ident, uri)| quote! { #uri => #enum_name::#ident })
        .collect::<Vec<_>>();

    let not_found_str_to_route = parts
        .iter()
        .filter(|(_, uri)| uri.value().ends_with("404"))
        .map(|(ident, _)| quote! { _ => #enum_name::#ident })
        .last();

    let not_found = match not_found_str_to_route {
        Some(not_found) => not_found,
        None => quote! { _ => unimplemented!() },
    };

    Ok(quote! {
        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(match self {
                    #(#route_to_str,)*
                })
            }
        }

        impl From<&str> for #enum_name {
            fn from(value: &str) -> Self {
                match value {
                    #(#str_to_route,)*
                    #not_found
                }
            }
        }
    })
}

impl Parse for Attr {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let route = input.parse::<LitStr>()?;
        Ok(Attr(route))
    }
}

#[derive(Clone)]
struct Attr(LitStr);
