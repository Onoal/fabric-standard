# Fabric Standard HTTP Cloudflare Runtime

This crate adapts Cloudflare Workers fetch invocation to Fabric Standard HTTP.

It owns:

- conversion from `worker::Request` to `fabric_standard_http::Request`;
- invocation of the same `PreparedApp` dispatcher used by every Standard HTTP runtime;
- conversion from `fabric_standard_http::Response` to `worker::Response`.

It does not own routing, middleware, handlers, 404 behavior, response builders,
or Fabric capability realization.

## Use

```rust
use fabric_standard_http::{App, Response as StandardResponse};
use fabric_standard_http_runtime_cloudflare::CloudflareApp;
use worker::{event, Context, Env, Request, Response};

#[event(fetch)]
pub async fn main(
    request: Request,
    _env: Env,
    _ctx: Context,
) -> worker::Result<Response> {
    let app = App::new()
        .get("/", |_ctx| async { Ok(StandardResponse::text("hello")) })?;

    CloudflareApp::new(app)?.fetch(request).await
}
```

The runtime currently supports pure Standard HTTP applications. Apps declaring
Fabric contributions or `KeyValue` requirements are rejected explicitly.
Cloudflare KV is not implemented or faked in this runtime; future Cloudflare
capability support must be a Fabric Resource realization.
