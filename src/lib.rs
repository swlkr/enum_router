pub use enum_router_macro::Routes;
extern crate self as enum_router;

#[cfg(test)]
mod tests {
    use enum_router::Routes;

    #[test]
    fn it_works() {
        #[derive(Routes)]
        enum Route {
            #[route("/")]
            Root,
            #[route("/frontend/inc")]
            Inc,
            #[route("/frontend/dec")]
            Dec,
        }

        assert_eq!(Route::Root.to_string(), "/")
    }
}
