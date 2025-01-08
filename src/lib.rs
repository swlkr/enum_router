pub use enum_router_macro::{resource, router, QueryString, Routes};
extern crate self as enum_router;

pub fn urlencode(s: impl std::fmt::Display) -> String {
    s.to_string()
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => c
                .to_string()
                .bytes()
                .map(|b| format!("%{:02X}", b))
                .collect(),
        })
        .collect()
}
