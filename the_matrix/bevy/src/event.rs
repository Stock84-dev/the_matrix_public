use std::collections::VecDeque;
use std::hash::Hash;
use std::iter::Filter;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

use bevy_utils::StableHashSet;
use mouse::rayon;
use tokio::sync::futures::Notified;
use tokio::sync::Notify;

use crate::prelude::*;
pub struct PipelinePlugin;

impl Plug for PipelinePlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        app.init_resource::<FlushingPipelinesResource>()
            .add_event::<FlushPipeline>()
            .add_system_to_stage(CoreStage::First, flush_pipelines)
    }
}
pub struct PipelinedEvents<T: Resource> {
    reader_id: usize,
    events: [Vec<Pipelined<T>>; 2],
}
impl<T: Resource> Default for PipelinedEvents<T> {
    fn default() -> Self {
        Self {
            reader_id: 0,
            events: [Vec::new(), Vec::new()],
        }
    }
}
impl<T: Resource> PipelinedEvents<T> {
    pub fn update_system(
        mut events: ResMut<PipelinedEvents<T>>,
        mut reader: EventReader<FlushPipeline>,
    ) {
        let writer_id = events.reader_id;
        events.events[writer_id].clear();
        events.reader_id ^= 1;
        let reader_id = events.reader_id;
        for e in reader.iter() {
            events.events[reader_id].retain(|x| {
                if x.id != e.id {
                    warn!("flushed {}", std::any::type_name::<PipelinedEvents<T>>());
                    true
                } else {
                    false
                }
            });
        }
    }
}

#[derive(Deref)]
pub struct Pipelined<T> {
    pub id: Entity,
    #[deref]
    inner: T,
}

// TODO: bring back clear system
#[derive(SystemParam)]
pub struct PipelinedReader<'w, 's, T: Resource> {
    events: Res<'w, PipelinedEvents<T>>,
    #[system_param(ignore)]
    marker: PhantomData<&'s usize>,
}
impl<'w, 's, T: Resource> PipelinedReader<'w, 's, T> {
    pub fn iter<'a>(&'a self) -> std::slice::Iter<'a, Pipelined<T>> {
        self.events.events[self.events.reader_id].iter()
    }

    pub fn par_iter<'a>(&'a self) -> rayon::slice::Iter<'a, Pipelined<T>> {
        self.events.events[self.events.reader_id].par_iter()
    }
}
pub struct PipelinedFilter<'w, 's, 'a, T> {
    query: &'a Query<'w, 's, &'static Pipeline>,
    _t: PhantomData<T>,
}
impl<'w, 's, 'a, T: Resource> FnOnce<(&&Pipelined<T>,)> for PipelinedFilter<'w, 's, 'a, T> {
    type Output = bool;

    extern "rust-call" fn call_once(self, args: (&&Pipelined<T>,)) -> Self::Output {
        self.call(args)
    }
}
impl<'w, 's, 'a, T: Resource> FnMut<(&&Pipelined<T>,)> for PipelinedFilter<'w, 's, 'a, T> {
    extern "rust-call" fn call_mut(&mut self, args: (&&Pipelined<T>,)) -> Self::Output {
        self.call(args)
    }
}
impl<'w, 's, 'a, T: Resource> Fn<(&&Pipelined<T>,)> for PipelinedFilter<'w, 's, 'a, T> {
    extern "rust-call" fn call(&self, args: (&&Pipelined<T>,)) -> Self::Output {
        let pipeline = self
            .query
            .get(args.0.id)
            .expect("pipeline component not found for event");
        if pipeline.is_flushing {
            warn!("portal filtered");
        }
        !pipeline.is_flushing
    }
}

#[derive(SystemParam)]
pub struct PipelinedWriter<'w, 's, T: Resource> {
    events: Res<'w, PipelinedEvents<T>>,
    #[system_param(ignore)]
    marker: PhantomData<&'s usize>,
}
impl<'w, 's, T: Resource> PipelinedWriter<'w, 's, T> {
    pub fn send(&self, event: T, entity: Entity) {
        let writer_id = self.events.reader_id ^ 1;
        unsafe {
            panic!("BUG: race condition with multiple writers");
            self.events.events[writer_id].as_mut_cast().push(Pipelined {
                id: entity,
                inner: event,
            });
        }
    }

    pub fn send_batch(&self, events: impl Iterator<Item = Pipelined<T>>) {
        let writer_id = self.events.reader_id ^ 1;
        unsafe {
            self.events.events[writer_id].as_mut_cast().extend(events);
        }
    }
}
impl<T> Pipelined<T> {
    pub fn new(entity: Entity, inner: T) -> Self {
        Self { id: entity, inner }
    }

    pub fn to_flushable<U>(&self, data: U) -> Pipelined<U> {
        Pipelined {
            id: self.id,
            inner: data,
        }
    }
}

#[derive(Default)]
struct FlushingPipelinesResource {
    flush: Vec<Entity>,
    flushing: Vec<Entity>,
    flushed: Vec<Entity>,
}
pub struct FlushPipeline {
    id: Entity,
}
impl FlushPipeline {
    pub fn new(id: Entity) -> Self {
        Self { id }
    }
    pub fn id(&self) -> Entity {
        self.id
    }
}

#[derive(SystemParam)]
pub struct PipelineFlusher<'w, 's> {
    inner: EventWriter<'w, 's, FlushPipeline>,
}
impl<'w, 's> PipelineFlusher<'w, 's> {
    pub fn flush(&mut self, pipeline: &mut Pipeline, entity: Entity) {
        // if on previous update pipeline has started flushing we extend that by sending it again
        // needs to happen for systems that read FlushPipeline event to reset their state again
        //        if pipeline.is_flushing {
        //            return;
        //        }
        pipeline.is_flushing = true;
        self.inner.send(FlushPipeline { id: entity });
    }
}
fn flush_pipelines(
    mut flushing_pipelines: ResMut<FlushingPipelinesResource>,
    mut reader: EventReader<FlushPipeline>,
    async_pipelines: Query<&AsyncPipeline>,
    mut pipelines: Query<&mut Pipeline>,
) {
    for id in flushing_pipelines.flushed.drain(..) {
        if let Ok(mut pipeline) = pipelines.get_mut(id) {
            pipeline.is_flushing = false;
        }
        if let Ok(pipeline) = async_pipelines.get(id) {
            pipeline.is_flushing.send(false).ignore();
        }
        println!("flushed");
    }
    let tmp = &mut *flushing_pipelines;
    std::mem::swap(&mut tmp.flushing, &mut tmp.flushed);
    std::mem::swap(&mut tmp.flush, &mut tmp.flushing);
    for e in reader.iter() {
        if let Ok(pipeline) = async_pipelines.get(e.id) {
            pipeline.is_flushing.send(true).ignore();
            flushing_pipelines.flush.push(e.id);
        }
        if let Ok(mut pipeline) = pipelines.get_mut(e.id) {
            pipeline.is_flushing = true;
            flushing_pipelines.flushing.push(e.id);
        }
        println!("flushing");
    }
    if !flushing_pipelines.flushed.is_empty() {
        EVENT_LOOP.update();
    }
}
pub struct AsyncPipelineWatch(tokio::sync::watch::Receiver<bool>);

impl AsyncPipelineWatch {
    pub async fn on_flushing(&mut self) -> Result<(), tokio::sync::watch::error::RecvError> {
        self.on(true).await
    }

    pub async fn on_flushed(&mut self) -> Result<(), tokio::sync::watch::error::RecvError> {
        self.on(false).await
    }

    async fn on(&mut self, value: bool) -> Result<(), tokio::sync::watch::error::RecvError> {
        loop {
            match self.0.changed().await {
                Ok(_) => {
                    if *self.0.borrow() == value {
                        return Ok(());
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[derive(Component)]
pub struct AsyncPipeline {
    is_flushing: tokio::sync::watch::Sender<bool>,
}
impl AsyncPipeline {
    pub fn new() -> (Self, AsyncPipelineWatch) {
        let (tx, rx) = tokio::sync::watch::channel(false);
        (Self { is_flushing: tx }, AsyncPipelineWatch(rx))
    }

    pub fn spawn(id: Entity, commands: &mut Commands) -> AsyncPipelineWatch {
        let (tx, rx) = AsyncPipeline::new();
        commands
            .entity(id)
            .insert(Pipeline { is_flushing: false })
            .insert(tx);
        rx
    }
}

#[derive(Component)]
pub struct Pipeline {
    is_flushing: bool,
}
impl Pipeline {
    pub fn is_flushing(&self) -> bool {
        self.is_flushing
    }
}
impl Clone for Pipeline {
    /// Increases reference count
    fn clone(&self) -> Self {
        Self {
            is_flushing: self.is_flushing.clone(),
        }
    }
}

// Contains Events that are unique, unlike normal events readers can see new events after update
// is called
pub struct UniqueEvents<T> {
    reader_id: usize,
    event_counts: [usize; 2],
    events: [StableHashSet<T>; 2],
}
impl<T> Default for UniqueEvents<T> {
    fn default() -> Self {
        Self {
            reader_id: 0,
            event_counts: [0, 0],
            events: [Default::default(), Default::default()],
        }
    }
}
impl<T: Resource + Hash + Eq> UniqueEvents<T> {
    pub fn update(&mut self) {
        trace!("update {}", std::any::type_name::<Self>());
        self.reader_id ^= 1;
        self.events[self.reader_id ^ 1].clear();
        self.event_counts[self.reader_id ^ 1] = self.event_counts[self.reader_id];
    }

    pub fn send(&mut self, event: T) {
        trace!("send {}", std::any::type_name::<Self>());
        let id = self.reader_id ^ 1;
        if self.events[id].insert(event) {
            self.event_counts[id] += 1;
        }
    }

    pub fn send_batch(&mut self, events: impl Iterator<Item = T>) {
        let id = self.reader_id ^ 1;
        let mut count = 0;
        self.events[id].extend(events.map(|x| {
            count += 1;
            x
        }));
        self.event_counts[id] += count;
    }

    pub fn update_system(mut events: ResMut<Self>) {
        events.update();
    }

    /// User must read all events, otherwise it would be missed
    pub fn iter<'a>(&'a self) -> std::collections::hash_set::Iter<'a, T> {
        trace!("read {}", std::any::type_name::<Self>());
        // NOTE: if event counts overflow this would be a bug
        // if we are running for 100 years, we could sustain 5849 million updates per second
        // if we use 8 wide SIMD with 256 cores we could sustain 2.856 million updates per second
        let reader_id = self.reader_id;
        //        let to_read = self.event_counts[reader_id] - *reader_event_count;
        //        let offset = self.events[reader_id].len() - to_read;
        //        *reader_event_count += to_read;
        //        self.events[reader_id].iter().skip(offset)
        self.events[reader_id].iter()
    }

    /// User must read all events, otherwise it would be missed
    pub fn par_iter<'a>(&'a self) -> rayon::collections::hash_set::Iter<'a, T> {
        trace!("read {}", std::any::type_name::<Self>());
        // NOTE: if event counts overflow this would be a bug
        // if we are running for 100 years, we could sustain 5849 million updates per second
        // if we use 8 wide SIMD with 256 cores we could sustain 2.856 million updates per second
        let reader_id = self.reader_id;
        //        let to_read = self.event_counts[reader_id] - *reader_event_count;
        //        let offset = self.events[reader_id].len() - to_read;
        //        *reader_event_count += to_read;
        //        self.events[reader_id].iter().skip(offset)
        self.events[reader_id].par_iter()
    }

    pub fn clear(&mut self) {
        self.event_counts[0] = 0;
        self.event_counts[1] = 0;
        self.events[0].clear();
        self.events[1].clear();
    }
}

// Reads events of type `T` in order and tracks which events have already been read.
#[derive(SystemParam)]
pub struct UniqueEventReader<'w, 's, T: Resource + Hash + Eq> {
    //    last_event_count: Local<'s, (usize, PhantomData<T>)>,
    events: Res<'w, UniqueEvents<T>>,
    #[system_param(ignore)]
    marker: PhantomData<&'s usize>,
}
impl<'w, 's, T: Resource + Hash + Eq> UniqueEventReader<'w, 's, T> {
    pub fn iter<'a>(&'a mut self) -> std::collections::hash_set::Iter<'a, T> {
        self.events.iter()
    }
}

// Sends events of type `T`.
#[derive(SystemParam)]
pub struct UniqueEventWriter<'w, 's, T: Resource + Hash + Eq> {
    events: ResMut<'w, UniqueEvents<T>>,
    #[system_param(ignore)]
    marker: PhantomData<&'s usize>,
}

// what if multiple event writers
impl<'w, 's, T: Resource + Hash + Eq> UniqueEventWriter<'w, 's, T> {
    pub fn send(&mut self, event: T) {
        self.events.send(event);
    }

    pub fn send_batch(&mut self, events: impl Iterator<Item = T>) {
        self.events.send_batch(events);
    }
}
struct PortalState<T: Resource> {
    event_count: usize,
    reader_id: usize,
    events: [Vec<T>; 2],
    bound: usize,
}
pub struct PortalInner<T: Resource> {
    state: RwLock<PortalState<T>>,
    updated: Notify,
}
pub struct Portal<T: Resource> {
    inner: Arc<PortalInner<T>>,
}
impl<T: Resource> Portal<T> {
    pub fn new(bound: usize) -> Self {
        Self {
            inner: Arc::new(PortalInner {
                state: RwLock::new(PortalState {
                    event_count: 0,
                    reader_id: 0,
                    events: [Vec::new(), Vec::new()],
                    bound,
                }),
                updated: Default::default(),
            }),
        }
    }
}
impl<T: Resource> Portal<T> {
    pub fn try_send(&self, value: T) -> Option<T> {
        let mut state = self.inner.state.write().unwrap();
        let writer_id = state.reader_id ^ 1;
        let full = state.events[writer_id].len() >= state.bound;
        if full {
            return Some(value);
        }
        state.event_count += 1;
        state.events[writer_id].push(value);
        EVENT_LOOP.update();
        trace!("send {}", std::any::type_name::<Self>());
        None
    }

    async fn wait<'a>(&'a self) -> RwLockWriteGuard<'a, PortalState<T>> {
        loop {
            let full;
            {
                let loop_state = self.inner.state.write().unwrap();
                let writer_id = loop_state.reader_id ^ 1;
                full = loop_state.events[writer_id].len() >= loop_state.bound;
                if !full {
                    return loop_state;
                }
            }
            if full {
                self.inner.updated.notified().await;
            }
        }
    }

    pub async fn send(&self, value: T) {
        let mut guard = self.wait().await;
        let writer_id = guard.reader_id ^ 1;
        guard.event_count += 1;
        guard.events[writer_id].push(value);
        EVENT_LOOP.update();
        trace!("send {}", std::any::type_name::<Self>());
    }

    pub fn update(&mut self) {
        trace!("update {}", std::any::type_name::<Self>());
        let mut state = self.inner.state.write().unwrap();
        state.reader_id ^= 1;
        let writer_id = state.reader_id ^ 1;
        state.events[writer_id].clear();
        drop(state);
        self.inner.updated.notify_waiters();
    }

    pub fn update_system(mut portal: ResMut<Self>) {
        portal.update();
    }
}
impl<T: Resource> Clone for Portal<T> {
    /// Increases reference count
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
pub struct PipelinedPortal<T: Resource>(pub(crate) Portal<Pipelined<T>>);

impl<T: Resource> PipelinedPortal<T> {
    pub async fn send(&self, event: T, entity: Entity) {
        self.0.send(Pipelined::new(entity, event)).await
    }

    pub fn update_system(mut portal: ResMut<Self>) {
        portal.0.update();
    }
}
impl<T: Resource> Clone for PipelinedPortal<T> {
    /// Increases reference count
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(SystemParam)]
pub struct PipelinedPortalReader<'w, 's, T: Resource> {
    portal: Res<'w, PipelinedPortal<T>>,
    query: Query<'w, 's, &'static Pipeline>,
}
impl<'w, 's, T: Resource> PipelinedPortalReader<'w, 's, T> {
    /// User must read all events, otherwise it would be missed
    pub fn read<'a>(&'a self) -> PipelinedPortalReadGuard<'w, 's, 'a, T> {
        trace!("read {}", std::any::type_name::<Self>());
        let state = self.portal.0.inner.state.read().unwrap();
        PipelinedPortalReadGuard {
            guard: state,
            query: &self.query,
        }
    }
}
pub struct PipelinedPortalReadGuard<'w, 's, 'a, T: Resource> {
    guard: RwLockReadGuard<'a, PortalState<Pipelined<T>>>,
    query: &'a Query<'w, 's, &'static Pipeline>,
}
impl<'w, 's, 'a, T: Resource> PipelinedPortalReadGuard<'w, 's, 'a, T> {
    pub fn iter(
        &'a self,
    ) -> Filter<std::slice::Iter<'a, Pipelined<T>>, PipelinedFilter<'w, 's, 'a, T>> {
        // we must filter instead of clearing
        // if we use clearing an event could be sent right after being cleared but before knowing
        // that pipeline is flushing
        // to circumwent that we could clear on 2 updates but then we could clear good event
        let reader_id = self.guard.reader_id;
        self.guard.events[reader_id][..]
            .iter()
            .filter(PipelinedFilter {
                query: &self.query,
                _t: Default::default(),
            })
    }

    pub fn par_iter(
        &'a self,
    ) -> rayon::iter::Filter<rayon::slice::Iter<'a, Pipelined<T>>, PipelinedFilter<'w, 's, 'a, T>>
    {
        let reader_id = self.guard.reader_id;
        self.guard.events[reader_id][..]
            .par_iter()
            .filter(PipelinedFilter {
                query: &self.query,
                _t: Default::default(),
            })
    }
}

#[derive(SystemParam)]
pub struct PortalReader<'w, 's, T: Resource> {
    portal: Res<'w, Portal<T>>,
    #[system_param(ignore)]
    marker: PhantomData<&'s usize>,
}
impl<'w, 's, T: Resource> PortalReader<'w, 's, T> {
    /// User must read all events, otherwise it would be missed
    pub fn read<'a>(&'a self) -> PortalReadGuard<'a, T> {
        trace!("read {}", std::any::type_name::<Self>());
        let state = self.portal.inner.state.read().unwrap();
        PortalReadGuard { guard: state }
    }
}
pub struct PortalReadGuard<'a, T: Resource> {
    guard: RwLockReadGuard<'a, PortalState<T>>,
}
impl<'a, T: Resource> PortalReadGuard<'a, T> {
    pub fn iter(&'a self) -> std::slice::Iter<'a, T> {
        let reader_id = self.guard.reader_id;
        self.guard.events[reader_id][..].iter()
    }

    pub fn par_iter(&'a self) -> rayon::slice::Iter<'a, T> {
        let reader_id = self.guard.reader_id;
        self.guard.events[reader_id][..].par_iter()
    }
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum EventLoopEvent {
    Exit,
}
pub struct EventLoopReadGuard<'a> {
    should_update: bool,
    guard: MutexGuard<'a, VecDeque<EventLoopEvent>>,
}
impl<'a> EventLoopReadGuard<'a> {
    pub fn update_needed(&self) -> bool {
        self.should_update
    }

    pub fn iter<'b>(&'b mut self) -> std::collections::vec_deque::Drain<'b, EventLoopEvent> {
        self.guard.drain(..)
    }
}

#[derive(Default)]
pub struct EventLoop {
    should_update: AtomicBool,
    events: Mutex<VecDeque<EventLoopEvent>>,
    changed: Notify,
}

impl EventLoop {
    pub fn wait_for_events(&self) -> Notified<'_> {
        self.changed.notified()
    }

    pub fn read(&self) -> EventLoopReadGuard {
        let guard = self.events.lock().unwrap();
        EventLoopReadGuard {
            should_update: self.should_update.fetch_and(false, Ordering::SeqCst),
            guard,
        }
    }

    pub fn send(&self, event: EventLoopEvent) {
        self.events.lock().unwrap().push_back(event);
        self.changed.notify_one();
    }

    pub fn update(&self) {
        self.should_update.store(true, Ordering::SeqCst);
        self.changed.notify_one();
    }
}

lazy_static! {
    pub static ref EVENT_LOOP: EventLoop = Default::default();
}

#[derive(Debug)]
pub enum MyEvent {
    Update,
    Exit,
}
