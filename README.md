# Leptos Server Sent Events

[![GitHub Actions](https://github.com/messense/leptos_sse/workflows/CI/badge.svg)](https://github.com/messense/leptos_sse/actions?query=workflow%3ACI)
[![Crates.io](https://img.shields.io/crates/v/leptos_sse.svg)](https://crates.io/crates/leptos_sse)
[![docs.rs](https://docs.rs/leptos_sse/badge.svg)](https://docs.rs/leptos_sse/)

Server signals are [leptos] [signals] kept in sync with the server through server-sent-events (SSE).

The signals are read-only on the client side, and can be written to by the server.
This is useful if you want real-time updates on the UI controlled by the server.

Changes to a signal are sent through a SSE to the client as [json patches].

[leptos]: https://crates.io/crates/leptos
[signals]: https://docs.rs/leptos/latest/leptos/struct.Signal.html
[json patches]: https://docs.rs/json-patch/latest/json_patch/struct.Patch.html

This project is heavily based on [leptos_server_signal](https://github.com/tqwewe/leptos_server_signal).

## Feature flags

- `ssr`: ssr is enabled when rendering the app on the server.
- `actix`: integration with the [Actix] web framework.
- `axum`: integration with the [Axum] web framework.

[actix]: https://crates.io/crates/actix-web
[axum]: https://crates.io/crates/axum

# Example

**Cargo.toml**

```toml
[dependencies]
leptos_sse = "*"
serde = { version = "*", features = ["derive"] }

[features]
ssr = [
  "leptos_sse/ssr",
  "leptos_sse/axum", # or actix
]
```

**Client**

```rust
use leptos::*;
use leptos_sse::create_sse_signal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Count {
    pub value: i32,
}

#[component]
pub fn App() -> impl IntoView {
    // Provide SSE connection
    leptos_sse::provide_sse("http://localhost:3000/sse").unwrap();

    // Create server signal
    let count = create_sse_signal::<Count>("counter");

    view! {
        <h1>"Count: " {move || count.get().value.to_string()}</h1>
    }
}
```

> If on stable, use `count.get().value` instead of `count().value`.

**Server (Axum)**

```rust
#[cfg(feature = "ssr")]
use {
    axum::response::sse::{Event, KeepAlive, Sse},
    futures::stream::Stream,
};

#[cfg(feature = "ssr")]
async fn handle_sse() -> Sse<impl Stream<Item = Result<Event, axum::BoxError>>> {
    use futures::stream;
    use leptos_sse::ServerSentEvents;
    use std::time::Duration;
    use tokio_stream::StreamExt as _;

    let mut value = 0;
    let stream = ServerSentEvents::new(
        "counter",
        stream::repeat_with(move || {
            let curr = value;
            value += 1;
            Ok(Count { value: curr })
        })
        .throttle(Duration::from_secs(1)),
    )
    .unwrap();
    Sse::new(stream).keep_alive(KeepAlive::default())
}
```

## License

This work is released under the MIT license. A copy of the license is provided in the [LICENSE](./LICENSE) file.