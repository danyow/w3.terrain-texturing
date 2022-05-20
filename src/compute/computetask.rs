use bevy::prelude::*;
use bevy::render::render_resource::{SpecializedComputePipeline, SpecializedComputePipelines};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::{RenderApp, RenderStage};
use std::marker::PhantomData;

use super::{GpuComputeTask, PipelineCache, PreparedGpuTasks};
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// simplified compute task transformation into renderworld.
// Note: only *added* tasks from app world are extracted into render world
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
pub trait ComputeTask: Component + Send + Sync {
    type ExtractedComputeTask: Component + Send + Sync + 'static;

    type ComputePipeline: SpecializedComputePipeline + Send + Sync;

    fn extract_task(&mut self) -> Self::ExtractedComputeTask;

    fn specialization_key(
        extracted_task: &Self::ExtractedComputeTask,
    ) -> <Self::ComputePipeline as SpecializedComputePipeline>::Key;

    fn prepare_task(
        extracted_task: Self::ExtractedComputeTask,
        render_device: &RenderDevice,
        pipeline: &Self::ComputePipeline,
        render_queue: &RenderQueue,
    ) -> GpuComputeTask;
}

pub struct ComputeTaskPlugin<T: ComputeTask>(PhantomData<fn() -> T>);

impl<T: ComputeTask> Default for ComputeTaskPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: ComputeTask> Plugin for ComputeTaskPlugin<T> {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<ExtractedComputeTasks<T>>()
            .add_system_to_stage(
                RenderStage::Extract,
                extract_compute_task_from_app_world::<T>,
            )
            .add_system_to_stage(
                RenderStage::Prepare,
                prepare_compute_task::<T>.after("prepare_render_asset"),
            );
    }
}

pub struct ExtractedComputeTasks<T: ComputeTask> {
    tasks: Vec<(Entity, T::ExtractedComputeTask)>,
}

impl<T: ComputeTask> Default for ExtractedComputeTasks<T> {
    fn default() -> Self {
        Self {
            tasks: Default::default(),
        }
    }
}

fn extract_compute_task_from_app_world<T: ComputeTask>(
    mut commands: Commands,
    mut query: Query<(Entity, &mut T), Added<T>>,
) {
    if !query.is_empty() {
        let mut added_tasks = Vec::new();
        for (entity, mut task) in query.iter_mut() {
            added_tasks.push((entity, task.extract_task()));
        }
        commands.insert_resource::<ExtractedComputeTasks<T>>(ExtractedComputeTasks {
            tasks: added_tasks,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn prepare_compute_task<T: ComputeTask>(
    mut extracted: ResMut<ExtractedComputeTasks<T>>,
    mut prepared: ResMut<PreparedGpuTasks>,
    pipeline: Res<T::ComputePipeline>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut pipelines: ResMut<SpecializedComputePipelines<T::ComputePipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
) {
    for (taskid, task) in extracted.tasks.drain(..) {
        let pipeline_key = T::specialization_key(&task);
        let pipeline_id = pipelines.specialize(&mut pipeline_cache, &pipeline, pipeline_key);
        let prepared_task = T::prepare_task(task, &render_device, &pipeline, &render_queue);
        prepared.add(taskid, prepared_task, pipeline_id);
    }
}

// Hack/Workaround for error:
//
//  computetask.rs:91:
//      `<<T as ComputeTask>::ComputePipeline as SpecializedComputePipeline>::Key` cannot be sent between threads safely
//
// in bevy_render/src/render_resource/pipeline_specializer.rs
// pub trait SpecializedComputePipeline {
//     type Key: Clone + Hash + PartialEq + Eq + Send + Sync;       // Send + Sync added
//     [...]
// }
