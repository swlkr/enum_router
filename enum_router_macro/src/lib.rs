use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Ident, LitStr, Result};

#[proc_macro_derive(Routes, attributes(get, post, delete, patch, put))]
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
    let vars = variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            let fields = &variant.fields;
            let attr = variant
                .attrs
                .iter()
                .map(|attr| {
                    let method = &attr
                        .path
                        .segments
                        .last()
                        .expect("#[get], #[post], #[delete], #[patch] or #[put] only")
                        .ident;
                    Attr {
                        method,
                        path: attr
                            .parse_args::<LitStr>()
                            .expect("attributes expect a string literal"),
                    }
                })
                .last()
                .expect("#[get] or #[post] attr required");

            (ident, fields, attr)
        })
        .collect::<Vec<_>>();

    let route_to_string = vars
        .iter()
        .map(|(ident, fields, Attr { path, .. })| match fields {
            syn::Fields::Named(fields) => {
                let format = path
                    .value()
                    .split('/')
                    .map(|part| if part.starts_with(":") { "{}" } else { part })
                    .collect::<Vec<_>>()
                    .join("/");
                let idents = fields
                    .named
                    .iter()
                    .map(|field| field.ident.as_ref().unwrap())
                    .collect::<Vec<_>>();

                quote! { #enum_name::#ident { #(#idents,)* } => format!(#format, #(#idents,)*) }
            }
            syn::Fields::Unnamed(fields) => {
                let format = path
                    .value()
                    .split('/')
                    .map(|part| if part.starts_with(":") { "{}" } else { part })
                    .collect::<Vec<_>>()
                    .join("/");

                let idents = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, _field)| Ident::new(&format!("x{}", i), Span::call_site()))
                    .collect::<Vec<_>>();

                quote! { #enum_name::#ident(#(#idents,)*) => format!(#format, #(#idents,)*) }
            }
            syn::Fields::Unit => quote! { #enum_name::#ident => #path.to_owned() },
        })
        .collect::<Vec<_>>();

    let route_to_path = vars
        .iter()
        .map(|(ident, fields, Attr { path, .. })| match fields {
            syn::Fields::Named(fields) => {
                let format = path
                    .value()
                    .split('/')
                    .map(|part| if part.starts_with(":") { "{}" } else { part })
                    .collect::<Vec<_>>()
                    .join("/");

                let idents = fields
                    .named
                    .iter()
                    .map(|field| field.ident.as_ref().unwrap())
                    .collect::<Vec<_>>();

                quote! { #enum_name::#ident { #(#idents,)* } => { format!(#format, #(#idents,)*); #path.to_owned() } }
            }
            syn::Fields::Unnamed(fields) => {
                let format = path
                    .value()
                    .split('/')
                    .map(|part| if part.starts_with(":") { "{}" } else { part })
                    .collect::<Vec<_>>()
                    .join("/");

                let idents = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, _field)| Ident::new(&format!("x{}", i), Span::call_site()))
                    .collect::<Vec<_>>();

                quote! { #enum_name::#ident(#(#idents,)*) => format!(#format, #(#idents,)*) }
            },
            syn::Fields::Unit => quote! { #enum_name::#ident => #path.to_owned() },
        })
        .collect::<Vec<_>>();

    let axum_route = vars
        .iter()
        .map(|(ident, _, Attr { path, method })| {
            let fn_name = pascal_to_camel(&ident.to_string());
            quote! { .route(#path, #method(#fn_name)) }
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        impl Route {
            fn url(&self) -> String {
                match self {
                    #(#route_to_string,)*
                }
            }

            fn path(&self) -> String {
                match self {
                    #(#route_to_path,)*
                }
            }

            fn router() -> ::axum::Router {
                use ::axum::routing::{get, post, patch, put, delete};
                ::axum::Router::new()#(#axum_route)*
            }
        }

        impl std::fmt::Display for Route {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!("{}", self.url()))
            }
        }
    })
}

fn pascal_to_camel(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut is_first = true;
    let mut prev_char = ' ';

    for c in input.chars() {
        if c.is_alphanumeric() {
            if is_first {
                result.push(c.to_lowercase().next().unwrap());
                is_first = false;
            } else {
                if prev_char.is_uppercase() {
                    result.push('_');
                }
                result.push(c.to_lowercase().next().unwrap());
            }
        }
        prev_char = c;
    }

    result
}

#[derive(Clone)]
struct Attr<'a> {
    path: LitStr,
    method: &'a Ident,
}
