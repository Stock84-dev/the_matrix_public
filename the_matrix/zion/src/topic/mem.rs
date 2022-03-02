use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use bevy::ecs::system::SystemParamFetch;
use bevy::log::trace;
use bevy::prelude::*;
use cache_padded::CachePadded;
use mouse::mem::DenseVec;
use mouse::prelude::*;
use spin::{Mutex, RwLock, RwLockReadGuard};
use tokio::sync::Notify;

use crate::{GlobalEntity, HashSet};

//#[derive(SystemParam)]
pub struct ResTopicWriter<'w, 's, T: Resource> {
    //    #[system_param(ignore)]
    topic: MemTopic<T>,
    //    #[system_param(ignore)]
    _w: PhantomData<&'w ()>,
    //    #[system_param(ignore)]
    _s: PhantomData<&'s ()>,
}

impl<'w, 's, T: Resource> bevy::ecs::system::SystemParam for ResTopicWriter<'w, 's, T> {
    type Fetch = ResTopicWriterState<(), T>;
}
#[doc(hidden)]
pub struct ResTopicWriterState<TSystemParamState, T> {
    state: TSystemParamState,
    marker: std::marker::PhantomData<(T)>,
}
unsafe impl<TSystemParamState: bevy::ecs::system::SystemParamState, T: Resource>
    bevy::ecs::system::SystemParamState for ResTopicWriterState<TSystemParamState, T>
{
    type Config = TSystemParamState::Config;
    fn init(
        world: &mut bevy::ecs::world::World,
        system_meta: &mut bevy::ecs::system::SystemMeta,
        config: Self::Config,
    ) -> Self {
        Self {
            state: TSystemParamState::init(world, system_meta, config),
            marker: std::marker::PhantomData,
        }
    }
    fn new_archetype(
        &mut self,
        archetype: &bevy::ecs::archetype::Archetype,
        system_meta: &mut bevy::ecs::system::SystemMeta,
    ) {
        self.state.new_archetype(archetype, system_meta)
    }
    fn default_config() -> TSystemParamState::Config {
        TSystemParamState::default_config()
    }
    fn apply(&mut self, world: &mut bevy::ecs::world::World) {
        self.state.apply(world)
    }
}
impl<'w, 's, T: Resource> bevy::ecs::system::SystemParamFetch<'w, 's>
    for ResTopicWriterState<(), T>
{
    type Item = ResTopicWriter<'w, 's, T>;
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: &'w bevy::ecs::world::World,
        change_tick: u32,
    ) -> Self::Item {
        let id = world.get_resource::<GlobalEntity>().unwrap().0;
        let topic = world.entity(id).get::<MemTopic<T>>().unwrap().clone();
        ResTopicWriter {
            topic,
            _w: <PhantomData<&'w ()>>::default(),
            _s: <PhantomData<&'s ()>>::default(),
        }
    }
}

impl<'w, 's, T: Resource> ResTopicWriter<'w, 's, T> {
    pub fn write(&self, event: T) {
        self.topic.write(event);
    }

    pub fn write_all(&self, items: impl IntoIterator<Item = T>) {
        self.topic.write_all(items);
    }
}

pub struct RawTopicReader<T: Resource> {
    pub topic: MemTopic<T>,
    reader_id: usize,
}
impl<T: Resource> RawTopicReader<T> {
    pub fn new(topic: &MemTopic<T>) -> Self {
        Self {
            topic: topic.clone(),
            reader_id: topic.new_reader(),
        }
    }
    /// Replaces itself with an invalid reader
    pub fn take(&mut self) -> RawTopicReader<T> {
        Self {
            topic: self.topic.clone(),
            reader_id: std::mem::replace(&mut self.reader_id, usize::MAX),
        }
    }

    pub async fn read<'a>(&'a self) -> LocalTopicReadGuard<'a, T> {
        self.topic.read(self.reader_id).await
    }

    pub fn try_read<'a>(&'a self) -> Option<LocalTopicReadGuard<'a, T>> {
        self.topic.try_read(self.reader_id)
    }
}

impl<'w, 's, T: Resource> From<ResTopicReader<'w, 's, T>> for RawTopicReader<T> {
    fn from(mut reader: ResTopicReader<'w, 's, T>) -> Self {
        reader.raw.take()
    }
}

impl<T: Resource> Drop for RawTopicReader<T> {
    fn drop(&mut self) {
        self.topic.despawn_reader(self.reader_id);
    }
}

//#[derive(SystemParam)]
pub struct ResTopicReader<'w, 's, T: Resource> {
    //    #[system_param(ignore)]
    pub raw: &'s mut RawTopicReader<T>,
    //    #[system_param(ignore)]
    _w: PhantomData<&'w ()>,
    //    #[system_param(ignore)]
    _s: PhantomData<&'s ()>,
}

impl<'w, 's, T: Resource> bevy::ecs::system::SystemParam for ResTopicReader<'w, 's, T> {
    type Fetch = ResTopicReaderState<(), T>;
}
#[doc(hidden)]
pub struct ResTopicReaderState<TSystemParamState, T: Resource> {
    state: TSystemParamState,
    topic: RawTopicReader<T>,
}
unsafe impl<TSystemParamState: bevy::ecs::system::SystemParamState, T: Resource>
    bevy::ecs::system::SystemParamState for ResTopicReaderState<TSystemParamState, T>
{
    type Config = TSystemParamState::Config;
    fn init(
        world: &mut bevy::ecs::world::World,
        system_meta: &mut bevy::ecs::system::SystemMeta,
        config: Self::Config,
    ) -> Self {
        let id = world.get_resource::<GlobalEntity>().unwrap().0;
        let topic = world.entity(id).get::<MemTopic<T>>().unwrap();
        Self {
            topic: RawTopicReader::new(topic),
            state: TSystemParamState::init(world, system_meta, config),
        }
    }
    fn new_archetype(
        &mut self,
        archetype: &bevy::ecs::archetype::Archetype,
        system_meta: &mut bevy::ecs::system::SystemMeta,
    ) {
        self.state.new_archetype(archetype, system_meta)
    }
    fn default_config() -> TSystemParamState::Config {
        TSystemParamState::default_config()
    }
    fn apply(&mut self, world: &mut bevy::ecs::world::World) {
        self.state.apply(world)
    }
}
impl<'w, 's, T: Resource> bevy::ecs::system::SystemParamFetch<'w, 's>
    for ResTopicReaderState<(), T>
{
    type Item = ResTopicReader<'w, 's, T>;
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: &'w bevy::ecs::world::World,
        change_tick: u32,
    ) -> Self::Item {
        ResTopicReader {
            raw: &mut state.topic,
            _w: <PhantomData<&'w ()>>::default(),
            _s: <PhantomData<&'s ()>>::default(),
        }
    }
}

impl<'w, 's, T: Resource> ResTopicReader<'w, 's, T> {
    pub async fn read<'a>(&'a self) -> LocalTopicReadGuard<'a, T> {
        self.raw.topic.read(self.raw.reader_id).await
    }

    pub fn try_read<'a>(&'a self) -> Option<LocalTopicReadGuard<'a, T>> {
        self.raw.topic.try_read(self.raw.reader_id)
    }
}

#[derive(SystemParam)]
pub struct TopicWriter<'w, 's, T: Resource> {
    #[system_param(ignore)]
    topic: Arc<MemTopic<T>>,
    #[system_param(ignore)]
    _w: PhantomData<&'w ()>,
    #[system_param(ignore)]
    _s: PhantomData<&'s ()>,
}

impl<'w, 's, T: Resource> Default for TopicWriter<'w, 's, T> {
    fn default() -> Self {
        panic!(
            "Cannot construct default topic writer, this is used to satisfy trait bounds for \
             local resources"
        )
    }
}

impl<'w, 's, T: Resource> TopicWriter<'w, 's, T> {
    pub fn write(&self, event: T) {
        self.topic.write(event);
    }

    pub fn write_all(&self, items: impl IntoIterator<Item = T>) {
        self.topic.write_all(items);
    }
}

#[derive(SystemParam)]
pub struct TopicReader<'w, 's, T: Resource> {
    #[system_param(ignore)]
    topic: MemTopic<T>,
    #[system_param(ignore)]
    reader_id: usize,
    #[system_param(ignore)]
    _w: PhantomData<&'w ()>,
    #[system_param(ignore)]
    _s: PhantomData<&'s ()>,
}

impl<'w, 's, T: Resource> Default for TopicReader<'w, 's, T> {
    fn default() -> Self {
        panic!(
            "Cannot construct default topic reader, this is used to satisfy trait bounds for \
             local resources"
        )
    }
}

impl<'w, 's, T: Resource> TopicReader<'w, 's, T> {
    pub async fn read<'a>(&'a self) -> LocalTopicReadGuard<'a, T> {
        self.topic.read(self.reader_id).await
    }

    pub fn try_read<'a>(&'a self) -> Option<LocalTopicReadGuard<'a, T>> {
        self.topic.try_read(self.reader_id)
    }

    pub fn id(&self) -> usize {
        self.reader_id
    }
}
// pub struct MemReader<T> {
//    id: usize,
//    events: Arc<MemTopic<T>>,
//}
// impl<T> MemReader<T> {
//    pub async fn read<'a>(&'a self) -> LocalTopicReadGuard<'a, T> {
//        self.events.read(self.id).await
//    }
//}
// pub struct MemWriter<T> {
//    events: Arc<MemTopic<T>>,
//}
// impl<T> MemWriter<T> {
//    pub fn write(&self, event: T) {
//        self.events.write([event]);
//    }
//
//    pub fn write_all(&self, items: impl IntoIterator<Item = T>) {
//        self.events.write(items);
//    }
//}

struct TopicStore<T> {
    events: Vec<T>,
}

impl<T> Default for TopicStore<T> {
    fn default() -> Self {
        Self { events: vec![] }
    }
}

pub struct MemTopicInner<T: Resource> {
    /// cumulative number of all events processed + number of events in read buffer
    n_total_readable_events: AtomicU64,
    notify: Notify,
    write_events: CachePadded<spin::Mutex<TopicStore<T>>>,
    read_events: CachePadded<spin::RwLock<TopicStore<T>>>,
    pub reader_cursors: spin::RwLock<Vec<CachePadded<Option<AtomicU64>>>>,
    n_writers: AtomicUsize,
}

#[derive(Component)]
pub struct MemTopic<T: Resource>(pub Arc<MemTopicInner<T>>);

impl<T: Resource> Default for MemTopic<T> {
    fn default() -> Self {
        Self(Arc::new(MemTopicInner {
            n_total_readable_events: Default::default(),
            notify: Default::default(),
            write_events: Default::default(),
            read_events: Default::default(),
            reader_cursors: Default::default(),
            n_writers: Default::default(),
        }))
    }
}

impl<T: Resource> MemTopic<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_reader(&self) -> usize {
        let mut cursors = self.0.reader_cursors.write();
        let start = AtomicU64::new(self.0.n_total_readable_events.load(Ordering::SeqCst));
        match cursors.iter().position(|x| x.is_none()) {
            None => {
                let i = cursors.len();
                cursors.push(CachePadded::new(Some(start)));
                i
            }
            Some(i) => {
                cursors[i] = CachePadded::new(Some(start));
                i
            }
        }
    }

    pub async fn read<'a>(&'a self, reader_id: usize) -> LocalTopicReadGuard<'a, T> {
        let reader_cursor;
        {
            let guard = self.0.reader_cursors.read();
            reader_cursor = guard
                .get(reader_id)
                .unwrap()
                .as_ref()
                .unwrap()
                .load(Ordering::SeqCst);
        }
        if reader_cursor == self.0.n_total_readable_events.load(Ordering::SeqCst) {
            // wait for new events
            self.0.notify.notified().await;
        }
        LocalTopicReadGuard {
            reader_cursor,
            reader_id,
            topic: self,
            guard: self.0.read_events.read(),
        }
    }

    pub fn try_read<'a>(&'a self, reader_id: usize) -> Option<LocalTopicReadGuard<'a, T>> {
        let reader_cursor;
        {
            let guard = self.0.reader_cursors.read();
            reader_cursor = guard
                .get(reader_id)
                .unwrap()
                .as_ref()
                .unwrap()
                .load(Ordering::SeqCst);
        }
        if reader_cursor == self.0.n_total_readable_events.load(Ordering::SeqCst) {
            return None;
        }
        Some(LocalTopicReadGuard {
            reader_cursor,
            reader_id,
            topic: self,
            guard: self.0.read_events.read(),
        })
    }

    pub fn write(&self, item: T) {
        self.write_all([item]);
    }

    pub fn write_all(&self, items: impl IntoIterator<Item = T>) {
        trace!("{}::write()", std::any::type_name::<Self>());
        // must lock first
        let mut guard = self.0.write_events.lock();
        guard.events.extend(items);
    }

    pub fn update(&self) {
        let mut write_store = self.0.write_events.lock();
        if write_store.events.is_empty() {
            return;
        }
        let mut guard = self.0.reader_cursors.read();
        let n_events = self.0.n_total_readable_events.load(Ordering::SeqCst);
        if !guard
            .iter()
            .filter_map(|x| x.as_ref())
            .all(|x| x.load(Ordering::SeqCst) == n_events)
        {
            return;
        }
        trace!("{}::update()", std::any::type_name::<Self>());
        let mut read_store = self.0.read_events.write();
        std::mem::swap(&mut read_store.events, &mut write_store.events);
        self.0
            .n_total_readable_events
            .fetch_add(read_store.events.len() as u64, Ordering::SeqCst);
        self.0.notify.notify_waiters();
    }

    pub fn consume(&self, reader_id: usize, n_items: usize) -> u64 {
        let prev = self
            .0
            .reader_cursors
            .read()
            .get(reader_id)
            .unwrap()
            .as_ref()
            .unwrap()
            .fetch_add(n_items as u64, Ordering::SeqCst);
        let cursor = prev + n_items as u64;
        cursor
    }

    pub fn despawn_reader(&self, reader_id: usize) {
        let mut guard = self.0.reader_cursors.write();
        if reader_id < guard.len() {
            guard[reader_id] = CachePadded::new(None);
        }
    }

    pub fn update_system(query: Query<&MemTopic<T>>) {
        for q in query.iter() {
            q.update();
        }
    }
}

impl<T: Resource> Clone for MemTopic<T> {
    /// Increases arc
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct LocalTopicReadGuard<'a, T: Resource> {
    reader_id: usize,
    reader_cursor: u64,
    topic: &'a MemTopic<T>,
    guard: RwLockReadGuard<'a, TopicStore<T>>,
}

impl<'a, T: Resource> LocalTopicReadGuard<'a, T> {
    pub fn read_all(&self) -> &[T] {
        //        debug!("{:#?}", &self.topic.0 as *const _);
        //        debug!("{:#?}", self.topic.0.reader_cursors);
        //        debug!("{:#?}", self.reader_cursor);
        trace!("{}::read_all()", std::any::type_name::<Self>());
        let values = self.peek_all();
        // borrow checker
        unsafe {
            *self.reader_cursor.as_mut_cast() = self.topic.consume(self.reader_id, values.len());
        }
        //        debug!("{:#?}", self.topic.0.reader_cursors);
        //        debug!("{:#?}", self.reader_cursor);
        values
    }

    pub fn read(&self) -> &T {
        trace!("{}::read()", std::any::type_name::<Self>());
        let value = self.peek().unwrap();
        unsafe {
            *self.reader_cursor.as_mut_cast() = self.topic.consume(self.reader_id, 1);
        }
        value
    }

    pub fn peek_all(&self) -> &[T] {
        trace!("{}::peek_all()", std::any::type_name::<Self>());
        let n_events = self.topic.0.n_total_readable_events.load(Ordering::SeqCst);
        let len = self.guard.events.len();
        &self.guard.events[len - (n_events - self.reader_cursor) as usize..]
    }

    pub fn peek(&self) -> Option<&T> {
        trace!("{}::peek()", std::any::type_name::<Self>());
        let n_events = self.topic.0.n_total_readable_events.load(Ordering::SeqCst);
        let len = self.guard.events.len();
        let to_read = self
            .guard
            .events
            .get(len - (n_events - self.reader_cursor) as usize);
        to_read
    }

    pub fn consume(&mut self, n_items: usize) {
        trace!("{}::consume()", std::any::type_name::<Self>());
        self.reader_cursor = self.topic.consume(self.reader_id, n_items);
    }
}
