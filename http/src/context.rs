use std::collections::BTreeMap;
use std::sync::Arc;

use fabric::authoring::RelationTarget;
use fabric_package_key_value::{KeyValue, KeyValueError};

use crate::{Error, Request, Result};

pub(crate) type KeyValueContract = <KeyValue as RelationTarget>::Contract;

#[derive(Clone)]
pub(crate) struct RuntimeResources {
    key_value: Option<Arc<KeyValueContract>>,
}

impl RuntimeResources {
    pub(crate) fn none() -> Self {
        Self { key_value: None }
    }

    pub(crate) fn key_value(key_value: Arc<KeyValueContract>) -> Self {
        Self {
            key_value: Some(key_value),
        }
    }
}

#[derive(Clone)]
pub struct KeyValueHandle {
    inner: Arc<KeyValueContract>,
}

impl KeyValueHandle {
    pub fn get(&self, key: String) -> std::result::Result<Option<Vec<u8>>, KeyValueError> {
        self.inner.get(key)
    }

    pub fn set(&self, key: String, value: Vec<u8>) -> std::result::Result<(), KeyValueError> {
        self.inner.set(key, value)
    }

    pub fn delete(&self, key: String) -> std::result::Result<Option<Vec<u8>>, KeyValueError> {
        self.inner.delete(key)
    }
}

#[derive(Clone)]
pub struct Context {
    request: Request,
    params: BTreeMap<String, String>,
    resources: RuntimeResources,
}

impl Context {
    pub(crate) fn new(
        request: Request,
        params: Vec<(String, String)>,
        resources: RuntimeResources,
    ) -> Self {
        Self {
            request,
            params: params.into_iter().collect(),
            resources,
        }
    }

    pub fn request(&self) -> &Request {
        &self.request
    }

    pub fn param(&self, name: &str) -> Result<&str> {
        self.params
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| Error::dispatch(format!("route parameter `{name}` was not captured")))
    }

    pub fn key_value(&self) -> Result<KeyValueHandle> {
        self.resources
            .key_value
            .as_ref()
            .map(|inner| KeyValueHandle {
                inner: Arc::clone(inner),
            })
            .ok_or(Error::ResourceUnavailable("KeyValue"))
    }
}
