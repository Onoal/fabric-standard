use std::sync::Arc;

use crate::context::RuntimeResources;
use crate::handler::Handler;
use crate::middleware::Next;
use crate::router::Router;
use crate::{Context, Request, Response, Result};

#[derive(Clone)]
pub struct PreparedApp {
    pub(crate) inner: Arc<PreparedInner>,
}

pub(crate) struct PreparedInner {
    router: Router,
    global_middleware: Vec<crate::middleware::Middleware>,
    not_found: Handler,
}

impl PreparedApp {
    pub(crate) fn new(
        router: Router,
        global_middleware: Vec<crate::middleware::Middleware>,
        not_found: Handler,
    ) -> Self {
        Self {
            inner: Arc::new(PreparedInner {
                router,
                global_middleware,
                not_found,
            }),
        }
    }

    pub async fn request(&self, request: Request) -> Result<Response> {
        self.dispatch(request, RuntimeResources::none()).await
    }

    pub(crate) async fn dispatch(
        &self,
        request: Request,
        resources: RuntimeResources,
    ) -> Result<Response> {
        self.inner.dispatch(request, resources).await
    }

    pub(crate) fn dispatch_blocking(
        &self,
        request: Request,
        resources: RuntimeResources,
    ) -> Result<Response> {
        let app = self.clone();
        std::thread::spawn(move || futures::executor::block_on(app.dispatch(request, resources)))
            .join()
            .map_err(|_| crate::Error::dispatch("HTTP dispatch thread panicked"))?
    }
}

impl PreparedInner {
    async fn dispatch(
        self: &Arc<Self>,
        request: Request,
        resources: RuntimeResources,
    ) -> Result<Response> {
        let Some(found) = self.router.find(&request)? else {
            let ctx = Context::new(request, Vec::new(), resources);
            return (self.not_found)(ctx).await;
        };
        let ctx = Context::new(request, found.params, resources);
        let mut response = self.run_global(ctx, found.index, 0).await?;
        if found.head_get_fallback {
            response = response.strip_body();
        }
        Ok(response)
    }

    pub(crate) async fn run_global(
        self: &Arc<Self>,
        ctx: Context,
        route_index: usize,
        index: usize,
    ) -> Result<Response> {
        if let Some(middleware) = self.global_middleware.get(index) {
            let next = Next::global(Arc::clone(self), route_index, index + 1);
            middleware(ctx, next).await
        } else {
            self.run_route(ctx, route_index, 0).await
        }
    }

    pub(crate) async fn run_route(
        self: &Arc<Self>,
        ctx: Context,
        route_index: usize,
        middleware_index: usize,
    ) -> Result<Response> {
        let route = &self.router.routes()[route_index];
        if let Some(middleware) = route.middleware.get(middleware_index) {
            let next = Next::route(Arc::clone(self), route_index, middleware_index + 1);
            middleware(ctx, next).await
        } else {
            (route.handler)(ctx).await
        }
    }
}

pub(crate) fn default_not_found() -> Handler {
    crate::handler::handler(|_ctx| async { Ok(Response::text("not found").with_status(404)) })
}
