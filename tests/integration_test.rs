use enum_router::*;
use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::Router;
use enum_router::router;
use enum_router_macro::resource;
use serde::Deserialize;
use tower::ServiceExt;

type Result<T> = core::result::Result<T, Box<dyn core::error::Error>>;

async fn index() -> impl IntoResponse {
    "index"
}

async fn login_form() -> impl IntoResponse {
    "login form"
}

async fn login() -> impl IntoResponse {
    "login"
}

#[derive(PartialEq, Debug, QueryString, Deserialize)]
pub struct Abc {
    abc: Option<u8>,
}

async fn abc(Query(abc): Query<Abc>) -> impl IntoResponse {
    Route::Abc(abc).to_string()
}

async fn xyz(Path(xyz): Path<String>) -> impl IntoResponse {
    format!("/xyz?xyz={}", xyz)
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
    #[get("/abc")]
    Abc(#[query] Abc),
    #[get("/xyz/{xyz}")]
    Xyz(String),
}

#[tokio::test]
async fn it_works() -> Result<()> {
    let app = Route::router();

    assert_eq!(StatusCode::OK, make_request(&app, "GET", "/").await);
    assert_eq!(StatusCode::OK, make_request(&app, "GET", "/login").await);
    assert_eq!(StatusCode::OK, make_request(&app, "POST", "/login").await);
    assert_eq!(
        StatusCode::NOT_FOUND,
        make_request(&app, "GET", "/nope").await
    );
    assert_eq!(StatusCode::OK, make_request(&app, "GET", "/abc").await);
    assert_eq!(
        StatusCode::OK,
        make_request(&app, "GET", "/abc?abc=123").await
    );

    assert_eq!("/abc?abc=123", Route::Abc(Abc { abc: Some(123) }).to_string());
    assert_eq!("/abc", Route::Abc(Abc { abc: None }).to_string());

    Ok(())
}

#[tokio::test]
async fn resource_routing_works() -> Result<()> {
    #[router]
    enum Route {
        #[get("/")]
        Index,
        #[resource]
        #[allow(unused)]
        Sessions
    }

    async fn index() -> String {
        Route::Index.to_string()
    }

    #[resource]
    pub enum Sessions {
        Index, New, Create, Edit(i64), Update(i64)
    }

    impl Sessions {
        async fn index() -> String {
            Self::Index.to_string()
        }

        async fn new() -> String {
            Self::New.to_string()
        }

        async fn create() -> String {
            Self::Create.to_string()
        }

        async fn edit(Path(id): Path<i64>) -> String {
            Self::Edit(id).to_string()
        }

        async fn update(Path(id): Path<i64>) -> String {
            Self::Update(id).to_string()
        }
    }


    let url = format!("{}", Route::Index);
    assert_eq!(url, "/");

    let url = format!("{}", Sessions::Index);
    assert_eq!(url, "/sessions");
    let url = format!("{}", Sessions::New);

    assert_eq!(url, "/sessions/new");

    let url = format!("{}", Sessions::Edit(1));
    assert_eq!(url, "/sessions/1/edit");

    let app = Route::router();
    assert_eq!(StatusCode::OK, make_request(&app, "GET", "/").await);
    assert_eq!(StatusCode::OK, make_request(&app, "GET", "/sessions").await);
    assert_eq!(StatusCode::OK, make_request(&app, "GET", "/sessions/new").await);
    assert_eq!(StatusCode::OK, make_request(&app, "GET", "/sessions/1/edit").await);
    assert_eq!(StatusCode::OK, make_request(&app, "PATCH", "/sessions/1").await);
    assert_eq!(StatusCode::OK, make_request(&app, "POST", "/sessions").await);

    Ok(())
}

#[tokio::test]
async fn state_works() -> Result<()> {
    use axum::extract::State;
    use std::sync::Arc;

    struct AppState {
        #[allow(unused)]
        a: String,
    }

    #[router(Arc<AppState>)]
    #[allow(unused)]
    enum Route {
        #[get("/")]
        Index,
    }

    async fn index(State(_s): State<Arc<AppState>>) -> &'static str {
        "index"
    }

    let app = Route::router().with_state(Arc::new(AppState { a: "".into() }));

    assert_eq!(StatusCode::OK, make_request(&app, "GET", "/").await);

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
