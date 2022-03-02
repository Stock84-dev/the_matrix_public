use std::sync::Arc;

use bevy::prelude::*;
use bevy::utils::HashMap;
use zion_db::definitions::{TopicId, TopicKind};
use zion_db::error::ZionResult;

use crate::topic::{TopicReader, TopicWriter};

#[derive(Default)]
pub struct BevyTopics(Vec<Option<Arc<dyn DynBevyTopic>>>);

impl BevyTopics {
    pub fn spawn(&mut self, topic: Arc<dyn DynBevyTopic>) -> usize {
        match self.0.iter_mut().find_position(|x| x.is_none()) {
            None => {
                let id = self.0.len();
                self.0.push(Some(topic));
                id
            }
            Some((i, old)) => {
                *old = Some(topic);
                i
            }
        }
    }

    pub fn despawn(&mut self, id: usize) -> Option<Arc<dyn DynBevyTopic>> {
        self.0.get_mut(id)?.take()
    }

    pub fn get(&self, id: usize) -> Option<&Option<Arc<dyn DynBevyTopic>>> {
        self.0.get(id)
    }
}

#[derive(Default)]
pub struct TopicIdToBevyTopicId(pub HashMap<TopicId, Entity>);

pub trait DynBevyTopic: DowncastSync + 'static {}
impl_downcast!(sync DynBevyTopic);

pub struct BevyTopicReader<T: Resource> {
    topic: Arc<BevyTopic<T>>,
}

impl<T: Resource> BevyTopicReader<T> {
    pub fn read(&self) -> &[T] {
        self.topic.read()
    }
}

impl<T: Resource> TopicReader for BevyTopicReader<T> {
    const TOPIC_KIND: TopicKind = TopicKind::Bevy;

    fn new(world: &World, id: usize) -> ZionResult<Self> {
        Ok(Self {
            topic: get_bevy_topic::<T>(world, id)?,
        })
    }
}

pub struct BevyTopicWriter<T: Resource> {
    topic: Arc<BevyTopic<T>>,
}

impl<T: Resource> BevyTopicWriter<T> {
    pub fn write(&self, event: T) {
        self.topic.write(event);
    }

    pub fn write_all(&self, events: impl IntoIterator<Item = T>) {
        self.topic.write_all(events);
    }
}

impl<T: Resource> TopicWriter for BevyTopicWriter<T> {
    const TOPIC_KIND: TopicKind = TopicKind::Bevy;

    fn new(world: &World, id: usize) -> ZionResult<Self> {
        Ok(Self {
            topic: get_bevy_topic::<T>(world, id)?,
        })
    }
}

fn get_bevy_topic<T: Resource>(world: &World, id: usize) -> ZionResult<Arc<BevyTopic<T>>> {
    Ok(world
        .get_resource::<BevyTopics>()
        .ok()?
        .get(id)
        .ok()?
        .ok()?
        .downcast_arc::<BevyTopic<T>>()
        .ok()
        .ok()?)
}

#[derive(Default, Component)]
pub struct BevyTopic<T: Resource> {
    write_events: spin::Mutex<Vec<T>>,
    read_events: Vec<T>,
}

impl<T: Resource> BevyTopic<T> {
    pub fn new() -> Self {
        Self { write_events: Default::default(), read_events: vec![] }
    }
    pub fn read(&self) -> &[T] {
        &self.read_events
    }

    pub fn write(&self, event: T) {
        self.write_events.lock().push(event);
    }

    pub fn write_all(&self, events: impl IntoIterator<Item = T>) {
        self.write_events.lock().extend(events);
    }

    /// This must be called when there are no writers and readers currently operating on this topic
    /// Should be put on Stage::Last
    pub unsafe fn update(&self) {
        let write = self.write_events.lock();
        std::mem::swap(self.read_events.as_mut_cast(), &mut *write);
    }

    pub unsafe fn update_system(id: Local<usize>, topics: ResMut<BevyTopics>) {
        topics.get(*id).map(|x| {
            x.as_ref()
                .map(|x| x.downcast_arc::<BevyTopic<T>>().map(|x| x.update()))
        });
    }
}

impl<T: Resource> DynBevyTopic for BevyTopic<T> {}
