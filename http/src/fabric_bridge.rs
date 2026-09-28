use std::sync::Arc;

use fabric::prelude::*;
use fabric_package_key_value::KeyValue;

use crate::context::RuntimeResources;
use crate::{Error, PreparedApp, Request, Response, Result};

#[derive(Clone)]
pub struct StandardHttpKeyValueRuntimeConfig {
    pub(crate) app: Arc<PreparedApp>,
}

fabric::component! {
    pub StandardHttpKeyValueRuntime {
        id: "onoal.fabric-standard.http.runtime.key-value";

        config: StandardHttpKeyValueRuntimeConfig;

        relations {
            requires {
                store: KeyValue;
            }
        }

        api {
            fn dispatch(&self, request: Request) -> Result<Response>;
        }

        runtime {
            fn dispatch(&self, request: Request) -> Result<Response> {
                let resources = RuntimeResources::key_value(self.relations().store.clone());
                self.config().app.dispatch_blocking(request, resources)
            }
        }
    }
}

pub fn key_value_runtime(
    occurrence: &'static str,
    app: Arc<PreparedApp>,
) -> Result<impl IntoFabricContribution> {
    let selected = KeyValue::select(occurrence)
        .map_err(|error| Error::authoring(format!("invalid KeyValue occurrence: {error}")))?;
    let component = StandardHttpKeyValueRuntime::define(StandardHttpKeyValueRuntimeConfig { app })
        .select_named_resource_provider(
            &fabric::authoring::ComponentResourceRequirement::new(
                fabric::component::ComponentRelationName::new("store")
                    .expect("static component relation role"),
                fabric::authoring::Requires::<KeyValue>::provisional(),
            ),
            &selected,
        );
    Ok(FabricContribution::new().component(component))
}
