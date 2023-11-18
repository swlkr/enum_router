use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{parse::Parse, parse_macro_input, Data, DeriveInput, Expr, Ident, LitStr, Result};

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
    let vars = variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            let fields = &variant.fields;
            let attr = variant
                .attrs
                .iter()
                .filter_map(|attr| attr.parse_args::<Attr>().ok())
                .last()
                .expect("#[route] attr required");

            (ident, fields, attr)
        })
        .collect::<Vec<_>>();

    let route_to_string = vars
        .iter()
        .map(|(ident, fields, Attr { url: uri, .. })| match fields {
            syn::Fields::Named(fields) => {
                let format = uri
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
                let format = uri
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
            syn::Fields::Unit => quote! { #enum_name::#ident => #uri.to_owned() },
        })
        .collect::<Vec<_>>();

    let route_to_path = vars
        .iter()
        .map(|(ident, fields, Attr { url: uri, ..})| match fields {
            syn::Fields::Named(fields) => {
                let format = uri
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

                quote! { #enum_name::#ident { #(#idents,)* } => { format!(#format, #(#idents,)*); #uri.to_owned() } }
            }
            syn::Fields::Unnamed(fields) => {
                let format = uri
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
            syn::Fields::Unit => quote! { #enum_name::#ident => #uri.to_owned() },
        })
        .collect::<Vec<_>>();

    let axum_route = vars
        .iter()
        .filter(|(_, _, Attr { handlers, .. })| handlers.is_some())
        .map(|(_ident, _, Attr { url, handlers })| {
            quote! { .route(#url, #handlers) }
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

impl Parse for Attr {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut attr = Attr {
            url: LitStr::new("", Span::call_site()),
            handlers: None,
        };
        let parsed = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated(input)?;
        for expr in parsed.iter() {
            match expr {
                Expr::Lit(expr) => match &expr.lit {
                    syn::Lit::Str(lit_str) => {
                        attr.url = lit_str.clone();
                    }
                    _ => panic!("#[route] first arg can only be &str"),
                },
                Expr::MethodCall(expr) => {
                    attr.handlers = Some(expr.to_token_stream());
                }
                Expr::Call(expr) => attr.handlers = Some(expr.to_token_stream()),
                _ => {}
            }
        }

        Ok(attr)
    }
}

#[derive(Clone)]
struct Attr {
    url: LitStr,
    handlers: Option<TokenStream2>,
}
