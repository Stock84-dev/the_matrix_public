use std::fmt::{Debug, Display, Formatter};

pub use anyhow::{anyhow, bail, ensure, Chain, Error, Result};
use chrono::{DateTime, Local};
use failure::{Fail, ResultExt};

use crate::sync::EventQueue;

lazy_static! {
    pub static ref ERRORS: EventQueue<UserError> = EventQueue::unbounded();
}

/// Provides 'compat' method for easy conversion between failure and anyhow types.
pub trait FailureExt<R> {
    fn compat(self) -> anyhow::Result<R>;
}

impl<R, E: Into<failure::Error>> FailureExt<R> for std::result::Result<R, E> {
    fn compat(self) -> Result<R, anyhow::Error> {
        match self {
            Ok(r) => Ok(r),
            Err(e) => {
                let e = e.into();
                let err = anyhow!("{:?}", e);
                Err(err)
            }
        }
    }
}
#[derive(Debug)]
pub struct FailureError {
    inner: failure::Error,
}

impl std::fmt::Display for FailureError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

impl std::error::Error for FailureError {}

pub trait BoxErrorExt<R> {
    fn sized(self) -> anyhow::Result<R>;
}

impl<R> BoxErrorExt<R> for std::result::Result<R, Box<dyn std::error::Error + Send + Sync>> {
    fn sized(self) -> Result<R, anyhow::Error> {
        self.map_err(|x| anyhow!("{:?}", x))
    }
}

/// Return an Err(e) from function. Expands to:
/// ```ignore
/// return Err(crate::anyhow::anyhow!(""));
/// ```
#[macro_export]
macro_rules! throw {
    ($($args:tt)*) => {
        return Err(mouse::error::anyhow!($($args)*));
    }
}

pub struct UserError {
    pub error: anyhow::Error,
    pub time: DateTime<Local>,
}

impl UserError {
    pub fn new(e: anyhow::Error) -> Self {
        UserError {
            error: e,
            time: Local::now(),
        }
    }
}
#[macro_export]
macro_rules! loop_error_context {
    ($result:expr, $context: expr) => {{
        let result = $crate::error::ErrCtxExt::context($result, $context);
        match result {
            Ok(value) => value,
            Err(e) => {
                log_error!(e);
                continue;
            }
        }
    }};
}

#[macro_export]
macro_rules! log_error {
    ($err:expr) => {{
        let e = $crate::error::Error::from($err);
        $crate::log::error!("{:?}", e);
        $crate::error::ERRORS.push_blocking($crate::error::UserError::new(e));
    }};
}

pub trait ResultCtxExt<T, E> {
    fn log(self) -> Option<T>;
    fn log_context<C>(self, context: C) -> Option<T>
    where
        C: Display + Send + Sync + 'static;
    fn log_with_context<C, F>(self, f: F) -> Option<T>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C;

    /// Wrap the error value with additional context.
    fn context<C>(self, context: C) -> Result<T, anyhow::Error>
    where
        Self: Sized,
        C: Display + Send + Sync + 'static;

    /// Wrap the error value with additional context that is evaluated lazily
    /// only once an error does occur.
    fn with_context<C, F>(self, f: F) -> Result<T, anyhow::Error>
    where
        Self: Sized,
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C;

    /// Ignore this result.
    fn ignore(self);
}

impl<U: anyhow::Context<T, E>, T, E> ResultCtxExt<T, E> for U {
    fn log(self) -> Option<T> {
        match self.context("") {
            Ok(x) => Some(x),
            Err(e) => {
                log_error(e);
                None
            }
        }
    }

    fn log_context<C>(self, context: C) -> Option<T>
    where
        C: Display + Send + Sync + 'static,
    {
        match self.context(context) {
            Ok(x) => Some(x),
            Err(e) => {
                log_error(e);
                None
            }
        }
    }

    fn log_with_context<C, F>(self, f: F) -> Option<T>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        match self.with_context(f) {
            Ok(x) => Some(x),
            Err(e) => {
                log_error(e);
                None
            }
        }
    }

    /// Wrap the error value with additional context.
    fn context<C>(self, context: C) -> Result<T, anyhow::Error>
    where
        Self: Sized,
        C: Display + Send + Sync + 'static,
    {
        anyhow::Context::context(self, context)
    }

    /// Wrap the error value with additional context that is evaluated lazily
    /// only once an error does occur.
    fn with_context<C, F>(self, f: F) -> Result<T, anyhow::Error>
    where
        Self: Sized,
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        anyhow::Context::with_context(self, f)
    }

    fn ignore(self) {
        // we do nothing with result by dropping it
    }
}

pub trait ErrCtxExt<E> {
    fn log(self);
    fn log_context<C>(self, context: C)
    where
        C: Display + Send + Sync + 'static;
    fn log_with_context<C, F>(self, f: F)
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C;

    /// Wrap the error value with additional context.
    fn context<C>(self, context: C) -> Result<(), anyhow::Error>
    where
        Self: Sized,
        C: Display + Send + Sync + 'static;

    /// Wrap the error value with additional context that is evaluated lazily
    /// only once an error does occur.
    fn with_context<C, F>(self, f: F) -> Result<(), anyhow::Error>
    where
        Self: Sized,
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<E: std::error::Error> ErrCtxExt<E> for E
where
    Result<(), E>: anyhow::Context<(), E>,
{
    fn log(self) {
        Err::<(), E>(self).log();
    }

    fn log_context<C>(self, context: C)
    where
        C: Display + Send + Sync + 'static,
    {
        Err::<(), E>(self).log_context(context);
    }

    fn log_with_context<C, F>(self, f: F)
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        Err::<(), E>(self).log_with_context(f);
    }

    fn context<C>(self, context: C) -> Result<(), Error>
    where
        Self: Sized,
        C: Display + Send + Sync + 'static,
    {
        Err::<(), E>(self).context(context)
    }

    fn with_context<C, F>(self, f: F) -> Result<(), Error>
    where
        Self: Sized,
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        Err::<(), E>(self).with_context(f)
    }
}

pub fn log_error(e: anyhow::Error) {
    crate::log::error!("{:?}", e);
    ERRORS.push_blocking(UserError::new(e));
}

pub struct UnsafeSendSyncError<T: std::error::Error> {
    inner: T,
}

impl<T: std::error::Error> UnsafeSendSyncError<T> {
    pub unsafe fn new(inner: T) -> Self {
        Self { inner }
    }
}

unsafe impl<T: std::error::Error> Send for UnsafeSendSyncError<T> {}
unsafe impl<T: std::error::Error> Sync for UnsafeSendSyncError<T> {}

impl<T: std::error::Error> Debug for UnsafeSendSyncError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.inner, f)
    }
}

impl<T: std::error::Error> Display for UnsafeSendSyncError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

impl<T: std::error::Error> std::error::Error for UnsafeSendSyncError<T> {}

pub trait ErrorExt {
    type Item;
    fn any_err(self) -> Result<Self::Item, anyhow::Error>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> ErrorExt for Result<T, E> {
    type Item = T;

    fn any_err(self) -> Result<Self::Item, anyhow::Error> {
        self.map_err(|e| e.into())
    }
}
