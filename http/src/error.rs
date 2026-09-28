use std::error::Error as StdError;
use std::fmt;

use fabric::component::ComponentError;
use fabric::prelude::{CompositionError, FabricBuildError, InstanceError, RuntimeCleanupError};
use fabric_package_key_value::KeyValueError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Authoring(String),
    Dispatch(String),
    MiddlewareMisuse(String),
    Application(Box<dyn StdError + Send + Sync>),
    Json(serde_json::Error),
    FabricBuild(Box<FabricBuildError>),
    Composition(Box<CompositionError>),
    Instance(Box<InstanceError>),
    RuntimeCleanup(Box<RuntimeCleanupError>),
    Component(Box<ComponentError>),
    KeyValue(Box<KeyValueError>),
    ResourceUnavailable(&'static str),
    RequiresFabricBoundExecution(&'static str),
}

impl Error {
    pub fn application(error: impl StdError + Send + Sync + 'static) -> Self {
        Self::Application(Box::new(error))
    }

    pub(crate) fn authoring(message: impl Into<String>) -> Self {
        Self::Authoring(message.into())
    }

    pub(crate) fn dispatch(message: impl Into<String>) -> Self {
        Self::Dispatch(message.into())
    }

    pub(crate) fn middleware(message: impl Into<String>) -> Self {
        Self::MiddlewareMisuse(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authoring(message) => write!(f, "HTTP authoring error: {message}"),
            Self::Dispatch(message) => write!(f, "HTTP dispatch error: {message}"),
            Self::MiddlewareMisuse(message) => write!(f, "middleware misuse: {message}"),
            Self::Application(error) => write!(f, "application error: {error}"),
            Self::Json(error) => write!(f, "JSON error: {error}"),
            Self::FabricBuild(error) => write!(f, "Fabric build error: {error}"),
            Self::Composition(error) => write!(f, "Fabric composition error: {error}"),
            Self::Instance(error) => write!(f, "Fabric instance error: {error}"),
            Self::RuntimeCleanup(error) => write!(f, "Fabric runtime cleanup error: {error}"),
            Self::Component(error) => write!(f, "Fabric component error: {error}"),
            Self::KeyValue(error) => write!(f, "KeyValue error: {error}"),
            Self::ResourceUnavailable(name) => write!(f, "resource unavailable: {name}"),
            Self::RequiresFabricBoundExecution(name) => write!(
                f,
                "{name}-backed HTTP apps require Fabric-bound execution; use App::test_runtime() for socket-free tests or a runtime-specific serve() for live HTTP transport"
            ),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Application(error) => Some(error.as_ref()),
            Self::Json(error) => Some(error),
            Self::FabricBuild(error) => Some(error.as_ref()),
            Self::Composition(error) => Some(error.as_ref()),
            Self::Instance(error) => Some(error.as_ref()),
            Self::RuntimeCleanup(error) => Some(error.as_ref()),
            Self::Component(error) => Some(error.as_ref()),
            Self::KeyValue(error) => Some(error.as_ref()),
            Self::Authoring(_)
            | Self::Dispatch(_)
            | Self::MiddlewareMisuse(_)
            | Self::ResourceUnavailable(_)
            | Self::RequiresFabricBoundExecution(_) => None,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
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

impl From<KeyValueError> for Error {
    fn from(value: KeyValueError) -> Self {
        Self::KeyValue(Box::new(value))
    }
}
