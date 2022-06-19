// ----------------------------------------------------------------------------
/// defines the max time for blocking errormap/mesh generation until remaining
/// work is deferred to next frame. prevents blocking of complete app.
const MAX_MESH_GENERATION_TIME_MS: instant::Duration = instant::Duration::from_millis(5);
const MAX_ERRORMAP_GENERATION_TIME_MS: instant::Duration = instant::Duration::from_millis(35);
// ----------------------------------------------------------------------------
/// defines how many tiles are processed in parallel before a check for max
/// generation time is made
const MESH_GENERATION_QUEUE_CHUNKSIZE: usize = 10;
// ----------------------------------------------------------------------------
use std::ops::Deref;
use std::sync::Arc;

use bevy::{
    ecs::schedule::StateData,
    math::{vec3, vec3a, Vec2, Vec3, Vec3Swizzles},
    prelude::*,
    render::primitives::Aabb,
    tasks::{AsyncComputeTaskPool, ComputeTaskPool, TaskPool},
};

use crate::cmds::{AsyncTaskFinishedEvent, AsyncTaskStartEvent, TrackedProgress};
use crate::config::{TerrainConfig, TILE_SIZE};
use crate::heightmap::{
    TerrainDataView, TerrainHeightMap, TerrainHeightMapView, TerrainNormals, TerrainTileId,
};
use crate::terrain_clipmap::ClipmapAssignment;
use crate::terrain_render::{
    TerrainMesh, TerrainMeshStats, TerrainMeshVertexData, TerrainRenderSettings,
};
use crate::EditorEvent;

use TerrainTileSystemLabel::*;

use self::errormap::{ErrorMapsPostprocessing, TileHeightErrors};
use self::lod::MeshLodTracker;
use self::settings::TerrainLodSettings;
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
    ErrorMapSeamProcessing,
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
            .with_system(
                async_errormap_seam_processing
                    .label(ErrorMapSeamProcessing)
                    .after(ErrorMapGeneration),
            )
            .with_system(async_tilemesh_generation.label(MeshGeneration))
            .with_system(lod::adjust_tile_mesh_lod.before(MeshGeneration))
            .with_system(lod::adjust_meshes_on_config_change.before(MeshGeneration))
            .with_system(collect_stats.after(MeshGeneration))
            .with_system(update_mesh_index_bound.after(MeshGeneration))
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
        app.init_resource::<TerrainMeshSettings>()
            .init_resource::<TerrainStats>()
            .init_resource::<MeshLodTracker>()
            .init_resource::<ErrorMapsPostprocessing>()
            .init_resource::<errormap::TileTriangleLookup>();
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct TerrainStats {
    pub tiles: u16,
    pub vertices: usize,
    pub triangles: usize,
    pub data_bytes: usize,
    pub last_update_tiles: u16,
    pub last_update_vertices: usize,
    pub last_update_triangles: usize,
    pub last_update_data_bytes: usize,
    pending_updates: bool,
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
impl TerrainTileComponent {
    // ------------------------------------------------------------------------
    pub fn assigned_lod(&self) -> u8 {
        self.mesh_conf.lod
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(Clone, Copy)]
struct IndexBound(f32);
// ----------------------------------------------------------------------------
#[derive(Clone)]
struct MeshReduction {
    lod: u8,
    current: f32,
    target: f32,
    special_case: bool,
    special_case_corner: bool,
    target_top: f32,
    target_bottom: f32,
    target_left: f32,
    target_right: f32,

    target_corner_tl: f32,
    target_corner_tr: f32,
    target_corner_bl: f32,
    target_corner_br: f32,

    priority: u32,

    idx_bound: IndexBound,
    idx_bound_wireframe: IndexBound,
}
// ----------------------------------------------------------------------------
mod errormap;
mod generator;
mod lod;
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

        let center = vec3a(0.0, min_height + half_height, 0.0);
        let half_extents = vec3a(half_size, half_height, half_size);

        Aabb {
            center,
            half_extents,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
/// Marks a tile to be usable for mesh lod assignment based on (some) distance
/// measure to TerrainLodAnchor. In general all mesh tiles should have adaptive
/// lods. But if errormaps are (re)calculated any lod changes have to be stopped.
#[derive(Component)]
struct AdaptiveTileMeshLods;
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
fn update_mesh_index_bound(
    mut query: Query<(&mut TerrainTileComponent, &Handle<TerrainMesh>)>,
    meshes: Res<Assets<TerrainMesh>>,
    render_settings: Res<TerrainRenderSettings>,
) {
    // meshes are static but number of generated triangles depends on used error
    // threshold. for some error threshold the vertex count will drop below
    // u16::MAX and allows to use smaller u16 indices for *ALL* bigger error
    // thresholds.
    //
    // update the u16 ibound based on last error threshold and vertex number
    if meshes.is_changed() {
        let with_wireframe = render_settings.overlay_wireframe;

        for (mut tile, mesh_handle) in query.iter_mut() {
            let current = tile.mesh_conf.current;
            let bound = if with_wireframe {
                &mut tile.mesh_conf.idx_bound_wireframe
            } else {
                &mut tile.mesh_conf.idx_bound
            };
            if bound.needs_update(current) {
                if let Some(vertex_count) = meshes
                    .get(mesh_handle)
                    .filter(|m| m.pending_upload())
                    .map(|m| m.stats().vertices)
                {
                    bound.update(current, vertex_count);
                }
            }
        }
    }
}
// ----------------------------------------------------------------------------
fn collect_stats(mut stats: ResMut<TerrainStats>, meshes: Res<Assets<TerrainMesh>>) {
    if meshes.is_changed() {
        let (summed, pending, count, count_pending) = meshes.iter().fold(
            (
                TerrainMeshStats::default(),
                TerrainMeshStats::default(),
                0,
                0,
            ),
            |accum, (_, m)| {
                if m.pending_upload() {
                    (
                        &accum.0 + m.stats(),
                        &accum.1 + m.stats(),
                        accum.2 + 1,
                        accum.3 + 1,
                    )
                } else {
                    (&accum.0 + m.stats(), accum.1, accum.2 + 1, accum.3)
                }
            },
        );

        stats.tiles = count;
        stats.vertices = summed.vertices as usize;
        stats.triangles = summed.triangles as usize;
        stats.data_bytes = summed.data_bytes as usize;

        if count_pending > 0 {
            // Note: this is not accurate. if generating of tiles is too slow
            // it will not catch up while camera is moving and data will grow
            // indefinitely until generation stops
            if !stats.pending_updates {
                // reset accumulated data as new updates are arriving
                stats.last_update_tiles = 0;
                stats.last_update_vertices = 0;
                stats.last_update_triangles = 0;
                stats.last_update_data_bytes = 0;
                stats.pending_updates = true;
            }
            // accumulate all subsequent updates until it's finished
            stats.last_update_tiles += count_pending;
            stats.last_update_vertices += pending.vertices as usize;
            stats.last_update_triangles += pending.triangles as usize;
            stats.last_update_data_bytes += pending.data_bytes as usize;
        } else {
            stats.pending_updates = false;
        }
    }
}
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn start_async_terraintile_tasks(
    mut commands: Commands,
    terrain_config: Res<TerrainConfig>,
    heightmap: Res<TerrainHeightMap>,
    thread_pool: Res<ComputeTaskPool>,

    tiles: Query<Entity, With<TerrainTileComponent>>,
    mut errormaps_postprocessing: ResMut<ErrorMapsPostprocessing>,

    mut tasks_queued: EventReader<AsyncTaskStartEvent>,
    mut task_finished: EventWriter<AsyncTaskFinishedEvent>,
) {
    use AsyncTaskStartEvent::*;

    for task in tasks_queued.iter() {
        match task {
            GenerateTerrainTiles => {
                debug!("generating terrain tiles...");
                commands.spawn_batch(generate_tiles(&*terrain_config, &*heightmap, &*thread_pool));
                commands.insert_resource(MeshLodTracker::new(&*terrain_config));
                task_finished.send(AsyncTaskFinishedEvent::TerrainTilesGenerated);
            }
            GenerateTerrainMeshErrorMaps => {
                debug!("generating error maps...");
                tiles.iter().for_each(|entity| {
                    commands
                        .entity(entity)
                        .insert(TileHeightErrorGenerationQueued);
                });
                *errormaps_postprocessing = ErrorMapsPostprocessing::new(
                    terrain_config.map_size(),
                    terrain_config.tile_count(),
                );
            }
            MergeTerrainMeshErrorMapSeams => {
                debug!("merging error map seams...");
                errormaps_postprocessing.start();
            }
            GenerateTerrainMeshes => {
                debug!("generating tile meshes...");
                tiles.iter().for_each(|entity| {
                    commands
                        .entity(entity)
                        .insert(TileMeshGenerationQueued)
                        // from this point on the lod for the meshes may be
                        // changed by a dedicated system
                        .insert(AdaptiveTileMeshLods);
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
    mut triangle_table: ResMut<errormap::TileTriangleLookup>,
    mut seamprocessing_queue: ResMut<ErrorMapsPostprocessing>,
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

        // divide into packets that are processed completely until processing time
        // is evaluated again...
        let queue = &mut tiles_to_process.chunks(MESH_GENERATION_QUEUE_CHUNKSIZE);

        // since all tiles are triangulated at the same local coordinates
        // (within the tile!) all possible triangle coordinates can be
        // precalculated once and shared for the errormap generation of all
        // tiles. speeds up generation significantly.
        if triangle_table.is_empty() {
            triangle_table.generate();
        }

        // sharable reference for scoped threads
        let triangles = &triangle_table;

        // ...measure duration after every packet
        while Instant::now().duration_since(start_time) < MAX_ERRORMAP_GENERATION_TIME_MS {
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
                                (
                                    *entity,
                                    *tileid,
                                    errormap::generate_errormap(triangles, &terraindata_view),
                                )
                            })
                        }
                    });
                    for (entity, tileid, tile_errors) in generated_errormaps.drain(..) {
                        remaining_tiles -= 1;
                        commands
                            .entity(entity)
                            .remove::<TileHeightErrorGenerationQueued>();

                        seamprocessing_queue.add_errormap(entity, tileid, tile_errors);
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
fn async_errormap_seam_processing(
    mut commands: Commands,
    mut errormaps_postprocessing: ResMut<ErrorMapsPostprocessing>,
    thread_pool: Res<ComputeTaskPool>,
    mut task_finished: EventWriter<AsyncTaskFinishedEvent>,
    mut editor_events: EventWriter<EditorEvent>,
    mut triangle_table: ResMut<errormap::TileTriangleLookup>,
) {
    if errormaps_postprocessing.processing_required() {
        use instant::Instant;

        let start_time = Instant::now();

        // sharable reference for scoped threads
        let triangles = &triangle_table;

        // ...measure duration after every packet
        while Instant::now().duration_since(start_time) < MAX_ERRORMAP_GENERATION_TIME_MS
            && !errormaps_postprocessing.is_queue_empty()
        {
            let mut generated_errormaps = thread_pool.scope(|s| {
                let mut max = MESH_GENERATION_QUEUE_CHUNKSIZE;
                while let Some((entity, tileid, mut errormap)) =
                    errormaps_postprocessing.next_package()
                {
                    s.spawn(async move {
                        errormap::update_errormap(triangles, &mut errormap);
                        (entity, tileid, errormap)
                    });

                    max -= 1;
                    if max == 0 {
                        break;
                    }
                }
            });
            // collect results
            errormaps_postprocessing.append_results(&mut generated_errormaps);
        }

        // progress update for GUI
        let (remaining_tiles, max_tiles) = errormaps_postprocessing.progress_info();
        editor_events.send(EditorEvent::ProgressTrackingUpdate(
            TrackedProgress::MergedTerrainErrorMapSeams(remaining_tiles, max_tiles),
        ));

        if errormaps_postprocessing.is_queue_empty() {
            errormaps_postprocessing.finalize_pass();

            if !errormaps_postprocessing.processing_required() {
                // done -> insert errormaps into terrain tiles
                for (entity, _, tile_errors) in errormaps_postprocessing.drain_results() {
                    commands.entity(entity).insert(tile_errors);
                }
                // free resources that are not needed anymore
                errormaps_postprocessing.free_resources();
                triangle_table.clear();

                // notify async task processort
                task_finished.send(AsyncTaskFinishedEvent::TerrainMeshErrorMapsSeamsMerged);
            }
        }
    }
}
// ----------------------------------------------------------------------------
#[derive(Component)]
/// marker for tiles which require regeneration of meshes
struct TileMeshGenerationQueued;
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn async_tilemesh_generation(
    mut commands: Commands,
    terrain_config: Res<TerrainConfig>,
    render_settings: Res<TerrainRenderSettings>,
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

        let include_wireframe_info = render_settings.overlay_wireframe;
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
                                    &tile.mesh_conf,
                                    terraindata_view,
                                    triangle_errors,
                                    include_wireframe_info,
                                    tile.mesh_conf.use_small_index(include_wireframe_info),
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
                            let m = meshes.get_mut(*handle).expect("existing tile mesh handle");
                            *m = new_mesh;
                        } else {
                            // tile has no mesh -> add generated mesh and assign handle to tile
                            e.insert(meshes.add(new_mesh));
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
    ClipmapAssignment,
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
                ClipmapAssignment::new(
                    terrain_config.max_clipmap_level(), // assign max level as default (covers complete map)
                    tile_center.xz(),
                    Vec2::ONE * TILE_SIZE as f32 * map_resolution,
                ),
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
fn despawn_tiles(mut commands: Commands, tiles: Query<Entity, With<TerrainTileComponent>>) {
    for tile in tiles.iter() {
        commands.entity(tile).despawn();
    }
    commands.insert_resource(TerrainStats::default());
    commands.insert_resource(MeshLodTracker::default());
}
// ----------------------------------------------------------------------------
impl MeshReduction {
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn use_small_index(&self, with_wireframe: bool) -> bool {
        if with_wireframe {
            self.idx_bound_wireframe.greater_or_equal(self.target)
        } else {
            self.idx_bound.greater_or_equal(self.target)
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl IndexBound {
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn needs_update(&self, error_threshold: f32) -> bool {
        // vertex count grows with error threshold monotonically. thus the
        // stored boundary for threshold can only be lowered.
        error_threshold < self.0
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn update(&mut self, error_threshold: f32, vertex_count: u32) {
        if vertex_count < u16::MAX as u32 {
            self.0 = error_threshold
        }
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn greater_or_equal(&self, error_threshold: f32) -> bool {
        error_threshold >= self.0
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Default for MeshReduction {
    fn default() -> Self {
        Self {
            lod: 255,
            current: f32::MAX,
            // make target error threshold "high" by default so new terrain is
            // showed quicker and only near tiles are "upgraded"
            target: 2.0,
            special_case: false,
            special_case_corner: false,

            target_top: 2.0,
            target_bottom: 2.0,
            target_left: 2.0,
            target_right: 2.0,

            target_corner_tl: 2.0,
            target_corner_tr: 2.0,
            target_corner_bl: 2.0,
            target_corner_br: 2.0,

            priority: 0,
            idx_bound: IndexBound::default(),
            idx_bound_wireframe: IndexBound::default(),
        }
    }
}
// ----------------------------------------------------------------------------
impl Default for IndexBound {
    fn default() -> Self {
        Self(f32::MAX)
    }
}
// ----------------------------------------------------------------------------
