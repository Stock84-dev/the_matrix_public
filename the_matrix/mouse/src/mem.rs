use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use itertools::Itertools;

use crate::helpers::some_if;

#[derive(Default)]
pub struct Arena<T: Reset> {
    items: Arc<Mutex<Vec<Arc<T>>>>,
}

impl<T: Default + Reset> Arena<T> {
    pub fn alloc(&self) -> Mut<T> {
        self.alloc_with(|| T::default())
    }
}

impl<T: Reset> Arena<T> {
    pub fn alloc_with(&self, init: impl FnOnce() -> T) -> Mut<T> {
        let mut guard = self.items.lock().unwrap();
        let i = match guard.iter().find_position(|x| Arc::strong_count(x) == 1) {
            None => {
                guard.push(Arc::new(init()));
                guard.len() - 1
            }
            Some((i, _)) => {
                Arc::get_mut(&mut guard[i])
                    .expect("Arena item reference count is not at 1")
                    .reset();
                i
            }
        };
        Mut {
            inner: guard[i].clone(),
        }
    }
}

impl<T: Reset> Arena<T> {
    pub fn clear(&self) {
        let mut guard = self.items.lock().unwrap();
        guard.clear();
    }

    pub fn garbage_collect(&self) {
        let mut guard = self.items.lock().unwrap();
        guard.retain(|x| Arc::strong_count(x) > 1);
    }
}

impl<T: Reset> Clone for Arena<T> {
    /// Increases reference count
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
        }
    }
}

pub trait Reset {
    fn reset(&mut self);
}

impl<T> Reset for Vec<T> {
    fn reset(&mut self) {
        self.clear();
    }
}

#[derive(Debug)]
pub struct Const<T> {
    inner: Arc<T>,
}

impl<T> Clone for Const<T> {
    /// Increases reference counter
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Deref for Const<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
pub struct Mut<T> {
    inner: Arc<T>,
}

impl<T> Into<Const<T>> for Mut<T> {
    fn into(self) -> Const<T> {
        Const { inner: self.inner }
    }
}

impl<T> Deref for Mut<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Mut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: write reference gets created if strong reference count is 1
        // weak references cannot be created
        unsafe { &mut *(&*self.inner as *const _ as *mut _) }
    }
}

pub struct DenseVec<T> {
    items: Vec<T>,
    indices: Vec<Option<usize>>,
}

impl<T> DenseVec<T> {
    pub fn new() -> Self {
        Self {
            items: vec![],
            indices: vec![],
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            indices: Vec::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, item: T) -> usize {
        let id = self.items.len();
        self.items.push(item);
        match self.indices.iter_mut().find_position(|x| x.is_none()) {
            None => {
                let index = self.indices.len();
                self.indices.push(Some(id));
                index
            }
            Some((i, index)) => {
                *index = Some(id);
                i
            }
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        let index = *self.indices.get(index)?.as_ref()?;
        self.items.get(index)
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        let index = self.indices.get_mut(index)?;
        let i = (*index)?;
        *index = None;
        let item = self.items.swap_remove(i);
        let last_i = self.items.len();
        self.indices.iter_mut().find_map(|x| {
            let index = x.as_mut()?;
            some_if(*index == last_i, || *index = i)
        });
        Some(item)
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T> {
        self.items.iter()
    }

    pub fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T> {
        self.items.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<T> IntoIterator for DenseVec<T> {
    type Item = <Vec<T> as IntoIterator>::Item;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a DenseVec<T> {
    type Item = <&'a Vec<T> as IntoIterator>::Item;
    type IntoIter = <&'a Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.items).into_iter()
    }
}

impl<'a, T> IntoIterator for &'a mut DenseVec<T> {
    type Item = <&'a mut Vec<T> as IntoIterator>::Item;
    type IntoIter = <&'a mut Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.items).into_iter()
    }
}

impl<T: Debug> Debug for DenseVec<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.items.fmt(f)
    }
}

impl<T> Default for DenseVec<T> {
    fn default() -> Self {
        DenseVec::new()
    }
}
