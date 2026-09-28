use std::io::{self, Write};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use fabric::prelude::*;
use fabric_composition_http_server::{http_server_stack, HttpServerCompositionConfig};
use fabric_package_networking_http::{HttpServer, HttpServerInstanceApi};
use fabric_package_networking_tcp::{
    TcpSocketAddress, TcpTransportInspector, TcpTransportInspectorInstanceApi,
};

use crate::app::AppParts;
use crate::context::RuntimeResources;
use crate::fabric_bridge::{
    key_value_runtime, key_value_runtime_api, StandardHttpKeyValueRuntime,
    StandardHttpKeyValueRuntimeInstanceApi,
};
use crate::{App, Error, Request, Result};

const COMPOSITION_ID: &str = "onoal.fabric-standard.http.local";
const INSTANCE_ID: &str = "onoal.fabric-standard.http.local.instance";
const TRANSPORT_NAME: &str = "standard-http";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServeConfig {
    pub bind: TcpSocketAddress,
}

impl ServeConfig {
    pub fn local() -> Self {
        Self {
            bind: TcpSocketAddress {
                host: "127.0.0.1".to_owned(),
                port: 3000,
            },
        }
    }

    pub fn bind(bind: TcpSocketAddress) -> Self {
        Self { bind }
    }

    pub fn loopback_ephemeral() -> Self {
        Self {
            bind: TcpSocketAddress::loopback_ephemeral(),
        }
    }
}

pub fn serve(app: App, config: ServeConfig) -> Result<()> {
    let parts = app.into_parts()?;
    serve_parts(parts, config, None).map(|_| ())
}

pub(crate) fn serve_parts(
    parts: AppParts,
    config: ServeConfig,
    max_requests: Option<usize>,
) -> Result<Option<TcpSocketAddress>> {
    serve_parts_with_ready(parts, config, max_requests, None)
}

fn serve_parts_with_ready(
    parts: AppParts,
    config: ServeConfig,
    max_requests: Option<usize>,
    ready: Option<Sender<TcpSocketAddress>>,
) -> Result<Option<TcpSocketAddress>> {
    let prepared = Arc::new(parts.prepared);
    let mut builder = Fabric::new(COMPOSITION_ID)?.with(http_server_stack(
        HttpServerCompositionConfig::bind(TRANSPORT_NAME, config.bind),
    ));
    for contribution in parts.fabric {
        builder = builder.with(contribution);
    }
    if let Some(occurrence) = parts.key_value {
        builder = builder.with(key_value_runtime(occurrence, Arc::clone(&prepared))?);
    }
    let composition = builder.build()?;
    let mut instance = composition.materialize_on(INSTANCE_ID, &HostDescriptor::native())?;
    instance.start()?;

    let inspector = instance.component::<TcpTransportInspector>()?;
    inspector.reconcile()?;
    let address = futures::executor::block_on(inspector.inspect_transport())?
        .actual
        .ok_or_else(|| Error::dispatch("HTTP server did not bind a TCP address"))?;

    println!(
        "Standard HTTP listening on http://{}:{}",
        address.host, address.port
    );
    io::stdout().flush().map_err(Error::application)?;
    if let Some(ready) = ready {
        let _ = ready.send(address.clone());
    }

    let server = instance.component::<HttpServer>()?;
    server.reconcile()?;
    let key_value_runtime = if parts.key_value.is_some() {
        let runtime = instance.component::<StandardHttpKeyValueRuntime>()?;
        runtime.reconcile()?;
        Some(runtime)
    } else {
        None
    };

    let mut handled = 0usize;
    loop {
        if max_requests.is_some_and(|limit| handled >= limit) {
            instance.stop()?;
            return Ok(Some(address));
        }
        let exchange = futures::executor::block_on(server.accept_exchange())??;
        let request = Request::from_http(exchange.request().clone())?;
        let response = if let Some(runtime) = &key_value_runtime {
            futures::executor::block_on(runtime.dispatch(request))??
        } else {
            futures::executor::block_on(prepared.dispatch(request, RuntimeResources::none()))?
        };
        exchange.respond(response.into_http())?;
        handled += 1;
    }
}

#[allow(dead_code)]
pub(crate) fn key_value_operation_id() -> fabric::component::OperationId {
    key_value_runtime_api::dispatch_id()
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::mpsc;
    use std::thread;

    use fabric_package_key_value::memory_key_value;

    use super::*;
    use crate::{App, Response};

    fn response_body(response: &crate::Response) -> String {
        String::from_utf8(response.body().to_vec()).expect("utf8")
    }

    #[test]
    fn resource_handler_uses_real_fabric_key_value_without_tcp() {
        let app = App::new()
            .with_fabric(memory_key_value("sessions"))
            .use_key_value("sessions")
            .expect("key value")
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
                        .as_deref()
                        .map(String::from_utf8_lossy)
                        .map(|value| value.into_owned())
                        .unwrap_or_else(|| "missing".to_owned()),
                ))
            })
            .expect("get");

        let parts = app.into_parts().expect("parts");
        let prepared = Arc::new(parts.prepared);
        let occurrence = parts.key_value.expect("key value occurrence");
        let composition = Fabric::new("onoal.fabric-standard.http.test.resource")
            .expect("fabric")
            .with(
                parts
                    .fabric
                    .into_iter()
                    .fold(FabricContribution::new(), |acc, contribution| {
                        acc.with(contribution)
                    }),
            )
            .with(key_value_runtime(occurrence, Arc::clone(&prepared)).expect("runtime"))
            .build()
            .expect("composition");
        let mut instance = composition
            .materialize_on(
                "onoal.fabric-standard.http.test.resource.instance",
                &HostDescriptor::native(),
            )
            .expect("instance");
        instance.start().expect("start");
        let runtime = instance
            .component::<StandardHttpKeyValueRuntime>()
            .expect("runtime");
        runtime.reconcile().expect("reconcile");

        let stored = futures::executor::block_on(
            runtime.dispatch(Request::post("/sessions/abc", Vec::new())),
        )
        .expect("component")
        .expect("standard");
        assert_eq!(response_body(&stored), "stored");

        let read = futures::executor::block_on(runtime.dispatch(Request::get("/sessions/abc")))
            .expect("component")
            .expect("standard");
        assert_eq!(response_body(&read), "active");

        instance.stop().expect("stop");
    }

    #[test]
    fn plain_app_has_no_key_value_requirement() {
        let parts = App::new()
            .get("/", |_ctx| async { Ok(Response::text("plain")) })
            .expect("route")
            .into_parts()
            .expect("parts");
        assert!(parts.key_value.is_none());
    }

    #[test]
    fn live_runtime_accepts_multiple_real_http_requests() {
        let app = App::new()
            .get("/", |_ctx| async { Ok(Response::text("hello")) })
            .expect("root")
            .get("/users/:id", |ctx| async move {
                Ok(Response::text(format!("user:{}", ctx.param("id")?)))
            })
            .expect("user");
        let parts = app.into_parts().expect("parts");
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            serve_parts_with_ready(parts, ServeConfig::loopback_ephemeral(), Some(2), Some(tx))
                .expect("serve")
        });
        let address = rx.recv().expect("ready");
        assert_eq!(http_get(&address, "/"), "hello");
        assert_eq!(http_get(&address, "/users/42"), "user:42");
        handle.join().expect("thread");
    }

    fn http_get(address: &TcpSocketAddress, path: &str) -> String {
        let mut stream =
            TcpStream::connect((address.host.as_str(), address.port)).expect("connect");
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
            address.host, address.port
        )
        .expect("request");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("response");
        response.split("\r\n\r\n").nth(1).expect("body").to_owned()
    }
}
