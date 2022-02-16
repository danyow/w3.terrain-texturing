// ----------------------------------------------------------------------------
use std::ops::Deref;
use std::sync::Arc;

use bevy::{
    ecs::schedule::StateData,
    math::{vec3, Vec2, Vec3},
    prelude::*,
    render::primitives::Aabb,
    tasks::ComputeTaskPool,
};

use crate::{
    cmds::{AsyncTaskFinishedEvent, AsyncTaskStartEvent},
    config::{TerrainConfig, TILE_SIZE},
    heightmap::{TerrainHeightMap, TerrainHeightMapView, TerrainTileId},
};
// ----------------------------------------------------------------------------
pub struct TerrainTilesGeneratorPlugin;
// ----------------------------------------------------------------------------
impl TerrainTilesGeneratorPlugin {
    // ------------------------------------------------------------------------
    /// async (re)generation of terrain tiles, errormaps, meshes based on camera
    /// position changes
    pub fn lazy_generation<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_update(state).with_system(start_async_terraintile_tasks)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Plugin for TerrainTilesGeneratorPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, _app: &mut App) {}
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(Component, Clone)]
pub struct TerrainTileComponent {
    id: TerrainTileId<TILE_SIZE>,
    min_height: f32,
    max_height: f32,
    /// tile center in world coordinates (with map resolution applied)
    pos_center: Vec3,
}
// ----------------------------------------------------------------------------
impl TerrainTileComponent {
    // ------------------------------------------------------------------------
    fn new(
        id: TerrainTileId<TILE_SIZE>,
        min_height: f32,
        max_height: f32,
        terrain_resolution: f32,
        terrain_centering_offset: Vec2,
    ) -> Self {
        let center = terrain_centering_offset
            + (id.sampling_offset() + id.half_extent()).as_vec2() * terrain_resolution;

        Self {
            id,
            min_height,
            max_height,
            // Note: remap 2d coordinates to 3d -> y becomes z!
            pos_center: vec3(center.x, 0.0, center.y),
        }
    }
    // ------------------------------------------------------------------------
    fn compute_aabb(
        &self,
        height_offset: f32,
        height_scaling: f32,
        terrain_resolution: f32,
    ) -> Aabb {
        let min_height = height_offset + height_scaling * self.min_height;
        let max_height = height_offset + height_scaling * self.max_height;

        let half_height = 0.5 * (max_height - min_height);
        let half_size = (TILE_SIZE / 2) as f32 * terrain_resolution;

        let center = vec3(0.0, min_height + half_height, 0.0);
        let half_extents = vec3(half_size, half_height, half_size);

        Aabb {
            center,
            half_extents,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
fn start_async_terraintile_tasks(
    mut commands: Commands,
    terrain_config: Res<TerrainConfig>,
    heightmap: Res<TerrainHeightMap>,
    thread_pool: Res<ComputeTaskPool>,

    mut tasks_queued: EventReader<AsyncTaskStartEvent>,
    mut task_finished: EventWriter<AsyncTaskFinishedEvent>,
) {
    use AsyncTaskStartEvent::*;

    if tasks_queued
        .iter()
        .any(|t| matches!(t, GenerateTerrainTiles))
    {
        // generates all necessary tiles
        let tiles = terrain_config.tiles_per_edge();
        debug!("generating terrain tiles...");

        // unfortunately heightmap doesn't have static lifetime and cannot be provided to
        // an async task/thread pool -> scoped, blocking threadpool

        // extract min max heights per tile to precaclulate bounding boxes
        let hm = Arc::new(heightmap.deref());
        let mut tile_elevation = thread_pool.scope(|s| {
            for y in 0..tiles {
                let heightmap_strip =
                    TerrainHeightMapView::new_strip(TerrainTileId::new(0, y), hm.clone());

                s.spawn(async move { heightmap_strip.tiles_min_max_y_strip() });
            }
        });

        // generate new TerrainTileInfos
        let map_resolution = terrain_config.resolution();
        let map_offset = terrain_config.map_offset();
        let height_scaling = terrain_config.height_scaling();
        let height_offset = terrain_config.min_height();

        let tiles = tile_elevation
            .drain(..)
            .flatten()
            .map(|(tile_id, min_height, max_height)| {
                let tile_info = TerrainTileComponent::new(
                    tile_id,
                    min_height.to_f32(),
                    max_height.to_f32(),
                    map_resolution,
                    // offset in absolute world coordinates to put center of terrain at origin
                    map_offset,
                );

                let tile_center = tile_info.pos_center;
                let aabb = tile_info.compute_aabb(height_offset, height_scaling, map_resolution);

                // default component bundle for terrain tile
                (
                    tile_info,
                    GlobalTransform::default(),
                    Transform::from_translation(tile_center),
                    aabb,
                    Visibility::default(),
                    ComputedVisibility::default(),
                )
            })
            .collect::<Vec<_>>();

        commands.spawn_batch(tiles);
        task_finished.send(AsyncTaskFinishedEvent::TerrainTilesGenerated);
    }
}
// ----------------------------------------------------------------------------
