pub use enum_router_macro::Routes;
extern crate self as enum_router;

#[cfg(test)]
mod tests {
    use enum_router::Routes;

    #[test]
    fn it_works() {
        #[derive(Routes, Debug, PartialEq)]
        enum Route {
            #[route("/")]
            Root,
            #[route("/todos")]
            Todos,
            #[route("/todos/:id")]
            ShowTodo { id: i32 },
            #[route("/todos/:id/edit")]
            EditTodo(i32),
        }

        assert_eq!(Route::Root.url(), "/");
        assert_eq!(Route::Todos.url(), "/todos");
        assert_eq!(Route::ShowTodo { id: 1 }.url(), "/todos/1");
        assert_eq!(Route::Root.path(), "/");
        assert_eq!(Route::EditTodo(0).url(), "/todos/0/edit");
        assert_eq!(Route::ShowTodo { id: i32::default() }.path(), "/todos/:id");
        // assert_eq!(Route::parse("/todos/1"), Some(Route::ShowTodo { id: 1 }));
        // assert_eq!(path!(Route::ShowTodo), "/todos/:id");
    }

    #[cfg(feature = "axum")]
    mod axum {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use axum::response::IntoResponse;
        use enum_router::Routes;
        use tower::ServiceExt;

        async fn get_index() -> impl IntoResponse {
            "get_index"
        }

        async fn post_index() -> impl IntoResponse {
            "post_index"
        }

        #[derive(Routes, Debug, PartialEq)]
        pub enum Route {
            #[allow(unused)]
            #[route("/", get(get_index).post(post_index))]
            Root,
        }

        #[tokio::test]
        async fn axum_router_works() {
            let app = Route::router();
            let response = app
                .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
