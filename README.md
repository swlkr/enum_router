# enum_router

enum_router is a rust enum router that doesn't support nesting.

```sh
cargo add enum_router # still not on crates.io (yet)
```

# Declare your routes

```rust
use enum_router::Routes;

#[derive(Routes)]
enum Route {
  #[route("/")]
  Root,
  #[route("/todos")]
  Todos,
  #[route("/todos/:id")]
  ShowTodo { id: i32 },
  #[route("/todos/:id/edit")]
  EditTodo(i32)
}
```

Then it works like this:

```rust
#[cfg(test)]
mod tests {
  #[test]
  fn it_works() {
    assert_eq!(Route::Root.url(), "/");
    assert_eq!(Route::Todos.url(), "/todos");
    assert_eq!(Route::Todos { id: 1 }.url(), "/todos/1");
  }
}
```
