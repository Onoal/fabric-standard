//! Cloudflare Workers runtime for Fabric Standard HTTP applications.

use std::error::Error as StdError;
use std::fmt;

use fabric_standard_http::runtime_support::RuntimeDefinition;
use fabric_standard_http::{App, Method, PreparedApp, Request, Response};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Standard(fabric_standard_http::Error),
    Worker(worker::Error),
    UnsupportedFabricContributions(usize),
    UnsupportedKeyValue(&'static str),
}

impl Error {
    fn into_worker_error(self) -> worker::Error {
        worker::Error::RustError(self.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standard(error) => write!(f, "Standard HTTP error: {error}"),
            Self::Worker(error) => write!(f, "Cloudflare Worker error: {error}"),
            Self::UnsupportedFabricContributions(count) => write!(
                f,
                "Cloudflare runtime does not yet support Fabric materialization; {count} Fabric contribution(s) were declared"
            ),
            Self::UnsupportedKeyValue(occurrence) => write!(
                f,
                "Cloudflare runtime does not yet support KeyValue occurrence `{occurrence}`; Cloudflare KV is not implemented as a Fabric realization in M3C"
            ),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Standard(error) => Some(error),
            Self::Worker(error) => Some(error),
            Self::UnsupportedFabricContributions(_) | Self::UnsupportedKeyValue(_) => None,
        }
    }
}

impl From<fabric_standard_http::Error> for Error {
    fn from(value: fabric_standard_http::Error) -> Self {
        Self::Standard(value)
    }
}

impl From<worker::Error> for Error {
    fn from(value: worker::Error) -> Self {
        Self::Worker(value)
    }
}

#[derive(Clone)]
pub struct CloudflareApp {
    app: PreparedApp,
}

impl CloudflareApp {
    pub fn new(app: App) -> Result<Self> {
        Self::from_runtime_definition(app.into_runtime_definition()?)
    }

    pub fn from_runtime_definition(definition: RuntimeDefinition) -> Result<Self> {
        let (prepared, fabric, key_value) = definition.into_parts();
        if let Some(occurrence) = key_value {
            return Err(Error::UnsupportedKeyValue(occurrence));
        }
        if !fabric.is_empty() {
            return Err(Error::UnsupportedFabricContributions(fabric.len()));
        }
        Ok(Self { app: prepared })
    }

    pub async fn fetch(&self, request: worker::Request) -> worker::Result<worker::Response> {
        self.try_fetch(request)
            .await
            .map_err(Error::into_worker_error)
    }

    pub async fn try_fetch(&self, request: worker::Request) -> Result<worker::Response> {
        let request = standard_request_from_worker(request).await?;
        let response = self.app.request(request).await?;
        worker_response_from_standard(response)
    }
}

async fn standard_request_from_worker(mut request: worker::Request) -> Result<Request> {
    let method = request.inner().method();
    let url = request.url()?;
    let mut target = url.path().to_owned();
    if let Some(query) = url.query() {
        target.push('?');
        target.push_str(query);
    }
    let headers: Vec<(String, String)> = request.headers().entries().collect();
    let body = request.bytes().await?;

    let mut standard = Request::new(Method::from(method.as_str()), target, body);
    for (name, value) in headers {
        standard = standard.with_header(name, value);
    }
    Ok(standard)
}

fn worker_response_from_standard(response: Response) -> Result<worker::Response> {
    let headers = worker::Headers::new();
    for header in response.headers() {
        headers.append(&header.name, &header.value)?;
    }
    Ok(worker::Response::builder()
        .with_status(response.status())
        .with_headers(headers)
        .fixed(response.body().to_vec()))
}

#[cfg(test)]
mod tests {
    use fabric_package_key_value::memory_key_value;
    use fabric_standard_http::{App, Response};

    use super::*;

    #[test]
    fn pure_app_can_be_prepared_for_cloudflare() {
        let runtime = CloudflareApp::new(
            App::new()
                .get("/", |_ctx| async { Ok(Response::text("hello")) })
                .expect("route"),
        )
        .expect("cloudflare runtime");

        let _ = runtime;
    }

    #[test]
    fn fabric_contributions_are_rejected_explicitly() {
        let result = CloudflareApp::new(
            App::new()
                .with_fabric(memory_key_value("sessions"))
                .get("/", |_ctx| async { Ok(Response::text("hello")) })
                .expect("route"),
        );
        let Err(error) = result else {
            panic!("unsupported Fabric contribution was accepted");
        };

        assert!(matches!(error, Error::UnsupportedFabricContributions(1)));
    }

    #[test]
    fn key_value_requirements_are_rejected_explicitly() {
        let result = CloudflareApp::new(
            App::new()
                .use_key_value("sessions")
                .expect("key value")
                .get("/", |ctx| async move {
                    let _ = ctx.key_value()?;
                    Ok(Response::text("hello"))
                })
                .expect("route"),
        );
        let Err(error) = result else {
            panic!("unsupported key value requirement was accepted");
        };

        assert!(matches!(error, Error::UnsupportedKeyValue("sessions")));
    }
}
