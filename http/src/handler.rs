use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::{Context, Response, Result};

pub(crate) type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub(crate) type Handler = Arc<dyn Fn(Context) -> BoxFuture<Result<Response>> + Send + Sync>;

pub(crate) fn handler<F, Fut>(handler: F) -> Handler
where
    F: Fn(Context) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Response>> + Send + 'static,
{
    Arc::new(move |ctx| Box::pin(handler(ctx)))
}
