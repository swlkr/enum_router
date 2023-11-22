use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, FieldsNamed, FieldsUnnamed, Ident, LitStr,
    Result, Variant,
};

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
    let Data::Enum(data) = input.data else {
        panic!("Only enums are supported");
    };

    let variants = data
        .variants
        .into_iter()
        .map(|variant| RouteVariant::from(variant))
        .collect::<Vec<_>>();

    let urls = variants
        .iter()
        .map(
            |RouteVariant {
                 ref variant,
                 ref fields,
                 ref path,
                 ..
             }| {
                let left = left(&enum_name, variant, fields);
                let right = right(fields, path);

                quote! { #left => #right }
            },
        )
        .collect::<Vec<_>>();

    let methods = variants
        .iter()
        .map(
            |RouteVariant {
                 ref method,
                 ref variant,
                 ref fields,
                 ..
             }| {
                let left = left(&enum_name, variant, fields);
                let right = method.to_string();

                quote! { #left => #right.to_owned() }
            },
        )
        .collect::<Vec<_>>();

    let axum_route = variants
        .iter()
        .map(
            |RouteVariant {
                 ref path,
                 ref method,
                 ref variant,
                 ..
             }| {
                let fn_string = pascal_to_camel(&variant.to_string());
                let fn_name = Ident::new(&fn_string, method.span());
                quote! { .route(#path, #method(#fn_name)) }
            },
        )
        .collect::<Vec<_>>();

    Ok(quote! {
        impl Route {
            fn url(&self) -> String {
                match self {
                    #(#urls,)*
                }
            }

            #[allow(unused)]
            fn method(&self) -> String {
                match self {
                    #(#methods,)*
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

fn right_from_unnamed(path: &LitStr, fields: &FieldsUnnamed) -> TokenStream2 {
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

    quote! { format!(#format, #(#idents,)*) }
}

fn right_from_named(fields: &FieldsNamed, path: &LitStr) -> TokenStream2 {
    let idents = fields
        .named
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<_>>();

    let query =
        idents
            .iter()
            .map(|ident| format!("{}={{:?}}", ident))
            .collect::<Vec<_>>()
            .join("&");

    let format = format!("{}?{}", path.value(), query);

    quote! { format!(#format, #(#idents,)*) }
}

fn right(fields: &Fields, path: &LitStr) -> TokenStream2 {
    match fields {
        Fields::Named(fields) => right_from_named(fields, path),
        Fields::Unnamed(fields) => right_from_unnamed(path, fields),
        Fields::Unit => quote! { #path.to_owned() },
    }
}

fn left_from_named(r#ident: &Ident, variant: &Ident, fields: &FieldsNamed) -> TokenStream2 {
    let idents = fields
        .named
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<_>>();

    quote! {
        #r#ident::#variant { #(#idents,)* }
    }
}

fn left_from_unnamed(r#ident: &Ident, variant: &Ident, fields: &FieldsUnnamed) -> TokenStream2 {
    let idents = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, _field)| Ident::new(&format!("x{}", i), Span::call_site()))
        .collect::<Vec<_>>();

    quote! {
        #r#ident::#variant(#(#idents,)*)
    }
}

fn left(r#ident: &Ident, variant: &Ident, fields: &Fields) -> TokenStream2 {
    match fields {
        syn::Fields::Named(fields) => left_from_named(r#ident, variant, fields),
        syn::Fields::Unnamed(fields) => left_from_unnamed(r#ident, variant, fields),
        syn::Fields::Unit => quote! { #r#ident::#variant },
    }
}

fn pascal_to_camel(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    if let Some(char) = &chars.nth(0) {
        result.push(char.to_ascii_lowercase());
    }

    while let Some(char) = chars.next() {
        if char.is_uppercase() {
            result.push('_');
            result.push(char.to_lowercase().next().unwrap());
        } else {
            result.push(char);
        }
    }

    result
}

#[derive(Clone)]
struct RouteVariant {
    method: Ident,
    path: LitStr,
    variant: Ident,
    fields: Fields,
}

impl From<Variant> for RouteVariant {
    fn from(value: Variant) -> Self {
        let variant = value.ident;
        let (method, path) = value
            .attrs
            .into_iter()
            .filter_map(
                |attr| match (attr.path.get_ident(), attr.parse_args::<LitStr>().ok()) {
                    (Some(ident), Some(path)) => Some((ident.clone(), path)),
                    _ => None,
                },
            )
            .last()
            .expect("should be #[get], #[post], #[put] or #[delete]");
        let fields = value.fields;

        RouteVariant {
            path,
            method,
            variant,
            fields,
        }
    }
}
