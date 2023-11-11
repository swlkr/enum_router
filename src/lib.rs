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

    #[test]
    fn not_found_works() {
        #[derive(Routes, Debug, PartialEq)]
        enum Route {
            #[route("/")]
            Root,
            #[route("/frontend/404")]
            NotFound,
        }

        assert_eq!(Route::NotFound.to_string(), "/frontend/404");

        let path = "/does-not-exist";
        let route = Route::from(path);
        assert_eq!(Route::NotFound, route)
    }
}
