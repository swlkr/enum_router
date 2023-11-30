# enum_router

enum_router is a rust enum router for axum that doesn't support nesting.

```sh
cargo add enum_router # still not on crates.io (yet)
```

# Declare your routes

```rust
use enum_router::Routes;

#[derive(Routes)]
enum Route {
  #[get("/")]
  Root,
  #[get("/todos/:id/edit")]
  EditTodo(i32)
  #[put("/todos/:id")]
  UpdateTodo(i32)
}
```

It will complain about missing functions which you still have to write:

```rust
async fn root() -> &'static str {
  "root"
}

async fn edit_todo(Path(id): Path<i32>) -> String {
  format!("todo {id}")
}

async fn update_todo(Path(id): Path<i32>) -> String {
  format!("todo {id}")
}
```

Then you now have axum routing like this:

```rust
#[tokio::main]
async fn main() {
  let router = Route::router();
  let addr = "127.0.0.1:9001".parse().unwrap();
  axum::Server::bind(&addr).serve(router.into_make_service()).await.unwrap();
}
```
