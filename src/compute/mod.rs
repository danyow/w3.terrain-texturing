// ----------------------------------------------------------------------------
mod cache;
mod computetask;
mod normals;
// ----------------------------------------------------------------------------
use async_channel::{Receiver, Sender};
use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderGraph},
        render_resource::{
            BufferAsyncError, BufferSlice, CommandEncoder, ComputePipeline, Maintain,
        },
        renderer::{RenderContext, RenderDevice},
        RenderApp, RenderStage,
    },
    utils::HashMap,
};
use futures_lite::Future;

use self::normals::{ComputeNormalsResult, GpuComputeNormals};
// ----------------------------------------------------------------------------
pub use cache::*;
pub use normals::AppComputeNormalsTask;
// ----------------------------------------------------------------------------
#[derive(Component)]
/// simplification: used as wrapper for all used compute task types
pub enum GpuComputeTask {
    ComputeNormals(GpuComputeNormals),
    // MipGeneration,
}
// ----------------------------------------------------------------------------
pub enum ComputeResultData {
    ComputeNormals(ComputeNormalsResult),
}
// ----------------------------------------------------------------------------
pub type TaskId = Entity;
// ----------------------------------------------------------------------------
pub struct ComputeResults {
    pub receiver: Receiver<(TaskId, ComputeResultData)>,
}
// ----------------------------------------------------------------------------
pub struct GpuComputeTaskPlugin;
// ----------------------------------------------------------------------------
impl Plugin for GpuComputeTaskPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        // plugin internal channel to receive compute task started events
        // (allowing to query *after* next command queue submission in subsequent
        // system)
        let (task_dispatch_notify, task_dispatch_receiver) = async_channel::unbounded();

        // channel to push compute task results to app world
        let (taskresult_sender, taskresult_receiver) = async_channel::unbounded();

        // receives the finished data for a task
        app.insert_resource(ComputeResults {
            receiver: taskresult_receiver,
        });

        let render_device = app.world.get_resource_mut::<RenderDevice>().unwrap();
        let compute_pipeline_cache = ComputePipelineCache::new(render_device.clone());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .insert_resource(compute_pipeline_cache)
            .init_resource::<PreparedGpuTasks>()
            .init_resource::<ComputeTaskQueue>()
            .insert_resource(PendingComputeResults {
                task_result_dispatcher: taskresult_sender,
                task_started_listener: task_dispatch_receiver,
            })
            // FIXME shader assets not supported due to duplication with "normal" RenderPipelineCache
            // .add_system_to_stage(RenderStage::Extract, ComputePipelineCache::extract_shaders)
            .add_system_to_stage(RenderStage::Queue, queue_tasks)
            .add_system_to_stage(
                RenderStage::Render,
                ComputePipelineCache::process_pipeline_queue_system,
            )
            // tasks can only be checked *after* the command queue was submitted!
            // so this has to be *after* the render phase
            .add_system_to_stage(RenderStage::Cleanup, check_task_results);

        let compute_node = ComputePassNode::new(&mut render_app.world, task_dispatch_notify);

        let mut render_graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_node("compute_task_pass", compute_node);

        // depends on ComputePipelineCache
        app.add_plugin(normals::ComputeNormalsPlugin);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// prepare + queue tasks
// ----------------------------------------------------------------------------
impl GpuComputeTask {
    // ------------------------------------------------------------------------
    fn record_commands(&self, pipeline: &ComputePipeline, cmd_encoder: &mut CommandEncoder) {
        match self {
            GpuComputeTask::ComputeNormals(t) => t.record_commands(pipeline, cmd_encoder),
        }
    }
    // ------------------------------------------------------------------------
    fn wait_for_result(
        &self,
    ) -> impl Future<Output = Result<BufferSlice, BufferAsyncError>> + Send {
        use futures_lite::FutureExt;

        match self {
            GpuComputeTask::ComputeNormals(t) => t.wait_for_result().boxed(),
        }
    }
    // ------------------------------------------------------------------------
    fn get_result<'a>(
        &'a self,
        wait_future: impl Future<Output = Result<BufferSlice<'a>, BufferAsyncError>> + Send,
    ) -> Result<ComputeResultData, BufferAsyncError> {
        use ComputeResultData::*;

        match self {
            GpuComputeTask::ComputeNormals(t) => t.get_result(wait_future).map(ComputeNormals),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(Default)]
/// Processed by ComputeTaskNode to retrieve GpuComputeTask data entity from
/// render world and pipeline from pipeline cache
struct ComputeTaskQueue {
    tasks: HashMap<Entity, CachedPipelineId>,
}
// ----------------------------------------------------------------------------
impl ComputeTaskQueue {
    // ------------------------------------------------------------------------
    fn clear(&mut self) {
        self.tasks.clear();
    }
    // ------------------------------------------------------------------------
    fn add_task(&mut self, taskid: Entity, pipelineid: CachedPipelineId) {
        self.tasks.insert(taskid, pipelineid);
    }
    // ------------------------------------------------------------------------
    fn remove_task(&mut self, taskid: Entity) {
        self.tasks.remove(&taskid);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(Default)]
struct PreparedGpuTasks(Vec<(Entity, GpuComputeTask, CachedPipelineId)>);
// ----------------------------------------------------------------------------
impl PreparedGpuTasks {
    // ------------------------------------------------------------------------
    pub fn add(&mut self, taskid: Entity, task: GpuComputeTask, pipeline: CachedPipelineId) {
        self.0.push((taskid, task, pipeline));
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
fn queue_tasks(
    mut commands: Commands,
    mut queue: ResMut<ComputeTaskQueue>,
    mut prepared: ResMut<PreparedGpuTasks>,
) {
    queue.clear();
    for (entity, task, pipeline_id) in prepared.0.drain(..) {
        commands.get_or_spawn(entity).insert(task);
        queue.add_task(entity, pipeline_id);
    }
}
// ----------------------------------------------------------------------------
// wait for completion and send results to app world
// ----------------------------------------------------------------------------
type DispatchedTaskEvent = Entity;

struct PendingComputeResults {
    task_result_dispatcher: Sender<(TaskId, ComputeResultData)>,
    task_started_listener: Receiver<DispatchedTaskEvent>,
}
// ----------------------------------------------------------------------------
fn check_task_results(
    mut commands: Commands,
    results_queue: Res<PendingComputeResults>,
    mut task_queue: ResMut<ComputeTaskQueue>,
    render_device: Res<RenderDevice>,
    query: Query<&mut GpuComputeTask>,
) {
    let mut pending = Vec::default();

    // collect all waiting futures first so device_poll needs to be called only
    // once
    while let Ok(taskid) = results_queue.task_started_listener.try_recv() {
        let compute_task = query
            .get_component::<GpuComputeTask>(taskid)
            .expect("GpuComputeTask from taskid");

        pending.push((taskid, compute_task.wait_for_result(), compute_task));

        // remove task from queue and render world
        task_queue.remove_task(taskid);
        commands.entity(taskid).despawn();
    }

    if !pending.is_empty() {
        // TODO does this have to be blocking?
        // following may return pending, but future will be consumed by poll_once
        // so how to repoll?
        //
        // render_device.poll(Maintain::Poll);
        //
        // let result = match future::block_on(future::poll_once(result_future)) {
        //     Some(Ok(buf_slice)) => compute_task.get_result(buf_slice),
        //     Some(Err(e)) => panic!("{}", e),
        //     None => { ? }
        // };
        //
        // -> moved blocking wait into GpuComputeTask

        // Note: device.poll has to be called *after* wait_for_result (slice.map_async)
        // and before get_result!
        render_device.poll(Maintain::Wait);

        for (taskid, wait_token, compute_task) in pending.drain(..) {
            match compute_task.get_result(wait_token) {
                Ok(result) => {
                    results_queue
                        .task_result_dispatcher
                        .try_send((taskid, result))
                        .map_err(|e| {
                            error!(
                                "failed to send compute task result for task {:?}: {}",
                                taskid, e
                            )
                        })
                        .ok();
                }
                Err(e) => error!("failed to extract compute task result: {}", e),
            }
        }
    }
}
// ----------------------------------------------------------------------------
// general computepass node for (and only for) GpuComputeTasks
// ----------------------------------------------------------------------------
struct ComputePassNode {
    tasks_query: QueryState<&'static GpuComputeTask>,
    task_dispatched_sender: Sender<DispatchedTaskEvent>,
}
// ----------------------------------------------------------------------------
impl ComputePassNode {
    // ------------------------------------------------------------------------
    pub fn new(world: &mut World, sender: Sender<DispatchedTaskEvent>) -> Self {
        Self {
            tasks_query: QueryState::new(world),
            task_dispatched_sender: sender,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl render_graph::Node for ComputePassNode {
    // ------------------------------------------------------------------------
    fn update(&mut self, world: &mut World) {
        self.tasks_query.update_archetypes(world);
    }
    // ------------------------------------------------------------------------
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.get_resource::<ComputePipelineCache>().unwrap();
        let queue = world.get_resource::<ComputeTaskQueue>().unwrap();

        if !queue.tasks.is_empty() {
            for (taskid, pipelineid) in queue.tasks.iter() {
                let task = self.tasks_query.get_manual(world, *taskid).unwrap();

                let pipeline = pipeline_cache.get(*pipelineid).unwrap();

                task.record_commands(pipeline, &mut render_context.command_encoder);

                self.task_dispatched_sender
                    .try_send(*taskid)
                    .map_err(|e| error!("failed to send compute task dispatch event: {}", e))
                    .ok();
            }
        }
        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
