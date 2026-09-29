use fabric::prelude::{FabricContribution, IntoFabricContribution};

use crate::handler::{handler, Handler};
use crate::middleware::{middleware, Middleware};
use crate::prepared::{default_not_found, PreparedApp};
use crate::route::RoutePattern;
use crate::router::{ensure_unique_route, Route, Router};
use crate::runtime_support::RuntimeDefinition;
use crate::test_runtime::TestRuntime;
use crate::{Context, Error, Method, Response, Result};

pub struct App {
    routes: Vec<Route>,
    global_middleware: Vec<Middleware>,
    not_found: Handler,
    fabric: Vec<FabricContribution>,
    key_value: Option<&'static str>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            global_middleware: Vec::new(),
            not_found: default_not_found(),
            fabric: Vec::new(),
            key_value: None,
        }
    }

    pub fn get<F, Fut>(self, path: &str, handler: F) -> Result<Self>
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Response>> + 'static,
    {
        self.route_method(Method::Get, path, handler)
    }

    pub fn post<F, Fut>(self, path: &str, handler: F) -> Result<Self>
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Response>> + 'static,
    {
        self.route_method(Method::Post, path, handler)
    }

    pub fn put<F, Fut>(self, path: &str, handler: F) -> Result<Self>
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Response>> + 'static,
    {
        self.route_method(Method::Put, path, handler)
    }

    pub fn patch<F, Fut>(self, path: &str, handler: F) -> Result<Self>
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Response>> + 'static,
    {
        self.route_method(Method::Patch, path, handler)
    }

    pub fn delete<F, Fut>(self, path: &str, handler: F) -> Result<Self>
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Response>> + 'static,
    {
        self.route_method(Method::Delete, path, handler)
    }

    pub fn options<F, Fut>(self, path: &str, handler: F) -> Result<Self>
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Response>> + 'static,
    {
        self.route_method(Method::Options, path, handler)
    }

    pub fn head<F, Fut>(self, path: &str, handler: F) -> Result<Self>
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Response>> + 'static,
    {
        self.route_method(Method::Head, path, handler)
    }

    pub fn middleware<F, Fut>(mut self, middleware_fn: F) -> Result<Self>
    where
        F: Fn(Context, crate::Next) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Response>> + 'static,
    {
        self.global_middleware.push(middleware(middleware_fn));
        Ok(self)
    }

    pub fn middleware_for<F, Fut>(
        mut self,
        method: Method,
        path: &str,
        middleware_fn: F,
    ) -> Result<Self>
    where
        F: Fn(Context, crate::Next) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Response>> + 'static,
    {
        let pattern = RoutePattern::parse(path)?;
        let route = self
            .routes
            .iter_mut()
            .find(|route| route.method == method && route.pattern.original() == pattern.original())
            .ok_or_else(|| {
                Error::authoring(format!(
                    "no route exists for {} {}",
                    method.as_str(),
                    pattern.original()
                ))
            })?;
        route.middleware.push(middleware(middleware_fn));
        Ok(self)
    }

    pub fn not_found<F, Fut>(mut self, handler_fn: F) -> Self
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Response>> + 'static,
    {
        self.not_found = handler(handler_fn);
        self
    }

    pub fn route(mut self, prefix: &str, mut child: App) -> Result<Self> {
        let prefix_pattern = RoutePattern::parse(prefix)?;
        if let (Some(left), Some(right)) = (self.key_value, child.key_value) {
            if left != right {
                return Err(Error::authoring(format!(
                    "conflicting KeyValue bindings `{left}` and `{right}`"
                )));
            }
        }
        if self.key_value.is_none() {
            self.key_value = child.key_value.take();
        }
        self.fabric.append(&mut child.fabric);
        for mut route in child.routes {
            route.pattern = route.pattern.prefixed(prefix_pattern.original())?;
            let child_globals = child.global_middleware.clone();
            let mut combined = child_globals;
            combined.append(&mut route.middleware);
            route.middleware = combined;
            ensure_unique_route(&self.routes, &route.method, &route.pattern)?;
            self.routes.push(route);
        }
        Ok(self)
    }

    pub fn with_fabric(mut self, contribution: impl IntoFabricContribution) -> Self {
        self.fabric.push(contribution.into_fabric_contribution());
        self
    }

    pub fn use_key_value(mut self, occurrence: &'static str) -> Result<Self> {
        if let Some(existing) = self.key_value {
            if existing != occurrence {
                return Err(Error::authoring(format!(
                    "only one KeyValue binding is supported in M2A (`{existing}` already declared)"
                )));
            }
        }
        self.key_value = Some(occurrence);
        Ok(self)
    }

    pub fn prepare(self) -> Result<PreparedApp> {
        let definition = self.into_runtime_definition()?;
        if definition.key_value_occurrence().is_some() {
            return Err(Error::RequiresFabricBoundExecution("KeyValue"));
        }
        let (prepared, _, _) = definition.into_parts();
        Ok(prepared)
    }

    pub async fn request(self, request: crate::Request) -> Result<Response> {
        self.prepare()?.request(request).await
    }

    pub fn test_runtime(self) -> Result<TestRuntime> {
        TestRuntime::new(self.into_runtime_definition()?)
    }

    pub fn into_runtime_definition(self) -> Result<RuntimeDefinition> {
        Ok(RuntimeDefinition {
            prepared: PreparedApp::new(
                Router::new(self.routes),
                self.global_middleware,
                self.not_found,
            ),
            fabric: self.fabric,
            key_value: self.key_value,
        })
    }

    fn route_method<F, Fut>(mut self, method: Method, path: &str, handler_fn: F) -> Result<Self>
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Response>> + 'static,
    {
        let pattern = RoutePattern::parse(path)?;
        ensure_unique_route(&self.routes, &method, &pattern)?;
        self.routes.push(Route {
            method,
            pattern,
            handler: handler(handler_fn),
            middleware: Vec::new(),
        });
        Ok(self)
    }
}
