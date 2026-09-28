use fabric_standard_http::{App, Response};
use fabric_standard_http_runtime_cloudflare::CloudflareApp;
use worker::{event, Context, Env, Request};

#[event(fetch)]
pub async fn main(
    request: Request,
    _env: Env,
    _ctx: Context,
) -> worker::Result<worker::Response> {
    let app = build_app().map_err(|error| worker::Error::RustError(error.to_string()))?;
    CloudflareApp::new(app)
        .map_err(|error| worker::Error::RustError(error.to_string()))?
        .fetch(request)
        .await
}

fn build_app() -> fabric_standard_http::Result<App> {
    App::new()
        .middleware(|ctx, next| async move {
            let mut response = next.run(ctx).await?;
            response.append_header("x-powered-by", "fabric-standard")?;
            Ok(response)
        })?
        .get("/", |_ctx| async {
            Ok(Response::text("hello from Fabric Standard HTTP on Cloudflare"))
        })?
        .get("/users/:id", |ctx| async move {
            Ok(Response::text(format!("user {}", ctx.param("id")?)))
        })?
        .get("/health", |_ctx| async {
            Response::json(&serde_json::json!({ "ok": true }))
        })
}
