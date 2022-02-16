// ----------------------------------------------------------------------------
// max (default) storage binding size 128mb //TODO is this guaranteed?
// mapsize 16384 * 512 rows * 12 byte (result buf with normals) = 96MB
const COMPUTE_NORMALS_MAX_ROWS: usize = 1024;
// ----------------------------------------------------------------------------
use std::sync::Arc;

use bevy::ecs::schedule::StateData;
use bevy::math::uvec2;
use bevy::prelude::*;

use crate::cmds::{AsyncTaskFinishedEvent, AsyncTaskStartEvent};
use crate::compute::{AppComputeNormalsTask, ComputeResultData, ComputeResults};
use crate::config::{TerrainConfig, TILE_SIZE};
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
#[derive(Clone, Copy)]
pub struct MinHeight(u16);
#[derive(Clone, Copy)]
pub struct MaxHeight(u16);
// ----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct TerrainTileId<const TILE_SIZE: u32>(u8, u8);
// ----------------------------------------------------------------------------
//TODO check if this redundant arc ref allocation can be fixed
#[allow(clippy::redundant_allocation)]
pub struct TerrainHeightMapView<'heightmap> {
    start_tile: TerrainTileId<TILE_SIZE>,
    heightmap: Arc<&'heightmap TerrainHeightMap>,
}
// ----------------------------------------------------------------------------
//TODO check if this redundant arc ref allocation can be fixed
#[allow(clippy::redundant_allocation)]
pub struct TerrainDataView<'heightmap, 'normals> {
    offset: UVec2,
    heightmap: Arc<&'heightmap TerrainHeightMap>,
    normals: Arc<&'normals TerrainNormals>,
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
// reduced views on heightmap/normals
// ----------------------------------------------------------------------------
impl TerrainHeightMap {
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn coordinates_to_offset(&self, p: UVec2) -> usize {
        // ensure that coordinates are within map by repeating last col & row
        (self.size * p.y.min(self.size - 1) + p.x.min(self.size - 1)) as usize
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn sample_interpolated_error(&self, a: UVec2, b: UVec2, middle: UVec2) -> f32 {
        let a = self.data[self.coordinates_to_offset(a)];
        let b = self.data[self.coordinates_to_offset(b)];
        let m = self.data[self.coordinates_to_offset(middle)];

        let interpolated = b as f32 / 2.0 + a as f32 / 2.0;

        (interpolated - m as f32).abs() * self.height_scaling
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl MinHeight {
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn to_f32(self) -> f32 {
        self.0 as f32
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl MaxHeight {
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn to_f32(self) -> f32 {
        self.0 as f32
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<'heightmap> TerrainHeightMapView<'heightmap> {
    // ------------------------------------------------------------------------
    pub(crate) fn tiles_min_max_y_strip(
        &self,
    ) -> Vec<(TerrainTileId<TILE_SIZE>, MinHeight, MaxHeight)> {
        let tile_size = self.start_tile.tile_size();
        let tiles_per_edge = (self.heightmap.size / tile_size) as usize;
        let offset_start = (self.start_tile.sampling_offset().y * self.heightmap.size) as usize;
        let offset_end = offset_start + (self.heightmap.size * tile_size) as usize;

        assert!(self.heightmap.data.len() >= offset_end);

        // println!("MINMAX_STRIP: {:?} {}", self.tileid, offset_start);
        let tiles_reduced_x = self.heightmap.data[offset_start..offset_end]
            .chunks_exact(tile_size as usize)
            .map(|slice| {
                slice
                    .iter()
                    .cloned()
                    .map(|h| (h, h))
                    .reduce(|(last_min, last_max), (next_min, next_max)| {
                        (last_min.min(next_min), last_max.max(next_max))
                    })
                    .unwrap()
            })
            .collect::<Vec<_>>();

        let init_value = (u16::MAX, u16::MIN);
        let mut result = vec![init_value; tiles_per_edge];

        for reduced_x_collection in tiles_reduced_x.chunks_exact(tiles_per_edge) {
            result = reduced_x_collection
                .iter()
                .zip(result.iter())
                .map(|((a_min, a_max), (b_min, b_max))| (*a_min.min(b_min), *a_max.max(b_max)))
                .collect::<Vec<_>>();
        }

        result
            .iter()
            .enumerate()
            .map(|(x, (min, max))| {
                (
                    TerrainTileId::new(x as u8, self.start_tile.y()),
                    MinHeight(*min),
                    MaxHeight(*max),
                )
            })
            .collect::<Vec<_>>()
    }
    // ------------------------------------------------------------------------
    //TODO check if this redundant arc ref allocation can be fixed
    #[allow(clippy::redundant_allocation)]
    pub(crate) fn new_strip(
        start: TerrainTileId<TILE_SIZE>,
        heightmap: Arc<&'heightmap TerrainHeightMap>,
    ) -> Self {
        Self {
            start_tile: start,
            heightmap,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<'heightmap, 'normals> TerrainDataView<'heightmap, 'normals> {
    // ------------------------------------------------------------------------
    //TODO check if this redundant arc ref allocation can be fixed
    #[allow(clippy::redundant_allocation)]
    pub fn new(
        offset: UVec2,
        heightmap: Arc<&'heightmap TerrainHeightMap>,
        normals: Arc<&'normals TerrainNormals>,
    ) -> Self {
        Self {
            offset,
            heightmap,
            normals,
        }
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn sample_interpolated_height_error(&self, a: UVec2, b: UVec2, middle: UVec2) -> f32 {
        self.heightmap.sample_interpolated_error(
            self.offset + a,
            self.offset + b,
            self.offset + middle,
        )
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn sample_height_and_normal(&self, pos: UVec2) -> (f32, [f32; 3]) {
        // Note: heightmap and normals have the same size!
        let offset = self.heightmap.coordinates_to_offset(pos);
        let height = self.heightmap.data[offset];
        let normal = self.normals.data[offset];

        (height as f32 * self.heightmap.height_scaling, normal)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<const TILE_SIZE: u32> TerrainTileId<TILE_SIZE> {
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn new(x: u8, y: u8) -> Self {
        Self(x, y)
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    #[inline(always)]
    pub fn x(&self) -> u8 {
        self.0
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    #[inline(always)]
    pub fn y(&self) -> u8 {
        self.1
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn sampling_offset(&self) -> UVec2 {
        uvec2(self.0 as u32 * TILE_SIZE, self.1 as u32 * TILE_SIZE)
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn half_extent(&self) -> UVec2 {
        uvec2(TILE_SIZE / 2, TILE_SIZE / 2)
    }
    // ------------------------------------------------------------------------
    const fn tile_size(&self) -> u32 {
        TILE_SIZE
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
