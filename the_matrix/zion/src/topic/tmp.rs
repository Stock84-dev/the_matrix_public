use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use cache_padded::CachePadded;
use mouse::prelude::*;
use spin::{Mutex, RwLock, RwLockReadGuard};
use tokio::sync::Notify;

pub struct MemReader<T> {
    id: usize,
    notify: Notify,
    events: Arc<MemTopic<T>>,
}

impl<T> MemReader<T> {
    pub async fn read<'a>(&'a self) -> LocalTopicReadGuard<'a, T> {
        self.events.read(self.id, &self.notify).await
    }
}

pub struct MemWriter<T> {
    events: Arc<MemTopic<T>>,
}

impl<T> MemWriter<T> {
    pub fn write(&self, event: T) {
        self.events.write([event]);
    }

    pub fn write_all(&self, items: impl IntoIterator<Item = T>) {
        self.events.write(items);
    }
}

struct ReaderCursors {
    cursors: Vec<CachePadded<AtomicU64>>,
    is_valid: Vec<bool>,
}

struct TopicStore<T> {
    cursor: u64,
    events: Vec<T>,
}

pub(crate) trait RamTopic: Send + Sync {}

pub struct MemTopic<T> {
    notify: Notify,
    write_events: CachePadded<spin::Mutex<TopicStore<T>>>,
    read_events: CachePadded<spin::RwLock<TopicStore<T>>>,
    reader_cursors: spin::RwLock<ReaderCursors>,
}

impl<T> MemTopic<T> {
    async fn read<'a>(&'a self, reader_id: usize, notify: &Notify) -> LocalTopicReadGuard<'a, T> {
        let n_events;
        let reader_cursor;
        {
            let guard = self.reader_cursors.read();
            n_events = self.read_events.read().cursor;
            reader_cursor = guard.cursors[reader_id].load(Ordering::SeqCst);
        }
        debug_assert!(n_events >= reader_cursor);
        if n_events == reader_cursor {
            notify.notified().await;
        }
        LocalTopicReadGuard {
            reader_cursor,
            reader_id,
            topic: self,
            guard: self.read_events.read(),
        }
    }

    fn write(&self, items: impl IntoIterator<Item = T>) {
        // must lock first
        let mut guard = self.write_events.lock();
        let mut prev_len = guard.events.len();
        guard.events.extend(items);
        let added = (guard.events.len() - prev_len) as u64;
        let prev_n_events = guard.cursor;
        guard.cursor += added;
    }

    fn update(&self) {
        let read_cursor = { self.read_events.read().cursor };
        let mut guard = self.reader_cursors.read();
        let target = read_cursor + guard.cursors.len() as u64;
        let should_update = guard
            .cursors
            .iter()
            .zip(guard.is_valid.iter())
            .filter_map(|(cursor, valid)| {
                if *valid {
                    Some(cursor.load(Ordering::SeqCst))
                } else {
                    None
                }
            })
            .all(|x| x == target);
        if should_update {
            let mut read_store = self.read_events.write();
            let mut write_store = self.write_events.lock();
            read_store.cursor = target;
            std::mem::swap(&mut read_store.events, &mut write_store.events);
            self.notify.notify_waiters();
        }
    }

    fn consume(&self, reader_id: usize, n_items: usize) -> u64 {
        let prev = self.reader_cursors.read().cursors[reader_id]
            .fetch_add(n_items as u64, Ordering::SeqCst);
        let cursor = prev + n_items as u64;

        debug_assert!(cursor <= self.read_events.read().cursor);
        cursor
    }
}

pub struct LocalTopicReadGuard<'a, T> {
    reader_id: usize,
    reader_cursor: u64,
    topic: &'a MemTopic<T>,
    guard: RwLockReadGuard<'a, TopicStore<T>>,
}

impl<'a, T> LocalTopicReadGuard<'a, T> {
    pub fn read_all(&mut self) -> &[T] {
        let values = self.peek_all();
        // borrow checker
        unsafe {
            *self.reader_cursor.as_mut_cast() = self.topic.consume(self.reader_id, values.len());
        }
        values
    }

    pub fn read(&mut self) -> Option<&T> {
        let value = self.peek();
        if value.is_some() {
            unsafe {
                *self.reader_cursor.as_mut_cast() = self.topic.consume(self.reader_id, 1);
            }
        }
        value
    }

    pub fn peek_all(&self) -> &[T] {
        debug_assert!(self.guard.cursor > self.reader_cursor);
        &self.guard.events[(self.guard.cursor - self.reader_cursor) as usize..]
    }

    pub fn peek(&self) -> Option<&T> {
        debug_assert!(self.guard.cursor > self.reader_cursor);
        let to_read = self
            .guard
            .events
            .get((self.guard.cursor - self.reader_cursor) as usize);
        to_read
    }

    pub fn consume(&mut self, n_items: usize) {
        self.reader_cursor = self.topic.consume(self.reader_id, n_items);
    }
}
