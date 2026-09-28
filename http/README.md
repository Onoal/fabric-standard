# Fabric Standard HTTP

Fabric Standard HTTP starts with a small application object:

```rust
use fabric_standard_http::{serve, App, Response, ServeConfig};

fn main() -> fabric_standard_http::Result<()> {
    let app = App::new()
        .get("/", |_ctx| async { Ok(Response::text("hello")) })?;

    serve(app, ServeConfig::local())
}
```

The same prepared application can run socket-free in tests or through a real Fabric HTTP server.

## What It Provides

- method + path route declaration;
- path parameters and wildcard captures;
- ordered repeated headers;
- query access without dropping repeated values;
- text, bytes, JSON, redirect, and raw HTTP responses;
- onion middleware;
- default and custom 404 handling;
- sub-app composition;
- local blocking `serve`;
- one first Fabric resource proof with `KeyValue`.

## Fabric Composition

For resource-backed handlers, add ordinary Fabric contributions during app authoring:

```rust
use fabric_package_key_value::memory_key_value;
use fabric_standard_http::{App, Response};

let app = App::new()
    .with_fabric(memory_key_value("sessions"))
    .use_key_value("sessions")?
    .post("/sessions/:id", |ctx| async move {
        let key = ctx.param("id")?.to_owned();
        ctx.key_value()?.set(key, b"active".to_vec())?;
        Ok(Response::text("stored"))
    })?;
```

`"sessions"` is the Fabric Resource occurrence name. Request-time access uses `ctx.key_value()`; it is not a string lookup registry.

## Running

```bash
cargo run -p onoal-fabric-standard-http --example hello
```

The example listens on `127.0.0.1:3000` and remains running until the process is terminated.
