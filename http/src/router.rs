use crate::handler::Handler;
use crate::middleware::Middleware;
use crate::route::{RouteKey, RoutePattern};
use crate::{Error, Method, Request, Result};

#[derive(Clone)]
pub(crate) struct Route {
    pub(crate) method: Method,
    pub(crate) pattern: RoutePattern,
    pub(crate) handler: Handler,
    pub(crate) middleware: Vec<Middleware>,
}

impl Route {
    pub(crate) fn key(&self) -> RouteKey {
        RouteKey::new(self.method.clone(), &self.pattern)
    }
}

#[derive(Clone)]
pub(crate) struct RouteMatch {
    pub(crate) index: usize,
    pub(crate) params: Vec<(String, String)>,
    pub(crate) head_get_fallback: bool,
}

#[derive(Clone)]
pub(crate) struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub(crate) fn new(routes: Vec<Route>) -> Self {
        Self { routes }
    }

    pub(crate) fn routes(&self) -> &[Route] {
        &self.routes
    }

    pub(crate) fn find(&self, request: &Request) -> Result<Option<RouteMatch>> {
        if matches!(request.method(), Method::Head) {
            if let Some(found) = self.find_method(&Method::Head, request.path(), false)? {
                return Ok(Some(found));
            }
            return self.find_method(&Method::Get, request.path(), true);
        }
        self.find_method(request.method(), request.path(), false)
    }

    fn find_method(
        &self,
        method: &Method,
        path: &str,
        head_get_fallback: bool,
    ) -> Result<Option<RouteMatch>> {
        for (index, route) in self.routes.iter().enumerate() {
            if &route.method != method {
                continue;
            }
            if let Some(params) = route.pattern.captures(path)? {
                return Ok(Some(RouteMatch {
                    index,
                    params,
                    head_get_fallback,
                }));
            }
        }
        Ok(None)
    }
}

pub(crate) fn ensure_unique_route(
    routes: &[Route],
    method: &Method,
    pattern: &RoutePattern,
) -> Result<()> {
    if routes
        .iter()
        .any(|route| route.key().matches(method, pattern.original()))
    {
        return Err(Error::authoring(format!(
            "duplicate route {} {}",
            method.as_str(),
            pattern.original()
        )));
    }
    Ok(())
}
