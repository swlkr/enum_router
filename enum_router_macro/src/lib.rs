use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{
    parse::Parse, parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput, Fields,
    FieldsNamed, FieldsUnnamed, Ident, ItemEnum, LitStr, Result, Type, Variant,
};

struct Args {
    state: Option<Type>,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let state = input.parse::<Type>().ok();

        Ok(Self { state })
    }
}

#[proc_macro_attribute]
pub fn router(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Args);
    let input = parse_macro_input!(input as ItemEnum);
    match router_macro(args, input) {
        Ok(s) => s.to_token_stream().into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn router_macro(args: Args, item_enum: ItemEnum) -> Result<TokenStream2> {
    let attr = match args.state {
        Some(st) => quote! { #st },
        None => quote! { () },
    };

    let expanded = quote! {
        #[derive(enum_router::Routes)]
        #[state(#attr)]
        #item_enum
    };

    Ok(expanded)
}

#[proc_macro_attribute]
pub fn resource(_args: TokenStream, input: TokenStream) -> TokenStream {
    // let args = parse_macro_input!(args as Args);
    let input = parse_macro_input!(input as ItemEnum);
    match resource_macro(input) {
        Ok(s) => s.to_token_stream().into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn resource_macro(item_enum: ItemEnum) -> Result<TokenStream2> {
    let ident = &item_enum.ident;
    let variants = item_enum
        .variants
        .iter()
        .map(|variant| RouteVariant::try_from(variant))
        .collect::<Result<Vec<_>>>()?;

    let routes: Vec<TokenStream2> = variants
        .iter()
        .map(
            |RouteVariant {
                 attr,
                 path,
                 variant,
                 fields: _,
             }| match attr {
                Attr::Resource => quote! { .merge(#variant::router()) },
                method => {
                    let fn_string = pascal_to_snake(&variant.to_string());
                    let fn_name = Ident::new(&fn_string, variant.span());
                    let method = Ident::new(&method.to_string(), variant.span());
                    let path = format!(
                        "/{}{}",
                        pascal_to_snake(&ident.to_string()),
                        path.value().replace("{{}}", "{id}")
                    );
                    quote! { .route(#path, #method(#ident::#fn_name)) }
                }
            },
        )
        .collect();

    let lowercase = ident.to_string().to_lowercase();

    let urls = variants
        .iter()
        .map(|rv| {
            let left = left(&ident, &rv.variant, &rv.fields);
            let right = right(&rv.fields, &rv.path);
            quote! { #left => #right }
        })
        .collect::<Vec<_>>();
    let expanded = quote! {
        #[derive(Debug)]
        #item_enum

        impl #ident {
            pub fn router() -> ::axum::Router {
                use ::axum::routing::{get, post, patch, put, delete, trace, head};
                ::axum::Router::new()
                    #(#routes)*
            }

            pub fn url(&self) -> String {
                match self {
                    #(#urls,)*
                }
            }
        }

        impl core::fmt::Display for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let url = self.url();
                f.write_fmt(format_args!("/{}{}", #lowercase, self.url()))
            }
        }
    };

    Ok(expanded)
}

#[proc_macro_derive(QueryString)]
pub fn query_string(s: TokenStream) -> TokenStream {
    let input = parse_macro_input!(s as DeriveInput);
    match query_string_macro(input) {
        Ok(s) => s.to_token_stream().into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn query_string_macro(input: DeriveInput) -> Result<TokenStream2> {
    let struct_name = input.ident;
    let Data::Struct(data) = input.data else {
        return Err(syn::Error::new(
            Span::call_site(),
            "Only structs are supported",
        ));
    };
    // let field_names = data
    //     .fields
    //     .iter()
    //     .filter_map(|field| field.ident.as_ref())
    //     .collect::<Vec<_>>();
    let field_tokens = data
        .fields
        .iter()
        .filter(|field| field.ident.is_some())
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            let name = ident.to_string();
            quote! { (#name, self.#ident) }
        })
        .collect::<Vec<_>>();

    let tokens = quote! {
        impl #struct_name {
            fn query_string(&self) -> String {
                [#(#field_tokens,)*]
                    .iter()
                    .filter(|(_, value)| value.is_some())
                    .map(|(key, value)| format!("{}={}", key, value.unwrap()))
                    .collect::<Vec<_>>()
                    .join("&")
            }
        }
    };

    Ok(tokens)
}

#[proc_macro_derive(
    Routes,
    attributes(get, post, delete, patch, put, trace, head, state, resource, query)
)]
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
        return Err(syn::Error::new(
            Span::call_site(),
            "Only enums are supported",
        ));
    };

    let arg = input
        .attrs
        .iter()
        .filter(|attr| match attr.path.get_ident() {
            Some(ident) => ident.to_string() == "state",
            None => false,
        })
        .filter_map(args)
        .last();

    let state_generic = match arg {
        Some(Args { state }) => quote! { #state },
        None => quote! { () },
    };

    let variants = data
        .variants
        .iter()
        .map(|variant| RouteVariant::try_from(variant))
        .collect::<Result<Vec<_>>>()?;

    let urls = variants
        .iter()
        .map(
            |RouteVariant {
                 variant,
                 fields,
                 path,
                 ..
             }| {
                let left = left(&enum_name, variant, fields);
                let right = right(&fields, path);

                quote! { #left => #right }
            },
        )
        .collect::<Vec<_>>();

    let methods = variants
        .iter()
        .map(
            |RouteVariant {
                 variant,
                 fields,
                 path,
                 ..
             }| {
                let left = left(&enum_name, variant, fields);
                let right = right(fields, path);

                quote! { #left => #right.to_owned() }
            },
        )
        .collect::<Vec<_>>();

    let axum_route = variants
        .iter()
        .map(
            |RouteVariant {
                 attr,
                 variant,
                 path,
                 ..
             }| match attr {
                Attr::Resource => quote! { .merge(#variant::router()) },
                method => {
                    let fn_string = pascal_to_snake(&variant.to_string());
                    let fn_name = Ident::new(&fn_string, variant.span());
                    let method = Ident::new(&method.to_string(), variant.span());
                    quote! { .route(#path, #method(#fn_name)) }
                }
            },
        )
        .collect::<Vec<_>>();

    let expanded = quote! {
        impl #enum_name {
            pub fn url(&self) -> String {
                match self {
                    #(#urls,)*
                }
            }

            #[allow(unused)]
            pub fn method(&self) -> String {
                match self {
                    #(#methods,)*
                }
            }

            pub fn router() -> ::axum::Router<#state_generic> {
                use ::axum::routing::{get, post, patch, put, delete, trace, head};
                ::axum::Router::new()#(#axum_route)*
            }
        }

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!("{}", self.url()))
            }
        }
    };

    Ok(expanded)
}

fn right_from_unnamed(path: &LitStr, fields: &FieldsUnnamed) -> TokenStream2 {
    let is_query = fields
        .unnamed
        .iter()
        .any(|field| field.attrs.iter().any(|attr| attr.path.is_ident("query")));

    match is_query {
        true => {
            quote! {
                {
                    let qs = x0.query_string();
                    if qs.is_empty() {
                        #path.to_string()
                    } else {
                        format!("{}?{}", #path, x0.query_string())
                    }
                }
            }
        }
        false => {
            let format = path
                .value()
                .split('/')
                .map(|part| if part.contains("{") { "{}" } else { part })
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
    }
}

fn right_from_named(fields: &FieldsNamed, path: &LitStr) -> TokenStream2 {
    let idents = fields
        .named
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<_>>();

    let query = idents
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

fn pascal_to_snake(input: &str) -> String {
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
enum Attr {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Trace,
    Head,
    Resource,
}

struct RouteVariant<'a> {
    attr: Attr,
    path: LitStr,
    variant: &'a Ident,
    fields: &'a Fields,
}

impl<'a> TryFrom<&'a Variant> for RouteVariant<'a> {
    type Error = syn::Error;

    fn try_from(value: &'a Variant) -> std::result::Result<Self, Self::Error> {
        let variant = &value.ident;

        let (attr, path) = if value.attrs.is_empty() {
            match variant.to_string().as_str() {
                "Index" => (Attr::Get, LitStr::new("", variant.span())),
                "Create" => (Attr::Post, LitStr::new("", variant.span())),
                "New" => (Attr::Get, LitStr::new("/new", variant.span())),
                "Show" => (Attr::Get, LitStr::new("/{{}}", variant.span())),
                "Edit" => (Attr::Get, LitStr::new("/{{}}/edit", variant.span())),
                "Update" => (Attr::Patch, LitStr::new("/{{}}", variant.span())),
                "Delete" => (Attr::Delete, LitStr::new("/{{}}", variant.span())),
                _ => todo!(),
            }
        } else {
            value
                .attrs
                .iter()
                .filter_map(|attr| {
                    let ident = attr.path.get_ident();
                    let lit_str = attr.parse_args::<LitStr>().ok();
                    match (ident, lit_str) {
                        (None, None) => None,
                        (None, Some(_)) => None,
                        (Some(_), None) => Some((
                            Attr::Resource,
                            LitStr::new(&format!("/{}", variant.to_string()), variant.span()),
                        )),
                        (Some(ident), Some(lit_str)) => Some((Attr::from(ident), lit_str)),
                    }
                })
                .nth(0)
                .ok_or(syn::Error::new(variant.span(), "Unsupported attr"))?
        };
        let fields = &value.fields;

        Ok(RouteVariant {
            attr,
            path,
            variant,
            fields,
        })
    }
}

impl From<&Ident> for Attr {
    fn from(value: &Ident) -> Self {
        match value.to_string().as_str() {
            "get" => Attr::Get,
            "post" => Attr::Post,
            "put" => Attr::Put,
            "patch" => Attr::Patch,
            "delete" => Attr::Delete,
            "head" => Attr::Head,
            "trace" => Attr::Trace,
            _ => Attr::Resource,
        }
    }
}

impl core::fmt::Display for Attr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Attr::Get => "get",
            Attr::Post => "post",
            Attr::Put => "put",
            Attr::Patch => "patch",
            Attr::Delete => "delete",
            Attr::Trace => "trace",
            Attr::Head => "head",
            Attr::Resource => "",
        })
    }
}

fn args(attr: &Attribute) -> Option<Args> {
    attr.parse_args::<Args>().ok()
}
