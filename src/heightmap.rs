// ----------------------------------------------------------------------------
// max (default) storage binding size 128mb //TODO is this guaranteed?
// mapsize 16384 * 512 rows * 12 byte (result buf with normals) = 96MB
const COMPUTE_NORMALS_MAX_ROWS: usize = 1024;
// ----------------------------------------------------------------------------
use bevy::ecs::schedule::StateData;
use bevy::prelude::*;

use crate::cmds::{AsyncTaskFinishedEvent, AsyncTaskStartEvent};
use crate::compute::{AppComputeNormalsTask, ComputeResultData, ComputeResults};
use crate::config::TerrainConfig;
// ----------------------------------------------------------------------------
pub struct HeightmapPlugin;
// ----------------------------------------------------------------------------
impl HeightmapPlugin {
    // ------------------------------------------------------------------------
    pub fn generate_heightmap_normals<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_update(state).with_system(generate_heightmap_normals)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Plugin for HeightmapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ComputeNormalsTaskQueue>()
            .init_resource::<TerrainHeightMap>()
            .init_resource::<TerrainNormals>();
    }
}
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct TerrainHeightMap {
    size: u32,
    data: Vec<u16>,
    height_scaling: f32,
}
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct TerrainNormals {
    size: u32,
    data: Vec<[f32; 3]>,
}
// ----------------------------------------------------------------------------
impl TerrainHeightMap {
    // ------------------------------------------------------------------------
    pub(crate) fn new(size: u32, height_scaling: f32, data: Vec<u16>) -> Self {
        Self {
            size,
            data,
            height_scaling,
        }
    }
    // ------------------------------------------------------------------------
    pub(crate) fn update(&mut self, new_heightmap: TerrainHeightMap) {
        self.size = new_heightmap.size;
        self.data = new_heightmap.data;
        self.height_scaling = new_heightmap.height_scaling;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(Default)]
struct ComputeNormalsTaskQueue {
    queue: Vec<AppComputeNormalsTask>,
    pending: usize,
}
// ----------------------------------------------------------------------------
impl ComputeNormalsTaskQueue {
    // ------------------------------------------------------------------------
    /// returns true if last pending was "finished" and queue is empty
    fn finished(&mut self, count: usize) -> bool {
        if self.pending > 0 {
            self.pending = self.pending.saturating_sub(count);
            self.pending == 0 && self.queue.is_empty()
        } else {
            false
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
/// system for any heightmap generation
fn generate_heightmap_normals(
    mut commands: Commands,
    mut tasks_queued: EventReader<AsyncTaskStartEvent>,
    mut task_finished: EventWriter<AsyncTaskFinishedEvent>,
    mut terrain_normals: ResMut<TerrainNormals>,
    mut compute_queue: ResMut<ComputeNormalsTaskQueue>,
    terrain_heightmap: Res<TerrainHeightMap>,
    terrain_config: Res<TerrainConfig>,
    compute_results: Res<ComputeResults>,
) {
    for task in tasks_queued.iter() {
        if let AsyncTaskStartEvent::GenerateHeightmapNormals = task {
            // since big maps are supported generate compute tasks with smaller
            // heightmap slices. put all tasks into a queue so they can be pushed
            // to GPU in subsequent frames/after results arrive
            debug!("generating normals...");

            let data_width = terrain_heightmap.size as usize;

            assert!(data_width.is_power_of_two());

            // scale down with map size (mostly for dev envs)
            let max_rows = data_width.min(COMPUTE_NORMALS_MAX_ROWS);

            let last_slice = data_width / max_rows - 1;

            for slice in 0..=last_slice {
                // Note: two additional rows are required: one before the data
                // and one after the data to allow "previous" and "next" row access
                let mut data = vec![0u16; data_width * (max_rows + 2)];
                let heightmap = &terrain_heightmap.data;

                if last_slice == 0 {
                    // only one slice -> duplicate first and last row
                    let first = 0..data_width;
                    let data_segment = first.end..first.end + data_width * max_rows;
                    let last = data_segment.end..;

                    data[first].copy_from_slice(&heightmap[0..data_width]);
                    data[data_segment].copy_from_slice(&heightmap[..]);
                    data[last].copy_from_slice(&heightmap[heightmap.len() - data_width..]);
                } else if slice == 0 {
                    // first slice -> duplicate first line
                    let start = 0;
                    let end = start + data_width * (max_rows + 1);

                    data[0..data_width].copy_from_slice(&heightmap[0..data_width]);
                    data[data_width..].copy_from_slice(&heightmap[start..end]);
                } else if slice == last_slice {
                    // last -> duplicate last line
                    let len = data_width * (max_rows + 1);
                    let start = slice * data_width * max_rows - data_width;
                    let end = start + len;

                    data[0..len].copy_from_slice(&heightmap[start..end]);
                    data[len..].copy_from_slice(&heightmap[end - data_width..]);
                } else {
                    let start = slice * data_width * max_rows - data_width;
                    let end = start + data_width * (max_rows + 2);

                    data.copy_from_slice(&heightmap[start..end]);
                }

                compute_queue.queue.push(AppComputeNormalsTask {
                    map_resolution: terrain_config.resolution(),
                    map_height_scaling: terrain_config.height_scaling(),
                    data_width: terrain_heightmap.size,
                    data_rows: max_rows as u32,
                    data_offset: slice * data_width * max_rows,
                    data: Some(data),
                });
            }

            // intialize current normals to new size
            *terrain_normals = TerrainNormals {
                size: terrain_heightmap.size,
                data: vec![
                    [0.0, 1.0, 0.0];
                    (terrain_heightmap.size * terrain_heightmap.size) as usize
                ],
            };
        }
    }

    if compute_queue.pending == 0 {
        if let Some(compute_task) = compute_queue.queue.pop() {
            compute_queue.pending += 1;
            let taskid = commands.spawn().insert(compute_task).id();
            debug!(
                "submitting compute normals task {:?}. remaining: {}",
                taskid,
                compute_queue.queue.len()
            );
        }
    }

    while let Ok((taskid, ComputeResultData::ComputeNormals(result))) =
        compute_results.receiver.try_recv()
    {
        debug!(
            "received compute normals result {:?} data at {}: {:?}",
            taskid,
            result.offset,
            result.normals.len()
        );
        // store data slice in full normals buffer
        let start = result.offset;
        let end = start + result.normals.len();
        terrain_normals.data[start..end].copy_from_slice(&result.normals);

        // cleanup finished compute task trigger
        commands.entity(taskid).despawn();

        if compute_queue.finished(1) {
            terrain_normals.set_changed();
            task_finished.send(AsyncTaskFinishedEvent::HeightmapNormalsGenerated);
        }
    }
}
// ----------------------------------------------------------------------------
