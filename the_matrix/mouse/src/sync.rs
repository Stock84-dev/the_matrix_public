use std::collections::VecDeque;

pub use parking_lot::*;
pub use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::Notify;

use crate::ext::ExecuteFutureExt;

pub mod priority {
    pub use priomutex::{Mutex, MutexGuard};
}

pub struct EventQueue<T> {
    queue: Mutex<VecDeque<T>>,
    notify: Notify,
    bound: usize,
}

impl<T> EventQueue<T> {
    pub fn unbounded() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            notify: Default::default(),
            bound: usize::MAX,
        }
    }

    pub fn bounded(capacity: usize) -> Self {
        Self {
            queue: Mutex::new(VecDeque::with_capacity(capacity)),
            notify: Default::default(),
            bound: capacity,
        }
    }

    pub async fn push(&self, item: T) {
        loop {
            {
                let q = self.queue.lock();
                if q.len() < self.bound {
                    break;
                }
            }
            self.notify.notified().await;
        }
        self.queue.lock().push_back(item);
        self.notify.notify_waiters();
    }

    pub fn push_blocking(&self, item: T) {
        loop {
            {
                let q = self.queue.lock();
                if q.len() < self.bound {
                    break;
                }
            }
            self.notify.notified().block();
        }
        self.queue.lock().push_back(item);
        self.notify.notify_waiters();
    }

    pub async fn pop(&self) -> T {
        loop {
            if let Some(value) = self.queue.lock().pop_front() {
                self.notify.notify_waiters();
                return value;
            }
            self.notify.notified().await;
        }
    }

    pub async fn pop_blocking(&self) -> T {
        loop {
            if let Some(value) = self.queue.lock().pop_front() {
                self.notify.notify_waiters();
                return value;
            }
            self.notify.notified().block();
        }
    }

    pub fn clear(&self) {
        self.queue.lock().clear();
    }

    pub fn capacity(&self) -> usize {
        self.queue.lock().capacity()
    }
}
