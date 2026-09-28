# Fabric Standard HTTP Native Runtime

This crate is the Native Fabric runtime for Fabric Standard HTTP.

It runs a `fabric_standard_http::App` as a long-running local Fabric-backed HTTP
server. The portable HTTP application lives in `onoal-fabric-standard-http`;
this crate owns only the native runtime boundary:

- Fabric HTTP Server Composition;
- TCP bind configuration;
- actual bound-address inspection;
- conversion between Fabric Ecosystem HTTP exchanges and Standard HTTP requests;
- blocking accept loop and native server lifecycle.

## Run

```rust
use fabric_standard_http::{App, Response};
use fabric_standard_http_runtime_native::{serve, ServeConfig};

fn main() -> fabric_standard_http_runtime_native::Result<()> {
    let app = App::new()
        .get("/", |_ctx| async { Ok(Response::text("hello")) })?;

    serve(app, ServeConfig::local())
}
```

From the repository root:

```bash
cargo run -p onoal-fabric-standard-http-runtime-native --example hello
```

Then in another terminal:

```bash
curl http://127.0.0.1:3000/
curl http://127.0.0.1:3000/users/42
curl http://127.0.0.1:3000/health
```

Native Fabric is one runtime realization. It is not the identity of the
Standard HTTP application.
