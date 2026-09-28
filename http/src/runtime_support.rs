//! Adapter-facing support for Standard HTTP runtime implementations.
//!
//! This module is for crates that execute Standard HTTP applications in a
//! concrete runtime. It intentionally exposes prepared dispatch and Fabric
//! authoring facts without exposing router, middleware, or handler internals.

use fabric::prelude::FabricContribution;

use crate::PreparedApp;

pub use crate::fabric_bridge::{
    key_value_runtime, StandardHttpKeyValueRuntime, StandardHttpKeyValueRuntimeInstanceApi,
};

pub struct RuntimeDefinition {
    pub(crate) prepared: PreparedApp,
    pub(crate) fabric: Vec<FabricContribution>,
    pub(crate) key_value: Option<&'static str>,
}

impl RuntimeDefinition {
    pub fn prepared(&self) -> &PreparedApp {
        &self.prepared
    }

    pub fn key_value_occurrence(&self) -> Option<&'static str> {
        self.key_value
    }

    pub fn into_parts(self) -> (PreparedApp, Vec<FabricContribution>, Option<&'static str>) {
        (self.prepared, self.fabric, self.key_value)
    }
}
