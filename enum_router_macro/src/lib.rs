use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Ident, LitStr, Result};

#[proc_macro_derive(Routes, attributes(get, post))]
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
                        .expect("#[get] or #[post] only")
                        .ident;
                    Attr {
                        method,
                        path: attr
                            .parse_args::<LitStr>()
                            .expect("#[get] or #[post] expect a &str"),
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
            quote! { .route(#path, #method(#ident::#method)) }
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

            #[cfg(feature = "axum")]
            #[cfg(feature = "backend")]
            fn router() -> ::axum::Router {
                use ::axum::routing::{get, post};
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

#[derive(Clone)]
struct Attr<'a> {
    path: LitStr,
    method: &'a Ident,
}
