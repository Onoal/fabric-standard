use std::sync::{Arc, Mutex};

use fabric_package_key_value::memory_key_value;
use fabric_standard_http::{App, Error, Method, Request, Response, TestRuntime};
use futures::executor::block_on;

fn text(response: &Response) -> String {
    String::from_utf8(response.body().to_vec()).expect("utf8 response")
}

#[test]
fn get_root_returns_text() {
    let response = block_on(
        App::new()
            .get("/", |_ctx| async { Ok(Response::text("hello")) })
            .expect("route")
            .request(Request::get("/")),
    )
    .expect("response");

    assert_eq!(response.status(), 200);
    assert_eq!(text(&response), "hello");
}

#[test]
fn pure_prepared_app_remains_valid() {
    let prepared = App::new()
        .get("/", |_ctx| async { Ok(Response::text("prepared")) })
        .expect("route")
        .prepare()
        .expect("prepare");

    let response = block_on(prepared.request(Request::get("/"))).expect("response");
    assert_eq!(text(&response), "prepared");
}

#[test]
fn resource_bound_prepare_and_request_reject_explicitly() {
    let prepare_result = App::new()
        .with_fabric(memory_key_value("sessions"))
        .use_key_value("sessions")
        .expect("key value")
        .get("/", |ctx| async move {
            let _ = ctx.key_value()?;
            Ok(Response::text("resource"))
        })
        .expect("route")
        .prepare();
    let Err(prepare_error) = prepare_result else {
        panic!("resource-bound prepare was accepted");
    };

    assert!(matches!(
        prepare_error,
        Error::RequiresFabricBoundExecution("KeyValue")
    ));
    assert!(prepare_error.to_string().contains("App::test_runtime()"));

    let request_error = block_on(
        App::new()
            .with_fabric(memory_key_value("sessions"))
            .use_key_value("sessions")
            .expect("key value")
            .get("/", |ctx| async move {
                let _ = ctx.key_value()?;
                Ok(Response::text("resource"))
            })
            .expect("route")
            .request(Request::get("/")),
    )
    .expect_err("resource-bound request");

    assert!(matches!(
        request_error,
        Error::RequiresFabricBoundExecution("KeyValue")
    ));
}

#[test]
fn plain_test_runtime_dispatches_without_fabric_resources() {
    let runtime: TestRuntime = App::new()
        .get("/", |_ctx| async { Ok(Response::text("plain runtime")) })
        .expect("route")
        .test_runtime()
        .expect("runtime");

    let response = block_on(runtime.request(Request::get("/"))).expect("response");
    assert_eq!(text(&response), "plain runtime");
    runtime.stop().expect("stop");
}

#[test]
fn key_value_test_runtime_uses_real_fabric_and_preserves_state() {
    let app = App::new()
        .with_fabric(memory_key_value("sessions"))
        .use_key_value("sessions")
        .expect("key value")
        .middleware(|ctx, next| async move {
            let mut response = next.run(ctx).await?;
            response.append_header("x-runtime", "test")?;
            Ok(response)
        })
        .expect("middleware")
        .post("/sessions/:id", |ctx| async move {
            let key = ctx.param("id")?.to_owned();
            ctx.key_value()?.set(key, b"active".to_vec())?;
            Ok(Response::text("stored"))
        })
        .expect("post")
        .get("/sessions/:id", |ctx| async move {
            let key = ctx.param("id")?.to_owned();
            let value = ctx.key_value()?.get(key)?;
            Ok(Response::text(
                value
                    .map(|value| String::from_utf8_lossy(&value).into_owned())
                    .unwrap_or_else(|| "missing".to_owned()),
            ))
        })
        .expect("get");

    let runtime = app.test_runtime().expect("runtime");

    let stored =
        block_on(runtime.request(Request::post("/sessions/abc", Vec::new()))).expect("stored");
    assert_eq!(text(&stored), "stored");
    assert_eq!(
        stored
            .headers()
            .iter()
            .find(|header| header.name == "x-runtime")
            .map(|header| header.value.as_str()),
        Some("test")
    );

    let loaded = block_on(runtime.request(Request::get("/sessions/abc"))).expect("loaded");
    assert_eq!(text(&loaded), "active");

    runtime.stop().expect("stop");
}

#[test]
fn path_params_are_exposed() {
    let response = block_on(
        App::new()
            .get("/users/:id", |ctx| async move {
                Ok(Response::text(format!("id={}", ctx.param("id")?)))
            })
            .expect("route")
            .request(Request::get("/users/42")),
    )
    .expect("response");

    assert_eq!(text(&response), "id=42");
}

#[test]
fn registration_order_defines_static_param_precedence() {
    let static_first = block_on(
        App::new()
            .get("/users/me", |_ctx| async { Ok(Response::text("static")) })
            .expect("static")
            .get("/users/:id", |_ctx| async { Ok(Response::text("param")) })
            .expect("param")
            .request(Request::get("/users/me")),
    )
    .expect("response");
    assert_eq!(text(&static_first), "static");

    let param_first = block_on(
        App::new()
            .get("/users/:id", |_ctx| async { Ok(Response::text("param")) })
            .expect("param")
            .get("/users/me", |_ctx| async { Ok(Response::text("static")) })
            .expect("static")
            .request(Request::get("/users/me")),
    )
    .expect("response");
    assert_eq!(text(&param_first), "param");
}

#[test]
fn duplicate_route_rejects() {
    let result = App::new()
        .get("/", |_ctx| async { Ok(Response::text("a")) })
        .expect("first")
        .get("/", |_ctx| async { Ok(Response::text("b")) });
    let Err(error) = result else {
        panic!("duplicate route was accepted");
    };
    assert!(matches!(error, Error::Authoring(_)));
}

#[test]
fn wildcard_and_trailing_slash_rules_work() {
    let wildcard = block_on(
        App::new()
            .get("/files/*path", |ctx| async move {
                Ok(Response::text(ctx.param("path")?.to_owned()))
            })
            .expect("route")
            .request(Request::get("/files/a/b")),
    )
    .expect("response");
    assert_eq!(text(&wildcard), "a/b");

    let slash = block_on(
        App::new()
            .get("/trail/", |_ctx| async { Ok(Response::text("slash")) })
            .expect("route")
            .request(Request::get("/trail")),
    )
    .expect("response");
    assert_eq!(slash.status(), 404);
}

#[test]
fn params_are_percent_decoded_and_invalid_encoding_errors() {
    let decoded = block_on(
        App::new()
            .get("/users/:id", |ctx| async move {
                Ok(Response::text(ctx.param("id")?.to_owned()))
            })
            .expect("route")
            .request(Request::get("/users/Ada%20Lovelace")),
    )
    .expect("decoded");
    assert_eq!(text(&decoded), "Ada Lovelace");

    let error = block_on(
        App::new()
            .get("/users/:id", |ctx| async move {
                Ok(Response::text(ctx.param("id")?.to_owned()))
            })
            .expect("route")
            .request(Request::get("/users/%zz")),
    )
    .expect_err("invalid percent");
    assert!(matches!(error, Error::Dispatch(_)));
}

#[test]
fn query_access_preserves_repeated_values() {
    let request = Request::get("/search?tag=a&tag=b&empty");
    assert_eq!(request.query_first("tag"), Some("a"));
    assert_eq!(request.query_values("tag"), vec!["a", "b"]);
    assert_eq!(request.query_values("missing"), Vec::<&str>::new());
    assert_eq!(request.query_first("empty"), Some(""));
}

#[test]
fn request_repeated_headers_are_preserved() {
    let request = Request::get("/headers")
        .with_header("x-test", "a")
        .with_header("x-test", "b");
    assert_eq!(request.header("x-test"), Some("a"));
    assert_eq!(request.headers_all("x-test"), vec!["a", "b"]);
}

#[test]
fn json_sets_content_type() {
    let response = Response::json(&serde_json::json!({ "ok": true })).expect("json");
    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case("content-type"))
            .map(|header| header.value.as_str()),
        Some("application/json")
    );
}

#[test]
fn response_bytes_status_header_and_redirect_work() {
    let response = Response::bytes(b"bytes".to_vec())
        .with_status(201)
        .with_header("x-test", "ok");
    assert_eq!(response.status(), 201);
    assert_eq!(response.body(), b"bytes");
    assert_eq!(response.headers()[0].name, "x-test");

    let redirect = Response::redirect("/next");
    assert_eq!(redirect.status(), 302);
    assert_eq!(
        redirect
            .headers()
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case("location"))
            .map(|header| header.value.as_str()),
        Some("/next")
    );
}

#[test]
fn middleware_order_route_middleware_short_circuit_and_next_misuse_work() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let first = Arc::clone(&events);
    let second = Arc::clone(&events);
    let route = Arc::clone(&events);

    let response = block_on(
        App::new()
            .middleware(move |ctx, next| {
                let first = Arc::clone(&first);
                async move {
                    first.lock().expect("events").push("global-before");
                    let response = next.run(ctx).await?;
                    first.lock().expect("events").push("global-after");
                    Ok(response)
                }
            })
            .expect("middleware")
            .middleware(move |ctx, next| {
                let second = Arc::clone(&second);
                async move {
                    second.lock().expect("events").push("second-before");
                    let response = next.run(ctx).await?;
                    second.lock().expect("events").push("second-after");
                    Ok(response)
                }
            })
            .expect("middleware")
            .get("/", |_ctx| async { Ok(Response::text("handler")) })
            .expect("route")
            .middleware_for(Method::Get, "/", move |ctx, next| {
                let route = Arc::clone(&route);
                async move {
                    route.lock().expect("events").push("route-before");
                    let response = next.run(ctx).await?;
                    route.lock().expect("events").push("route-after");
                    Ok(response)
                }
            })
            .expect("route middleware")
            .request(Request::get("/")),
    )
    .expect("response");

    assert_eq!(text(&response), "handler");
    assert_eq!(
        events.lock().expect("events").as_slice(),
        [
            "global-before",
            "second-before",
            "route-before",
            "route-after",
            "second-after",
            "global-after",
        ]
    );

    let short = block_on(
        App::new()
            .middleware(|_ctx, _next| async { Ok(Response::text("short")) })
            .expect("middleware")
            .get("/", |_ctx| async { Ok(Response::text("handler")) })
            .expect("route")
            .request(Request::get("/")),
    )
    .expect("short");
    assert_eq!(text(&short), "short");

    let misuse = block_on(
        App::new()
            .middleware(|ctx, next| async move {
                let _ = next.run(ctx.clone()).await?;
                next.run(ctx).await
            })
            .expect("middleware")
            .get("/", |_ctx| async { Ok(Response::text("handler")) })
            .expect("route")
            .request(Request::get("/")),
    )
    .expect_err("next misuse");
    assert!(matches!(misuse, Error::MiddlewareMisuse(_)));
}

#[test]
fn not_found_head_options_and_sub_apps_work() {
    let default_404 = block_on(App::new().request(Request::get("/missing"))).expect("404");
    assert_eq!(default_404.status(), 404);

    let custom = block_on(
        App::new()
            .not_found(|_ctx| async { Ok(Response::text("custom").with_status(404)) })
            .request(Request::get("/missing")),
    )
    .expect("custom");
    assert_eq!(text(&custom), "custom");

    let explicit_head = block_on(
        App::new()
            .get("/health", |_ctx| async { Ok(Response::text("get")) })
            .expect("get")
            .head("/health", |_ctx| async { Ok(Response::text("head")) })
            .expect("head")
            .request(Request::new(Method::Head, "/health", Vec::new())),
    )
    .expect("head");
    assert_eq!(text(&explicit_head), "head");

    let fallback_head = block_on(
        App::new()
            .get("/health", |_ctx| async { Ok(Response::text("get")) })
            .expect("get")
            .request(Request::new(Method::Head, "/health", Vec::new())),
    )
    .expect("fallback");
    assert_eq!(fallback_head.status(), 200);
    assert_eq!(fallback_head.body(), b"");

    let options = block_on(
        App::new()
            .get("/health", |_ctx| async { Ok(Response::text("get")) })
            .expect("get")
            .request(Request::new(Method::Options, "/health", Vec::new())),
    )
    .expect("options");
    assert_eq!(options.status(), 404);

    let api = App::new()
        .get("/users/:id", |ctx| async move {
            Ok(Response::text(format!("api:{}", ctx.param("id")?)))
        })
        .expect("api");
    let response = block_on(
        App::new()
            .route("/api", api)
            .expect("sub app")
            .request(Request::get("/api/users/42")),
    )
    .expect("response");
    assert_eq!(text(&response), "api:42");
}
