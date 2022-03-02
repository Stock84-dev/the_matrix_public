use std::fmt::Display;
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;

use futures_lite::future;
use pin_project::pin_project;
use tokio::task::JoinHandle;

use crate::prelude::*;

#[pin_project]
#[derive(Component)]
pub struct Task<T>(#[pin] JoinHandle<T>);

impl<T> Task<T> {
    pub fn new(handle: JoinHandle<T>) -> Self {
        Self(handle)
    }

    pub fn poll(&mut self) -> Option<T> {
        future::block_on(future::poll_once(&mut self.0)).map(|x| x.unwrap())
    }

    pub fn cancel(&self) {
        self.0.abort()
    }
}

pub trait BevyFutExt: Future {
    /// Updates event loop once future resolves
    fn spawn_update(self) -> Task<Self::Output>
    where
        Self: Send + 'static,
        Self::Output: Send;
}

pub trait BevyTryFut<T>: Future {
    fn spawn_handled(self) -> Task<()>;
    fn spawn_handled_with_context<C, U>(self, f: U) -> Task<()>
    where
        C: Display + Send + Sync + 'static,
        U: Future<Output = C> + Send + 'static;
}

impl<F: Future<Output = Result<T, E>>, T, E> BevyTryFut<T> for F
where
    Self: Send + 'static,
    Self::Output: mouse::error::ResultCtxExt<T, E> + Send,
{
    fn spawn_handled(self) -> Task<()> {
        self.spawn_handled_with_context(async { "future failed to execute" })
    }

    fn spawn_handled_with_context<C, U>(self, f: U) -> Task<()>
    where
        C: Display + Send + Sync + 'static,
        U: Future<Output = C> + Send + 'static,
    {
        Task::new(mouse::ext::ExecuteFutureExt::spawn(async move {
            let result = self.await;
            EVENT_LOOP.update();
            if result.is_err() {
                let context = f.await;
                result.log_context(context);
            }
        }))
    }
}

impl<T: Future> BevyFutExt for T
where
    Self: Sized,
{
    fn spawn_update(self) -> Task<Self::Output>
    where
        Self: Send + 'static,
        Self::Output: Send,
    {
        Task::new(mouse::ext::ExecuteFutureExt::spawn(async move {
            let result = self.await;
            EVENT_LOOP.update();
            result
        }))
    }
}

/// Atomic reference counter that implements Component
#[derive(Component)]
pub struct Carc<T>(Arc<T>);

impl<T> Carc<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new(data))
    }
}

impl<T> Deref for Carc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Clone for Carc<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
