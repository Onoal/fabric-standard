use std::sync::{Arc, Mutex};

use fabric_package_networking_http::{HttpHeader, HttpRequest, HttpVersion};
use fabric_standard_http::{App, Error, Method, Request, Response};
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
fn request_raw_and_repeated_headers_are_preserved() {
    let raw = HttpRequest {
        method: "GET".to_owned(),
        target: "/headers".to_owned(),
        version: HttpVersion::Http11,
        headers: vec![
            HttpHeader::new("x-test", "a"),
            HttpHeader::new("x-test", "b"),
        ],
        body: Vec::new(),
    };
    let request = Request::from_http(raw).expect("request");
    assert_eq!(request.raw().target, "/headers");
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
