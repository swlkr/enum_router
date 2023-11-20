pub use enum_router_macro::Routes;
extern crate self as enum_router;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::response::IntoResponse;
    use axum::Router;
    use enum_router::Routes;
    use tower::ServiceExt;

    async fn index() -> impl IntoResponse {
        "index"
    }

    async fn login_form() -> impl IntoResponse {
        "login form"
    }

    async fn login() -> impl IntoResponse {
        "login"
    }

    #[allow(unused)]
    #[derive(Routes, Debug, PartialEq)]
    pub enum Route {
        #[get("/")]
        Index,
        #[get("/login")]
        LoginForm,
        #[post("/login")]
        Login,
    }

    #[tokio::test]
    async fn it_works() -> Result<(), Box<dyn std::error::Error>> {
        let app = Route::router();

        assert_eq!(StatusCode::OK, make_request(&app, "GET", "/").await);
        assert_eq!(StatusCode::OK, make_request(&app, "GET", "/login").await);
        assert_eq!(StatusCode::OK, make_request(&app, "POST", "/login").await);
        assert_eq!(
            StatusCode::NOT_FOUND,
            make_request(&app, "GET", "/nope").await
        );

        Ok(())
    }

    fn request(method: &str, uri: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    async fn make_request(app: &Router, method: &str, uri: &str) -> StatusCode {
        app.clone()
            .oneshot(request(method, uri))
            .await
            .unwrap()
            .status()
    }
}
