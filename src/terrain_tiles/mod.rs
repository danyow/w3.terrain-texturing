// ----------------------------------------------------------------------------
/// defines the max time for blocking errormap/mesh generation until remaining
/// work is deferred to next frame. prevents blocking of complete app.
const MAX_MESH_GENERATION_TIME_MS: instant::Duration = instant::Duration::from_millis(30);
// ----------------------------------------------------------------------------
/// defines how many tiles are processed in parallel before a check for max
/// generation time is made
const MESH_GENERATION_QUEUE_CHUNKSIZE: usize = 10;
// ----------------------------------------------------------------------------
use std::ops::Deref;
use std::sync::Arc;

use bevy::{
    ecs::schedule::StateData,
    math::{vec3, Vec2, Vec3, Vec3Swizzles},
    prelude::*,
    render::primitives::Aabb,
    tasks::{AsyncComputeTaskPool, ComputeTaskPool, TaskPool},
};

use crate::cmds::{AsyncTaskFinishedEvent, AsyncTaskStartEvent, TrackedProgress};
use crate::config::{TerrainConfig, TILE_SIZE};
use crate::heightmap::{
    TerrainDataView, TerrainHeightMap, TerrainHeightMapView, TerrainNormals, TerrainTileId,
};
use crate::EditorEvent;

use TerrainTileSystemLabel::*;

use self::generator::TileHeightErrors;
// ----------------------------------------------------------------------------
pub struct TerrainTilesGeneratorPlugin;

pub use self::settings::{LodSlot, TerrainMeshSettings};
// ----------------------------------------------------------------------------
#[derive(Component)]
pub struct TerrainLodAnchor;
// ----------------------------------------------------------------------------
#[derive(Debug, Clone, Hash, Eq, PartialEq, SystemLabel)]
pub enum TerrainTileSystemLabel {
    ErrorMapGeneration,
    MeshGeneration,
}
// ----------------------------------------------------------------------------
impl TerrainTilesGeneratorPlugin {
    // ------------------------------------------------------------------------
    /// async (re)generation of terrain tiles, errormaps, meshes based on lod_anchor
    /// position changes
    pub fn lazy_generation<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_update(state)
            .with_system(start_async_terraintile_tasks)
            .with_system(async_errormap_generation.label(ErrorMapGeneration))
            .with_system(async_tilemesh_generation.label(MeshGeneration))
            .with_system(adjust_tile_mesh_lod.before(MeshGeneration))
            .with_system(adjust_meshes_on_config_change.before(MeshGeneration))
    }
    // ------------------------------------------------------------------------
    pub fn reset_data<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_enter(state).with_system(despawn_tiles)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Plugin for TerrainTilesGeneratorPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainMeshSettings>();
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(Component, Clone)]
pub struct TerrainTileComponent {
    id: TerrainTileId<TILE_SIZE>,
    min_height: f32,
    max_height: f32,
    mesh_conf: MeshReduction,
    /// tile center in world coordinates (with map resolution applied)
    pos_center: Vec3,
}
// ----------------------------------------------------------------------------
#[derive(Clone)]
struct MeshReduction {
    current: f32,
    target: f32,
    priority: u32,
}
// ----------------------------------------------------------------------------
mod generator;
mod settings;
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
            mesh_conf: MeshReduction::default(),
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
#[allow(clippy::too_many_arguments)]
fn start_async_terraintile_tasks(
    mut commands: Commands,
    terrain_config: Res<TerrainConfig>,
    heightmap: Res<TerrainHeightMap>,
    thread_pool: Res<ComputeTaskPool>,

    tiles: Query<Entity, With<TerrainTileComponent>>,

    mut tasks_queued: EventReader<AsyncTaskStartEvent>,
    mut task_finished: EventWriter<AsyncTaskFinishedEvent>,
) {
    use AsyncTaskStartEvent::*;

    for task in tasks_queued.iter() {
        match task {
            GenerateTerrainTiles => {
                debug!("generating terrain tiles...");
                commands.spawn_batch(generate_tiles(&*terrain_config, &*heightmap, &*thread_pool));
                task_finished.send(AsyncTaskFinishedEvent::TerrainTilesGenerated);
            }
            GenerateTerrainMeshErrorMaps => {
                debug!("generating error maps...");
                tiles.iter().for_each(|entity| {
                    commands
                        .entity(entity)
                        .insert(TileHeightErrorGenerationQueued);
                });
            }
            GenerateTerrainMeshes => {
                debug!("generating tile meshes...");
                tiles.iter().for_each(|entity| {
                    commands.entity(entity).insert(TileMeshGenerationQueued);
                });
            }
            _ => {}
        }
    }
}
// ----------------------------------------------------------------------------
#[derive(Component)]
/// marker for tiles which require regeneration of errormap
struct TileHeightErrorGenerationQueued;
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn async_errormap_generation(
    mut commands: Commands,
    terrain_config: Res<TerrainConfig>,
    tiles: Query<(Entity, &TerrainTileComponent), With<TileHeightErrorGenerationQueued>>,
    heightmap: Res<TerrainHeightMap>,
    normals: Res<TerrainNormals>,
    thread_pool: Res<ComputeTaskPool>,
    mut task_finished: EventWriter<AsyncTaskFinishedEvent>,
    mut editor_events: EventWriter<EditorEvent>,
) {
    if !tiles.is_empty() {
        use instant::Instant;

        let heightmap = Arc::new(heightmap.deref());
        let normals = Arc::new(normals.deref());
        let start_time = Instant::now();

        // remap tiles to cloned data
        let tiles_to_process = tiles
            .iter()
            .map(|(entity, tile)| (entity, tile.id))
            .collect::<Vec<_>>();
        let mut remaining_tiles = tiles_to_process.len();

        // divide into packets that are parallelized...
        let queue = &mut tiles_to_process.chunks(MESH_GENERATION_QUEUE_CHUNKSIZE);

        // ...measure duration after every packet
        while Instant::now().duration_since(start_time) < MAX_MESH_GENERATION_TIME_MS {
            match queue.next() {
                Some(package) => {
                    let mut generated_errormaps = thread_pool.scope(|s| {
                        for (entity, tileid) in package {
                            let terraindata_view = TerrainDataView::new(
                                tileid.sampling_offset(),
                                heightmap.clone(),
                                normals.clone(),
                            );

                            s.spawn(async move {
                                (*entity, generator::generate_errormap(&terraindata_view))
                            })
                        }
                    });
                    for (entity, tile_errors) in generated_errormaps.drain(..) {
                        remaining_tiles -= 1;
                        commands
                            .entity(entity)
                            .insert(tile_errors)
                            .remove::<TileHeightErrorGenerationQueued>();
                    }
                }
                None => break,
            }
        }
        // progress update for GUI
        let max_tiles = terrain_config.tile_count();
        editor_events.send(EditorEvent::ProgressTrackingUpdate(
            TrackedProgress::GeneratedTerrainErrorMaps(
                max_tiles.saturating_sub(remaining_tiles),
                max_tiles,
            ),
        ));

        if queue.next().is_none() {
            task_finished.send(AsyncTaskFinishedEvent::TerrainMeshErrorMapsGenerated);
        }
    }
}
// ----------------------------------------------------------------------------
#[derive(Component)]
/// marker for tiles which require regeneration of meshes
struct TileMeshGenerationQueued;
// ----------------------------------------------------------------------------
//TODO make proper specialized mesh type so updates just take data instead clone (?)
type TerrainMesh = Mesh;
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn async_tilemesh_generation(
    mut commands: Commands,
    terrain_config: Res<TerrainConfig>,
    heightmap: Res<TerrainHeightMap>,
    normals: Res<TerrainNormals>,
    mut meshes: ResMut<Assets<TerrainMesh>>,
    tiles: Query<
        (
            Entity,
            &TerrainTileComponent,
            &TileHeightErrors,
            Option<&Handle<TerrainMesh>>,
        ),
        With<TileMeshGenerationQueued>,
    >,
    thread_pool: Res<AsyncComputeTaskPool>,
    mut task_finished: EventWriter<AsyncTaskFinishedEvent>,
    mut editor_events: EventWriter<EditorEvent>,
) {
    if !tiles.is_empty() {
        use instant::Instant;

        let heightmap = Arc::new(heightmap.deref());
        let normals = Arc::new(normals.deref());
        let terrain_config = &terrain_config;
        let start_time = Instant::now();

        // remap tiles to cloned data
        let mut tiles_to_process = tiles.iter().collect::<Vec<_>>();
        let mut remaining_tiles = tiles_to_process.len();

        // prioritize tiles which are closer to viewer (and visible) based on
        // priority
        tiles_to_process.sort_unstable_by_key(|(_, tile, _, _)| tile.mesh_conf.priority);

        // divide into packets that are parallelized...
        let queue = &mut tiles_to_process.chunks(MESH_GENERATION_QUEUE_CHUNKSIZE);

        // ...measure duration after every packet
        while Instant::now().duration_since(start_time) < MAX_MESH_GENERATION_TIME_MS {
            match queue.next() {
                Some(package) => {
                    let mut generated_meshes = thread_pool.scope(|s| {
                        for (_, tile, triangle_errors, _) in package {
                            let terraindata_view = TerrainDataView::new(
                                tile.id.sampling_offset(),
                                heightmap.clone(),
                                normals.clone(),
                            );

                            s.spawn(async move {
                                generator::generate_tilemesh(
                                    tile.id,
                                    terrain_config.resolution(),
                                    terrain_config.min_height(),
                                    tile.mesh_conf.target,
                                    terraindata_view,
                                    triangle_errors,
                                )
                            })
                        }
                    });
                    // attach generated meshes to tile entities and remove marker component
                    for ((entity, _, _, mesh_handle), new_mesh) in
                        package.iter().zip(generated_meshes.drain(..))
                    {
                        remaining_tiles -= 1;
                        let mut e = commands.entity(*entity);
                        e.remove::<TileMeshGenerationQueued>();

                        if let Some(handle) = mesh_handle {
                            // mesh is an update
                            meshes
                                .get_mut(*handle)
                                .expect("existing tile mesh handle")
                                .clone_from(&new_mesh.mesh());
                        } else {
                            // tile has no mesh -> add generated mesh and assign handle to tile
                            e.insert(meshes.add(new_mesh.mesh()));
                        }
                    }
                }
                None => break,
            }
        }
        // progress update for GUI
        let max_tiles = terrain_config.tile_count();
        editor_events.send(EditorEvent::ProgressTrackingUpdate(
            TrackedProgress::GeneratedTerrainMeshes(
                max_tiles.saturating_sub(remaining_tiles),
                max_tiles,
            ),
        ));

        if queue.next().is_none() {
            task_finished.send(AsyncTaskFinishedEvent::TerrainMeshesGenerated);
        }
    }
}
// ----------------------------------------------------------------------------
type TerrainTileBundle = (
    TerrainTileComponent,
    GlobalTransform,
    Transform,
    Aabb,
    Visibility,
    ComputedVisibility,
);
// ----------------------------------------------------------------------------
fn generate_tiles(
    terrain_config: &TerrainConfig,
    heightmap: &TerrainHeightMap,
    thread_pool: &TaskPool,
) -> Vec<TerrainTileBundle> {
    // generates all necessary tiles
    let tiles = terrain_config.tiles_per_edge();

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

    tile_elevation
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
        .collect::<Vec<_>>()
}
// ----------------------------------------------------------------------------
fn update_tilemesh_lods(
    mut commands: Commands,
    settings: Res<TerrainMeshSettings>,
    lod_anchor: &Transform,
    mut query: Query<(Entity, &ComputedVisibility, &mut TerrainTileComponent)>,
) {
    for (entity, vis, mut tile) in query.iter_mut() {
        // FIXME either 2d or 3d distance to tilecenter
        let distance = tile.pos_center.xz().distance(lod_anchor.translation.xz());
        // let distance = tile.pos_center.distance(lod_anchor.translation);
        let settings = settings.lod_settings_from_distance(distance);

        tile.mesh_conf.target = settings.threshold;

        if tile.mesh_conf.target != tile.mesh_conf.current {
            commands.entity(entity).insert(TileMeshGenerationQueued);

            // adjust priority based on distance from lod_anchor and visibility
            tile.mesh_conf.priority = if vis.is_visible {
                distance as u32
            } else {
                // adding big num will push priority after all visibles
                // asummption: distance >= 1_000_000 are not used
                distance as u32 + 1_000_000
            };
            tile.mesh_conf.current = tile.mesh_conf.target;
        }
    }
}
// ----------------------------------------------------------------------------
fn adjust_meshes_on_config_change(
    commands: Commands,
    settings: Res<TerrainMeshSettings>,
    lod_anchor: Query<&Transform, With<TerrainLodAnchor>>,
    query: Query<(Entity, &ComputedVisibility, &mut TerrainTileComponent)>,
) {
    if settings.is_changed() {
        if let Ok(lod_anchor) = lod_anchor.get_single() {
            update_tilemesh_lods(commands, settings, lod_anchor, query);
        }
    }
}
// ----------------------------------------------------------------------------
fn adjust_tile_mesh_lod(
    commands: Commands,
    settings: Res<TerrainMeshSettings>,
    lod_anchor: Query<&Transform, With<TerrainLodAnchor>>,
    query: Query<(Entity, &ComputedVisibility, &mut TerrainTileComponent)>,
) {
    if !settings.ignore_anchor {
        // TODO add hysteresis for current anchor pos
        if let Ok(lod_anchor) = lod_anchor.get_single() {
            update_tilemesh_lods(commands, settings, lod_anchor, query);
        }
    }
}
// ----------------------------------------------------------------------------
fn despawn_tiles(mut commands: Commands, tiles: Query<Entity, With<TerrainTileComponent>>) {
    for tile in tiles.iter() {
        commands.entity(tile).despawn();
    }
}
// ----------------------------------------------------------------------------
impl Default for MeshReduction {
    fn default() -> Self {
        Self {
            current: f32::MAX,
            // make target error threshold "high" by default so new terrain is
            // showed quicker and only near tiles are "upgraded"
            target: 2.0,
            priority: 0,
        }
    }
}
// ----------------------------------------------------------------------------
