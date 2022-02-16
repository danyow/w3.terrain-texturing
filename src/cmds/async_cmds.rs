// ----------------------------------------------------------------------------
use enum_dispatch::enum_dispatch;

use bevy::{prelude::*, tasks::IoTaskPool, utils::HashSet};

use crate::config;
use crate::heightmap::TerrainHeightMap;
use crate::loader::LoaderPlugin;
use crate::{TaskResult, TaskResultData};

use super::{
    AsyncTask, AsyncTaskFinishedEvent, AsyncTaskStartEvent, GenerateHeightmapNormals,
    LoadHeightmap, LoadTerrainMaterialSet, WaitForTerrainLoaded, GenerateTerrainTiles,
};
// ----------------------------------------------------------------------------
pub struct AsyncCmdsPlugin;
// ----------------------------------------------------------------------------
#[derive(Default)]
/// manages dependent sub cmds/tasks of async cmds. provides start events for
/// subsequent tasks/cmds if all preconditions are met.
pub struct AsyncCommandManager {
    changed: bool,
    ready: HashSet<AsyncTaskFinishedEvent>,
    pending: HashSet<AsyncTask>,
}
// ----------------------------------------------------------------------------
impl Plugin for AsyncCmdsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AsyncCommandManager>()
            .add_event::<AsyncTaskStartEvent>()
            .add_event::<AsyncTaskFinishedEvent>();
    }
}
// ----------------------------------------------------------------------------
//TODO some form cancelation of future ready updates if any fails until some
// reset/other condition is met (inflight event tracking?/generation tracking/entity + component)
impl AsyncCommandManager {
    // ------------------------------------------------------------------------
    pub fn add_new(&mut self, task: AsyncTask) {
        // add task and all subsequent dependending tasks to pending and remove
        // ready events
        self.ready.remove(&task.ready_event());
        for subsequent in task.subsequent_tasks() {
            self.add_new(subsequent);
        }
        self.pending.insert(task);
        self.changed = true;
    }
    // ------------------------------------------------------------------------
    fn update(&mut self, ready_event: AsyncTaskFinishedEvent) {
        debug!("AsyncCommandManager.update: {:?}", ready_event);
        self.changed = self.ready.insert(ready_event) || self.changed;
    }
    // ------------------------------------------------------------------------
    fn get_start_events(&mut self) -> Option<Vec<AsyncTaskStartEvent>> {
        if self.changed && !self.pending.is_empty() {
            self.changed = false;
            // check preconditions of pending events and trigger start if
            // conditions are met
            let mut still_pending = HashSet::default();
            let mut start_events = Vec::new();
            for pending in self.pending.drain() {
                debug!("checking [{:?}] preconditions: {:?}", self.ready, pending);
                if pending
                    .preconditions()
                    .iter()
                    .any(|c| !self.ready.contains(c))
                {
                    debug!("some precondition still pending");
                    still_pending.insert(pending);
                } else {
                    // all pre conditions met
                    debug!("all precondition met");
                    start_events.push(pending.start_event());
                }
            }
            // remove all started from pending list
            self.pending = still_pending;
            if !start_events.is_empty() {
                return Some(start_events);
            }
        }
        None
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub(crate) fn start_async_operations(
    mut commands: Commands,
    mut async_cmd_tracker: ResMut<AsyncCommandManager>,
    mut tasks_finished: EventReader<AsyncTaskFinishedEvent>,
    mut task_ready: EventWriter<AsyncTaskStartEvent>,
    thread_pool: Res<IoTaskPool>,
    terrain_config: Res<config::TerrainConfig>,
) {
    for task in tasks_finished.iter().copied() {
        async_cmd_tracker.update(task);
    }

    if let Some(mut new_tasks) = async_cmd_tracker.get_start_events() {
        use AsyncTaskStartEvent::*;

        for task in new_tasks.drain(..) {
            match task {
                // -- these tasks can be handled by futures
                LoadHeightmap => {
                    let task = thread_pool.spawn(LoaderPlugin::load_heightmap(&terrain_config));
                    commands.spawn().insert(task);
                }
                LoadTerrainMaterialSet => task_ready.send(LoadTerrainMaterialSet),
                // -- these tasks are more involved and will be handled by specialized systems
                GenerateHeightmapNormals => task_ready.send(GenerateHeightmapNormals),
                GenerateTerrainTiles => task_ready.send(GenerateTerrainTiles),
                // -- these are just wrapper for sinks (join multiple events but do nothing)
                WaitForTerrainLoaded => task_ready.send(WaitForTerrainLoaded),
            }
        }
    }
}
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub(crate) fn poll_async_task_state(
    mut commands: Commands,
    mut pending_futures: Query<(Entity, &mut TaskResult)>,
    mut task_finished: EventWriter<AsyncTaskFinishedEvent>,
    mut task_ready: EventReader<AsyncTaskStartEvent>,
    mut terrain_heightmap: ResMut<TerrainHeightMap>,
) {
    use futures_lite::future;

    for (entity, mut task) in pending_futures.iter_mut() {
        if let Some(task_result) = future::block_on(future::poll_once(&mut *task)) {
            commands.entity(entity).despawn();

            match task_result {
                Ok(result) => match result {
                    TaskResultData::HeightmapData(new_heightmap) => {
                        info!("loading heightmap...finished");
                        // must be updated in place as commands.insert_resource is queued but
                        // event may trigger next step earlier
                        // commands.insert_resource(new_heightmap);
                        terrain_heightmap.update(new_heightmap);

                        task_finished.send(AsyncTaskFinishedEvent::HeightmapLoaded);
                    }
                },
                Err(e) => {
                    //TODO this involves canceling all futures and stoping other tasks
                    error!("{}", e);
                }
            }
        }
    }

    // some tasks can be used as generaic wait until without any work and need
    // to transition to finished state directly
    for task in task_ready.iter().copied() {
        if let AsyncTaskStartEvent::WaitForTerrainLoaded = task {
            task_finished.send(AsyncTaskFinishedEvent::TerrainLoaded);
        }
    }
}
// ----------------------------------------------------------------------------
// dependencies between async tasks
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
#[enum_dispatch(AsyncTask)]
#[rustfmt::skip]
trait AsyncTaskNode {
    fn preconditions(&self) -> &[AsyncTaskFinishedEvent] { &[]}
    fn start_event(self) -> AsyncTaskStartEvent;
    fn ready_event(&self) -> AsyncTaskFinishedEvent;
    fn subsequent_tasks(&self) -> Vec<AsyncTask> { Vec::default() }
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
impl AsyncTaskNode for LoadHeightmap {
    fn start_event(self) -> AsyncTaskStartEvent { AsyncTaskStartEvent::LoadHeightmap }
    fn ready_event(&self) -> AsyncTaskFinishedEvent { AsyncTaskFinishedEvent::HeightmapLoaded }
    fn subsequent_tasks(&self) -> Vec<AsyncTask> {
        vec![
            GenerateHeightmapNormals::default().into(),
            GenerateTerrainTiles::default().into(),
        ]
    }
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
impl AsyncTaskNode for GenerateHeightmapNormals {
    fn preconditions(&self) -> &[AsyncTaskFinishedEvent] { &[AsyncTaskFinishedEvent::HeightmapLoaded] }
    fn start_event(self) -> AsyncTaskStartEvent { AsyncTaskStartEvent::GenerateHeightmapNormals }
    fn ready_event(&self) -> AsyncTaskFinishedEvent { AsyncTaskFinishedEvent::HeightmapNormalsGenerated }
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
impl AsyncTaskNode for GenerateTerrainTiles {
    fn preconditions(&self) -> &[AsyncTaskFinishedEvent] { &[AsyncTaskFinishedEvent::HeightmapLoaded] }
    fn start_event(self) -> AsyncTaskStartEvent { AsyncTaskStartEvent::GenerateTerrainTiles }
    fn ready_event(&self) -> AsyncTaskFinishedEvent { AsyncTaskFinishedEvent::TerrainTilesGenerated }
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
impl AsyncTaskNode for LoadTerrainMaterialSet {
    fn start_event(self) -> AsyncTaskStartEvent { AsyncTaskStartEvent::LoadTerrainMaterialSet }
    fn ready_event(&self) -> AsyncTaskFinishedEvent { AsyncTaskFinishedEvent::TerrainMaterialSetLoaded }
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
impl AsyncTaskNode for WaitForTerrainLoaded {
    fn preconditions(&self) -> &[AsyncTaskFinishedEvent] { &[
        AsyncTaskFinishedEvent::HeightmapNormalsGenerated,
        AsyncTaskFinishedEvent::TerrainTilesGenerated,
        AsyncTaskFinishedEvent::TerrainMaterialSetLoaded,
    ]}
    fn start_event(self) -> AsyncTaskStartEvent { AsyncTaskStartEvent::WaitForTerrainLoaded }
    fn ready_event(&self) -> AsyncTaskFinishedEvent { AsyncTaskFinishedEvent::TerrainLoaded }
}
// ----------------------------------------------------------------------------
