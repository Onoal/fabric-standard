use fabric_standard_http::{App, Response};
use fabric_standard_http_runtime_native::{serve, ServeConfig};

fn main() -> fabric_standard_http_runtime_native::Result<()> {
    let app = App::new()
        .middleware(|ctx, next| async move {
            let mut response = next.run(ctx).await?;
            response.append_header("x-powered-by", "fabric-standard")?;
            Ok(response)
        })?
        .get("/", |_ctx| async {
            Ok(Response::text("hello from Fabric Standard HTTP"))
        })?
        .get("/users/:id", |ctx| async move {
            Ok(Response::text(format!("user {}", ctx.param("id")?)))
        })?
        .get("/health", |_ctx| async {
            Response::json(&serde_json::json!({ "ok": true }))
        })?;

    serve(app, ServeConfig::local())
}
