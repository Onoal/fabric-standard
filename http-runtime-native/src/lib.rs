//! Native Fabric runtime for Fabric Standard HTTP applications.

use std::error::Error as StdError;
use std::fmt;
use std::io::{self, Write};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use fabric::component::ComponentError;
use fabric::prelude::*;
use fabric_composition_http_server::{http_server_stack, HttpServerCompositionConfig};
use fabric_package_networking_http::{
    HttpError, HttpHeader, HttpRequest, HttpResponse, HttpServer, HttpServerInstanceApi,
};
use fabric_package_networking_tcp::{
    TcpSocketAddress, TcpTransportInspector, TcpTransportInspectorInstanceApi,
};
use fabric_standard_http::runtime_support::{
    key_value_runtime, RuntimeDefinition, StandardHttpKeyValueRuntime,
    StandardHttpKeyValueRuntimeInstanceApi,
};
use fabric_standard_http::{App, Method, Request, Response};

const COMPOSITION_ID: &str = "onoal.fabric-standard.http.local";
const INSTANCE_ID: &str = "onoal.fabric-standard.http.local.instance";
const TRANSPORT_NAME: &str = "standard-http";

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Standard(fabric_standard_http::Error),
    Application(Box<dyn StdError + Send + Sync>),
    InvalidRequestTarget(String),
    FabricBuild(Box<FabricBuildError>),
    Composition(Box<CompositionError>),
    Instance(Box<InstanceError>),
    RuntimeCleanup(Box<RuntimeCleanupError>),
    Component(Box<ComponentError>),
    Http(Box<HttpError>),
}

impl Error {
    fn application(error: impl StdError + Send + Sync + 'static) -> Self {
        Self::Application(Box::new(error))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standard(error) => write!(f, "Standard HTTP error: {error}"),
            Self::Application(error) => write!(f, "application error: {error}"),
            Self::InvalidRequestTarget(target) => {
                write!(
                    f,
                    "native HTTP request target `{target}` does not contain an absolute path"
                )
            }
            Self::FabricBuild(error) => write!(f, "Fabric build error: {error}"),
            Self::Composition(error) => write!(f, "Fabric composition error: {error}"),
            Self::Instance(error) => write!(f, "Fabric instance error: {error}"),
            Self::RuntimeCleanup(error) => write!(f, "Fabric runtime cleanup error: {error}"),
            Self::Component(error) => write!(f, "Fabric component error: {error}"),
            Self::Http(error) => write!(f, "HTTP package error: {error}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Standard(error) => Some(error),
            Self::Application(error) => Some(error.as_ref()),
            Self::FabricBuild(error) => Some(error.as_ref()),
            Self::Composition(error) => Some(error.as_ref()),
            Self::Instance(error) => Some(error.as_ref()),
            Self::RuntimeCleanup(error) => Some(error.as_ref()),
            Self::Component(error) => Some(error.as_ref()),
            Self::Http(error) => Some(error.as_ref()),
            Self::InvalidRequestTarget(_) => None,
        }
    }
}

impl From<fabric_standard_http::Error> for Error {
    fn from(value: fabric_standard_http::Error) -> Self {
        Self::Standard(value)
    }
}

impl From<FabricBuildError> for Error {
    fn from(value: FabricBuildError) -> Self {
        Self::FabricBuild(Box::new(value))
    }
}

impl From<CompositionError> for Error {
    fn from(value: CompositionError) -> Self {
        Self::Composition(Box::new(value))
    }
}

impl From<InstanceError> for Error {
    fn from(value: InstanceError) -> Self {
        Self::Instance(Box::new(value))
    }
}

impl From<RuntimeCleanupError> for Error {
    fn from(value: RuntimeCleanupError) -> Self {
        Self::RuntimeCleanup(Box::new(value))
    }
}

impl From<ComponentError> for Error {
    fn from(value: ComponentError) -> Self {
        Self::Component(Box::new(value))
    }
}

impl From<HttpError> for Error {
    fn from(value: HttpError) -> Self {
        Self::Http(Box::new(value))
    }
}

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
    let definition = app.into_runtime_definition()?;
    serve_definition(definition, config, None).map(|_| ())
}

fn serve_definition(
    definition: RuntimeDefinition,
    config: ServeConfig,
    max_requests: Option<usize>,
) -> Result<Option<TcpSocketAddress>> {
    serve_definition_with_ready(definition, config, max_requests, None)
}

fn serve_definition_with_ready(
    definition: RuntimeDefinition,
    config: ServeConfig,
    max_requests: Option<usize>,
    ready: Option<Sender<TcpSocketAddress>>,
) -> Result<Option<TcpSocketAddress>> {
    let (prepared, fabric, key_value) = definition.into_parts();
    let prepared = Arc::new(prepared);
    let mut builder = Fabric::new(COMPOSITION_ID)?.with(http_server_stack(
        HttpServerCompositionConfig::bind(TRANSPORT_NAME, config.bind),
    ));
    for contribution in fabric {
        builder = builder.with(contribution);
    }
    if let Some(occurrence) = key_value {
        builder = builder.with(key_value_runtime(occurrence, Arc::clone(&prepared))?);
    }
    let composition = builder.build()?;
    let mut instance = composition.materialize_on(INSTANCE_ID, &HostDescriptor::native())?;
    instance.start()?;

    let inspector = instance.component::<TcpTransportInspector>()?;
    inspector.reconcile()?;
    let address = futures::executor::block_on(inspector.inspect_transport())?
        .actual
        .ok_or_else(|| Error::InvalidRequestTarget("HTTP server did not bind".to_owned()))?;

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
    let key_value_runtime = if key_value.is_some() {
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
        let request = standard_request_from_native(exchange.request().clone())?;
        let response = if let Some(runtime) = &key_value_runtime {
            futures::executor::block_on(runtime.dispatch(request))??
        } else {
            futures::executor::block_on(prepared.request(request))?
        };
        exchange.respond(native_response_from_standard(response))?;
        handled += 1;
    }
}

fn standard_request_from_native(request: HttpRequest) -> Result<Request> {
    let path = request
        .target
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(request.target.as_str());
    if !path.starts_with('/') {
        return Err(Error::InvalidRequestTarget(request.target));
    }
    let mut standard = Request::new(
        Method::from(request.method.as_str()),
        request.target,
        request.body,
    );
    for header in request.headers {
        standard = standard.with_header(header.name, header.value);
    }
    Ok(standard)
}

fn native_response_from_standard(response: Response) -> HttpResponse {
    let mut native = HttpResponse::new(response.status(), response.body().to_vec());
    native.headers = response
        .headers()
        .iter()
        .map(|header| HttpHeader::new(header.name.clone(), header.value.clone()))
        .collect();
    native
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::mpsc;
    use std::thread;

    use fabric_standard_http::{App, Response};

    use super::*;

    #[test]
    fn live_runtime_accepts_multiple_real_http_requests() {
        let app = App::new()
            .get("/", |_ctx| async { Ok(Response::text("hello")) })
            .expect("root")
            .get("/users/:id", |ctx| async move {
                Ok(Response::text(format!("user:{}", ctx.param("id")?)))
            })
            .expect("user");
        let definition = app.into_runtime_definition().expect("definition");
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            serve_definition_with_ready(
                definition,
                ServeConfig::loopback_ephemeral(),
                Some(2),
                Some(tx),
            )
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
