use std::future::Future;
use std::sync::{Arc, Mutex};

use crate::handler::BoxFuture;
use crate::prepared::PreparedInner;
use crate::{Context, Error, Response, Result};

pub(crate) type Middleware =
    Arc<dyn Fn(Context, Next) -> BoxFuture<Result<Response>> + Send + Sync>;

pub(crate) fn middleware<F, Fut>(middleware: F) -> Middleware
where
    F: Fn(Context, Next) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Response>> + Send + 'static,
{
    Arc::new(move |ctx, next| Box::pin(middleware(ctx, next)))
}

pub struct Next {
    inner: Arc<PreparedInner>,
    route_index: usize,
    phase: NextPhase,
    middleware_index: usize,
    used: Mutex<bool>,
}

pub(crate) enum NextPhase {
    Global,
    Route,
}

impl Next {
    pub(crate) fn global(
        inner: Arc<PreparedInner>,
        route_index: usize,
        middleware_index: usize,
    ) -> Self {
        Self {
            inner,
            route_index,
            phase: NextPhase::Global,
            middleware_index,
            used: Mutex::new(false),
        }
    }

    pub(crate) fn route(
        inner: Arc<PreparedInner>,
        route_index: usize,
        middleware_index: usize,
    ) -> Self {
        Self {
            inner,
            route_index,
            phase: NextPhase::Route,
            middleware_index,
            used: Mutex::new(false),
        }
    }

    pub async fn run(&self, ctx: Context) -> Result<Response> {
        {
            let mut used = self
                .used
                .lock()
                .map_err(|_| Error::middleware("next state lock poisoned"))?;
            if *used {
                return Err(Error::middleware("Next can only be run once"));
            }
            *used = true;
        }
        match self.phase {
            NextPhase::Global => {
                self.inner
                    .run_global(ctx, self.route_index, self.middleware_index)
                    .await
            }
            NextPhase::Route => {
                self.inner
                    .run_route(ctx, self.route_index, self.middleware_index)
                    .await
            }
        }
    }
}
