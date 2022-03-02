
use std::sync::{Condvar, Mutex};

pub struct Condpair<T> {
    pub var: Condvar,
    pub value: Mutex<T>,
}

impl<T> Condpair<T> {
    pub fn new(value: T) -> Self {
        Self {
            var: Default::default(),
            value: Mutex::new(value),
        }
    }
}

pub struct UnsafeSyncRefCell<T> {
    inner: T,
}

impl<T> UnsafeSyncRefCell<T> {
    pub unsafe fn new(data: T) -> Self {
        Self { inner: data }
    }

    pub fn borrow_mut(&self) -> &mut T {
        unsafe { &mut *(&self.inner as *const _ as *mut T) }
    }

    pub fn borrow(&self) -> &T {
        &self.inner
    }
}

unsafe impl<T> Sync for UnsafeSyncRefCell<T> {}
