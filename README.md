# enum_router

enum_router is a rust enum router that doesn't support nesting or params.

# Install

```sh
cargo add enum_router
```

# Declare your routes

```rust
use enum_router::Routes;

#[derive(Routes)]
struct Route {
  #[route("/")]
  Root,
  #[route("/todos")]
  Todos
  #[route("/404")]
  NotFound
}

// this gives you two way conversions from &str and to strings with std::fmt::Display

let path = "/";
let route = Route::from(path); // => Route::Root

let path = "/does-not-exist";
let route = Route::from(path); // => Route::NotFound. Any route that ends with 404 returns NotFound

let path = Route::Root.to_string(); // => "/"

let path = format!("{}", Route::Todos); // => "/todos"
```

That's it!
