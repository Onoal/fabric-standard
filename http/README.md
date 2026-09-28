# Fabric Standard HTTP

Fabric Standard HTTP starts with a small application object:

```rust
use fabric_standard_http::{App, Response};

# fn build_app() -> fabric_standard_http::Result<App> {
let app = App::new()
    .get("/", |_ctx| async { Ok(Response::text("hello")) })?;

# Ok(app)
# }
```

The same application can run socket-free in tests or through runtime-specific
entrypoint crates such as `onoal-fabric-standard-http-runtime-native`.

## What It Provides

- method + path route declaration;
- path parameters and wildcard captures;
- ordered repeated headers;
- query access without dropping repeated values;
- text, bytes, JSON, and redirect responses;
- onion middleware;
- default and custom 404 handling;
- sub-app composition;
- one first Fabric resource proof with `KeyValue`.

## Socket-Free Execution

Pure HTTP applications can be prepared and dispatched without Fabric
materialization:

```rust
use fabric_standard_http::{App, Request, Response};

# async fn test() -> fabric_standard_http::Result<()> {
let prepared = App::new()
    .get("/", |_ctx| async { Ok(Response::text("hello")) })?
    .prepare()?;

let response = prepared.request(Request::get("/")).await?;
# Ok(())
# }
```

`PreparedApp` represents prepared HTTP semantics only. If a handler requires a
Fabric-bound capability, use `TestRuntime` for socket-free tests:

```rust
use fabric_package_key_value::memory_key_value;
use fabric_standard_http::{App, Request, Response};

# async fn test() -> fabric_standard_http::Result<()> {
let app = App::new()
    .with_fabric(memory_key_value("sessions"))
    .use_key_value("sessions")?
    .post("/sessions/:id", |ctx| async move {
        let id = ctx.param("id")?.to_owned();
        ctx.key_value()?.set(id, b"active".to_vec())?;
        Ok(Response::text("stored"))
    })?;

let runtime = app.test_runtime()?;
let response = runtime
    .request(Request::post("/sessions/abc", Vec::new()))
    .await?;
runtime.stop()?;
# Ok(())
# }
```

`TestRuntime` materializes the app's Fabric contributions and relation-bound
capabilities without opening TCP sockets.

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

Native serving lives in `onoal-fabric-standard-http-runtime-native`:

```bash
cargo run -p onoal-fabric-standard-http-runtime-native --example hello
```

The example listens on `127.0.0.1:3000` and remains running until the process is terminated.
