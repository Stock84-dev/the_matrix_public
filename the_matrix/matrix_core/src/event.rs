use core::slice::{Iter, IterMut};
use std::ops::Index;
use std::sync::Arc;
use std::vec::IntoIter;

use futures::future::try_join_all;
use futures::Future;
use mouse::error::Result;
use tokio::sync::Mutex;

pub struct Listeners<T: ?Sized> {
    listeners: Vec<Box<T>>,
}

impl<'t, T: 't + ?Sized> Listeners<T> {
    pub fn new() -> Listeners<T> {
        Listeners {
            listeners: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Listeners<T> {
        Listeners {
            listeners: Vec::with_capacity(capacity),
        }
    }

    pub fn broadcast(&mut self, mut closure: impl FnMut(&mut T)) {
        for l in self.listeners.iter_mut() {
            closure(l);
        }
    }

    pub fn broadcast_result(
        &mut self,
        mut closure: impl FnMut(&mut T) -> Result<()>,
    ) -> Result<()> {
        for l in self.listeners.iter_mut() {
            closure(l)?;
        }
        Ok(())
    }

    pub async fn broadcast_async<R>(
        &'t mut self,
        coroutine: impl FnMut(&'t mut Box<T>) -> R + 't,
    ) -> Result<()>
    where
        R: Future<Output = Result<()>>,
    {
        try_join_all(self.listeners.iter_mut().map(coroutine)).await?;
        Ok(())
    }

    pub async fn broadcast_async1<A, R>(
        &'t mut self,
        accumulator: Arc<Mutex<A>>,
        mut coroutine: impl FnMut(&'t mut Box<T>, Arc<Mutex<A>>) -> R + 't,
    ) -> Result<()>
    where
        R: Future<Output = Result<()>>,
    {
        let mut coroutines = Vec::with_capacity(self.listeners.len());
        for l in self.listeners.iter_mut() {
            coroutines.push(coroutine(l, accumulator.clone()));
        }

        futures::future::try_join_all(coroutines).await?;

        Ok(())
    }

    pub fn push(&mut self, listener: Box<T>) {
        self.listeners.push(listener);
    }

    pub fn iter(&self) -> Iter<'_, Box<T>> {
        self.listeners.iter()
    }

    pub fn into_iter(self) -> IntoIter<Box<T>> {
        self.listeners.into_iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, Box<T>> {
        self.listeners.iter_mut()
    }
}

macro_rules! broadcast_async {
    ($listeners:ident, $coroutine:expr) => {{
        let mut coroutines = Vec::with_capacity($listeners.len());
        for l in $listeners.iter_mut() {
            coroutines.push(coroutine(l));
        }

        futures::future::try_join_all(coroutines).await?;
    }};
}

impl<T> Index<usize> for Listeners<T> {
    type Output = Box<T>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.listeners[index]
    }
}
