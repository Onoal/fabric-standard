use std::sync::Arc;

use fabric::prelude::*;

use crate::app::AppParts;
use crate::context::RuntimeResources;
use crate::fabric_bridge::{
    key_value_runtime, StandardHttpKeyValueRuntime, StandardHttpKeyValueRuntimeInstanceApi,
};
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
    pub(crate) fn new(parts: AppParts) -> Result<Self> {
        if parts.fabric.is_empty() && parts.key_value.is_none() {
            return Ok(Self {
                mode: TestRuntimeMode::Pure {
                    app: parts.prepared,
                },
            });
        }

        let prepared = Arc::new(parts.prepared);
        let mut builder = Fabric::new(COMPOSITION_ID)?;
        for contribution in parts.fabric {
            builder = builder.with(contribution);
        }
        let key_value = parts.key_value.is_some();
        if let Some(occurrence) = parts.key_value {
            builder = builder.with(key_value_runtime(occurrence, Arc::clone(&prepared))?);
        }

        let composition = builder.build()?;
        let mut instance = composition.materialize_on(INSTANCE_ID, &HostDescriptor::native())?;
        instance.start()?;

        if key_value {
            let runtime = instance.component::<StandardHttpKeyValueRuntime>()?;
            runtime.reconcile()?;
        }

        Ok(Self {
            mode: TestRuntimeMode::FabricBound {
                app: Arc::unwrap_or_clone(prepared),
                instance: Some(Box::new(instance)),
                key_value,
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
