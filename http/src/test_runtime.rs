use std::sync::Arc;

use fabric::prelude::*;

use crate::context::RuntimeResources;
use crate::fabric_bridge::{
    key_value_runtime, StandardHttpKeyValueRuntime, StandardHttpKeyValueRuntimeInstanceApi,
};
use crate::runtime_support::RuntimeDefinition;
use crate::{PreparedApp, Request, Response, Result};

const COMPOSITION_ID: &str = "onoal.fabric-standard.http.test";
const INSTANCE_ID: &str = "onoal.fabric-standard.http.test.instance";

pub struct TestRuntime {
    mode: TestRuntimeMode,
}

enum TestRuntimeMode {
    Pure {
        app: PreparedApp,
    },
    FabricBound {
        app: PreparedApp,
        instance: Option<Box<Instance>>,
        key_value: bool,
    },
}

impl TestRuntime {
    pub(crate) fn new(definition: RuntimeDefinition) -> Result<Self> {
        let (prepared, fabric, key_value) = definition.into_parts();
        if fabric.is_empty() && key_value.is_none() {
            return Ok(Self {
                mode: TestRuntimeMode::Pure { app: prepared },
            });
        }

        let prepared = Arc::new(prepared);
        let mut builder = Fabric::new(COMPOSITION_ID)?;
        for contribution in fabric {
            builder = builder.with(contribution);
        }
        let requires_key_value = key_value.is_some();
        if let Some(occurrence) = key_value {
            builder = builder.with(key_value_runtime(occurrence, Arc::clone(&prepared))?);
        }

        let composition = builder.build()?;
        let mut instance = composition.materialize_on(INSTANCE_ID, &HostDescriptor::native())?;
        instance.start()?;

        if requires_key_value {
            let runtime = instance.component::<StandardHttpKeyValueRuntime>()?;
            runtime.reconcile()?;
        }

        Ok(Self {
            mode: TestRuntimeMode::FabricBound {
                app: Arc::unwrap_or_clone(prepared),
                instance: Some(Box::new(instance)),
                key_value: requires_key_value,
            },
        })
    }

    pub async fn request(&self, request: Request) -> Result<Response> {
        match &self.mode {
            TestRuntimeMode::Pure { app } => app.request(request).await,
            TestRuntimeMode::FabricBound {
                app,
                instance,
                key_value,
            } => {
                if *key_value {
                    let instance = instance
                        .as_ref()
                        .ok_or_else(|| crate::Error::dispatch("TestRuntime has been stopped"))?;
                    let runtime = instance.component::<StandardHttpKeyValueRuntime>()?;
                    runtime.dispatch(request).await?
                } else {
                    app.dispatch(request, RuntimeResources::none()).await
                }
            }
        }
    }

    pub fn stop(mut self) -> Result<()> {
        self.stop_inner()
    }

    fn stop_inner(&mut self) -> Result<()> {
        if let TestRuntimeMode::FabricBound { instance, .. } = &mut self.mode {
            if let Some(instance) = instance {
                instance.stop()?;
            }
            *instance = None;
        }
        Ok(())
    }
}

impl Drop for TestRuntime {
    fn drop(&mut self) {
        let _ = self.stop_inner();
    }
}
