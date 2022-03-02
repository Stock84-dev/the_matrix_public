use std::marker::PhantomData;
use std::sync::Arc;

use bevy::app::App;
use bevy::utils::HashMap;
use mouse::sync::RwLock;
use zion_db::definitions::{WorkflowId, WorkflowLayout, WorkflowLayoutId};
use zion_db::error::{ZionError, ZionResult};

use crate::system::{SystemManager, WorkflowSystems};
use crate::topic::{TempTopicsLookup, TopicManager};

pub struct WorkflowManager {
    workflows: HashMap<WorkflowLayoutId, WorkflowLayout>,
    //    system_layouts: HashMap<SystemLayoutId, SystemLayout>,
    //    topic_layouts: HashMap<TopicId, TopicLayout>,
    last_workflow_id: u64,
    topic_manager: TopicManager,
    system_manager: SystemManager,
}

impl WorkflowManager {
    pub fn new() -> Self {
        Self {
            workflows: Default::default(),
            last_workflow_id: 0,
            topic_manager: TopicManager::new(),
            system_manager: SystemManager::new(),
        }
    }

    pub async fn spawn(&mut self, app: &mut App, workflow: &WorkflowLayout) -> ZionResult<()> {
        let workflow_id = WorkflowId(self.last_workflow_id);
        let tmp_topics_lookup = self.topic_manager.spawn_topics(
            app,
            workflow
                .systems
                .iter()
                .map(|x| {
                    x.writer_topics
                        .iter()
                        .map(|x| &x.topic_layout)
                        .chain(x.reader_topics.iter().map(|x| &x.topic_layout))
                })
                .flatten(),
        )?;

        let workflow_systems =
            match self
                .system_manager
                .spawn(app, &workflow.systems, &tmp_topics_lookup, workflow_id)
            {
                Ok(ws) => ws,
                Err(e) => {
                    if let Err(roll_e) = self.topic_manager.rollback(app) {
                        return Err(ZionError::TransactionFailedToRollback {
                            transaction_error: Box::new(roll_e),
                            cause: Box::new(e),
                        });
                    }
                    return Err(ZionError::FailedToSpawnSystems(Box::new(e)));
                }
            };

        app.world.spawn().insert(workflow_id);
        self.last_workflow_id += 1;
        Ok(())
    }
}
