// ----------------------------------------------------------------------------
use bevy::{
    math::{Vec2, Vec3Swizzles},
    prelude::*,
    utils::{HashMap, HashSet},
};

use super::{
    AdaptiveTileMeshLods, MeshReduction, TerrainConfig, TerrainLodAnchor, TerrainLodSettings,
    TerrainMeshSettings, TerrainTileComponent, TerrainTileId, TileMeshGenerationQueued, TILE_SIZE,
};

use super::generator::TileTriangle;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(super) struct MeshLodTracker {
    forced_update: bool,
    last_pos: Vec2,
    lods: HashMap<TerrainTileId<TILE_SIZE>, TrackedMeshErrorThresholds>,
    changed: HashSet<TerrainTileId<TILE_SIZE>>,
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
pub(super) fn adjust_meshes_on_config_change(
    commands: Commands,
    settings: Res<TerrainMeshSettings>,
    lod_anchor: Query<&Transform, With<TerrainLodAnchor>>,
    mut tracker: ResMut<MeshLodTracker>,
    query: Query<
        (Entity, &ComputedVisibility, &mut TerrainTileComponent),
        With<AdaptiveTileMeshLods>,
    >,
) {
    if settings.is_changed() {
        if let Ok(lod_anchor) = lod_anchor.get_single() {
            update_tilemesh_lods(commands, settings, lod_anchor, tracker.as_mut(), query);
        }
    }
}
// ----------------------------------------------------------------------------
pub(super) fn adjust_tile_mesh_lod(
    commands: Commands,
    settings: Res<TerrainMeshSettings>,
    lod_anchor: Query<&Transform, With<TerrainLodAnchor>>,
    mut tracker: ResMut<MeshLodTracker>,
    query: Query<
        (Entity, &ComputedVisibility, &mut TerrainTileComponent),
        With<AdaptiveTileMeshLods>,
    >,
) {
    if !settings.ignore_anchor {
        // TODO add hysteresis for current anchor pos
        if let Ok(lod_anchor) = lod_anchor.get_single() {
            if tracker.lazy_update(lod_anchor.translation.xz()) {
                update_tilemesh_lods(commands, settings, lod_anchor, tracker.as_mut(), query);
            }
        }
    }
}
// ----------------------------------------------------------------------------
fn update_tilemesh_lods(
    mut commands: Commands,
    settings: Res<TerrainMeshSettings>,
    lod_anchor: &Transform,
    tracker: &mut MeshLodTracker,
    mut query: Query<
        (Entity, &ComputedVisibility, &mut TerrainTileComponent),
        With<AdaptiveTileMeshLods>,
    >,
) {
    if tracker.lods.is_empty() {
        // initialization time
        for (_, _, tile) in query.iter() {
            tracker.lods.insert(
                tile.id,
                TrackedMeshErrorThresholds::new(tile.mesh_conf.current, tile.mesh_conf.lod),
            );
        }
    } else {
        tracker.changed.clear();
    }

    for (_, vis, tile) in query.iter() {
        // maximum metric
        let distance = (tile.pos_center.xz() - lod_anchor.translation.xz())
            .abs()
            .length();
        // .max_element();

        let settings = settings.lod_settings_from_distance(distance);

        // IF updated use priority based on distance from lod_anchor and visibility
        let priority = if vis.is_visible {
            distance as u32
        } else {
            // adding big num will push priority after all visibles
            // asummption: distance >= 1_000_000 are not used
            distance as u32 + 1_000_000
        };

        tracker.propagate_updates(tile.id, priority, settings);
    }

    tracker.patch_seams();

    for (entity, _, mut tile) in query
        .iter_mut()
        .filter(|(_, _, tile)| tracker.changed.contains(&tile.id))
    {
        let lod_info = tracker.lods.get_mut(&tile.id).unwrap();

        if lod_info.changed() {
            tile.mesh_conf.priority = lod_info.priority;
            tile.mesh_conf.lod = lod_info.level;
            tile.mesh_conf.target = lod_info.main;

            tile.mesh_conf.special_case = lod_info.special_case;
            tile.mesh_conf.special_case_corner = lod_info.special_case_corner;
            tile.mesh_conf.target_top = lod_info.top;
            tile.mesh_conf.target_bottom = lod_info.bottom;
            tile.mesh_conf.target_left = lod_info.left;
            tile.mesh_conf.target_right = lod_info.right;

            tile.mesh_conf.target_corner_tl = lod_info.corner_tl;
            tile.mesh_conf.target_corner_tr = lod_info.corner_tr;
            tile.mesh_conf.target_corner_bl = lod_info.corner_bl;
            tile.mesh_conf.target_corner_br = lod_info.corner_br;

            tile.mesh_conf.current = tile.mesh_conf.target;

            commands.entity(entity).insert(TileMeshGenerationQueued);
        }
    }
}
// ----------------------------------------------------------------------------
// tracker
// ----------------------------------------------------------------------------
impl MeshLodTracker {
    // ------------------------------------------------------------------------
    pub fn new(conf: &TerrainConfig) -> Self {
        Self {
            forced_update: false,
            last_pos: Vec2::ZERO,
            lods: HashMap::with_capacity(conf.tile_count()),
            changed: HashSet::with_capacity(conf.tile_count()),
        }
    }
    // ------------------------------------------------------------------------
    // pub fn force_update(&mut self) {
    //     self.forced_update = true;
    // }
    // ------------------------------------------------------------------------
    /// skips update if new position did not change significantly from last
    /// run check
    pub fn lazy_update(&mut self, pos: Vec2) -> bool {
        // if self.forced_update || self.last_pos.distance(pos) > (TILE_SIZE / 4) as f32 {
        if self.forced_update || self.last_pos.distance(pos) > 4.0 {
            // self.update(pos)
            self.last_pos = pos;
            true
        } else {
            false
        }
    }
    // ------------------------------------------------------------------------
    fn propagate_updates(
        &mut self,
        tileid: TerrainTileId<TILE_SIZE>,
        priority: u32,
        target_lod: &TerrainLodSettings,
    ) {
        // // TODO sort out target/current usage (currently target is used for generation but current not updated)
        // tile.mesh_conf.target = settings.threshold;

        let tile_lod = self.lods.get_mut(&tileid).unwrap();

        // priority is updated always because bordering tiles *may* need propagated updates, too
        // and these updates should reflect priority (based on visibility and distance)
        tile_lod.priority = priority;

        // if tile_lod.main != target_lod.threshold || tile_lod.new_main != target_lod.threshold
        //     || tile_lod.special_case
        // if current_main != target_lod.threshold
        // if tile_lod.lod_target != target_lod.threshold
        // || tile_lod.main != target_lod.threshold    //??
        // || tile_lod.special_case                    //??
        {
            // update target tile...
            // tile_lod.lod_target = target_lod.threshold;
            tile_lod.new_main = target_lod.threshold;
            tile_lod.new_top = target_lod.threshold;
            tile_lod.new_bottom = target_lod.threshold;
            tile_lod.new_left = target_lod.threshold;
            tile_lod.new_right = target_lod.threshold;

            tile_lod.new_corner_tl = target_lod.threshold;
            tile_lod.new_corner_tr = target_lod.threshold;
            tile_lod.new_corner_bl = target_lod.threshold;
            tile_lod.new_corner_br = target_lod.threshold;

            tile_lod.level = target_lod.level;

            // self.changed.insert(tile_lod.id);
            self.changed.insert(tileid);

            if self.lods.contains_key(&tileid.top()) {
                self.changed.insert(tileid.top());
            }
            if self.lods.contains_key(&tileid.bottom()) {
                self.changed.insert(tileid.bottom());
            }
            if self.lods.contains_key(&tileid.left()) {
                self.changed.insert(tileid.left());
            }
            if self.lods.contains_key(&tileid.right()) {
                self.changed.insert(tileid.right());
            }
        }
    }
    // ------------------------------------------------------------------------
    fn patch_seams(&mut self) {
        // 2 passes

        // new_main as max error from surrounding tile targets
        for changed_tile in &self.changed {
            let tile_main = self.lods.get(changed_tile).unwrap().new_main;

            let top = self
                .lods
                .get(&changed_tile.top())
                .map_or(tile_main, |t| t.new_main);
            let bottom = self
                .lods
                .get(&changed_tile.bottom())
                .map_or(tile_main, |t| t.new_main);
            let left = self
                .lods
                .get(&changed_tile.left())
                .map_or(tile_main, |t| t.new_main);
            let right = self
                .lods
                .get(&changed_tile.right())
                .map_or(tile_main, |t| t.new_main);

            let top_left = self
                .lods
                .get(&changed_tile.corner_top_left())
                .map_or(tile_main, |t| t.new_main);
            let top_right = self
                .lods
                .get(&changed_tile.corner_top_right())
                .map_or(tile_main, |t| t.new_main);
            let bottom_left = self
                .lods
                .get(&changed_tile.corner_bottom_left())
                .map_or(tile_main, |t| t.new_main);
            let bottom_right = self
                .lods
                .get(&changed_tile.corner_bottom_right())
                .map_or(tile_main, |t| t.new_main);

            self.lods.get_mut(changed_tile).unwrap().new_merged_main = top
                .max(bottom)
                .max(left)
                .max(right)
                .max(top_left)
                .max(top_right)
                .max(bottom_left)
                .max(bottom_right)
                .max(tile_main);
        }

        for changed_tile in &self.changed {
            let tile = self.lods.get(changed_tile).unwrap();

            if tile.new_main != tile.new_merged_main {
                let tile_main = tile.new_merged_main;

                let extract_threshold = |thresholds: &TrackedMeshErrorThresholds| -> f32 {
                    if thresholds.new_main != thresholds.new_merged_main {
                        // also a border tile -> cannot use threshold as is
                        // naive approach to take from merged_main
                        // ---
                        //  2
                        // ---
                        // {2}   taken from above merged_main
                        // (2)   merged_main
                        // {1}   taken from below merged_main   |
                        // ---                                  | conflict due to different border values
                        // {2}   taken from above merged_main   |
                        // (1)   merge_main
                        // {0}   taekn from below merged_main
                        // ---
                        //  0
                        // ---
                        //
                        // therefore min of conflicting border values for both tiles
                        //
                        tile_main.min(thresholds.new_merged_main)
                    } else {
                        thresholds.new_merged_main
                    }
                };

                let new_top = self
                    .lods
                    .get(&changed_tile.top())
                    .map_or(tile_main, extract_threshold);
                let new_bottom = self
                    .lods
                    .get(&changed_tile.bottom())
                    .map_or(tile_main, extract_threshold);
                let new_left = self
                    .lods
                    .get(&changed_tile.left())
                    .map_or(tile_main, extract_threshold);
                let new_right = self
                    .lods
                    .get(&changed_tile.right())
                    .map_or(tile_main, extract_threshold);

                let new_corner_top_left = self
                    .lods
                    .get(&changed_tile.corner_top_left())
                    .map_or(tile_main, extract_threshold);
                let new_corner_top_right = self
                    .lods
                    .get(&changed_tile.corner_top_right())
                    .map_or(tile_main, extract_threshold);
                let new_corner_bottom_left = self
                    .lods
                    .get(&changed_tile.corner_bottom_left())
                    .map_or(tile_main, extract_threshold);
                let new_corner_bottom_right = self
                    .lods
                    .get(&changed_tile.corner_bottom_right())
                    .map_or(tile_main, extract_threshold);

                let tile = self.lods.get_mut(changed_tile).unwrap();

                tile.new_top = new_top;
                tile.new_bottom = new_bottom;
                tile.new_left = new_left;
                tile.new_right = new_right;

                tile.new_corner_tl = new_corner_top_left;
                tile.new_corner_tr = new_corner_top_right;
                tile.new_corner_bl = new_corner_bottom_left;
                tile.new_corner_br = new_corner_bottom_right;
            }
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// internal helper
// ----------------------------------------------------------------------------
struct TrackedMeshErrorThresholds {
    main: f32,
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,

    corner_tl: f32,
    corner_tr: f32,
    corner_bl: f32,
    corner_br: f32,

    new_main: f32,
    new_top: f32,
    new_bottom: f32,
    new_left: f32,
    new_right: f32,

    new_corner_tl: f32,
    new_corner_tr: f32,
    new_corner_bl: f32,
    new_corner_br: f32,

    new_merged_main: f32,

    level: u8,
    priority: u32,
    special_case: bool,
    special_case_corner: bool,
}
// ----------------------------------------------------------------------------
impl TrackedMeshErrorThresholds {
    // ------------------------------------------------------------------------
    fn new(main: f32, level: u8) -> Self {
        Self {
            main,
            top: main,
            bottom: main,
            left: main,
            right: main,

            corner_tl: main,
            corner_tr: main,
            corner_bl: main,
            corner_br: main,

            level,
            priority: u32::MAX,

            new_main: main,
            new_top: main,
            new_bottom: main,
            new_left: main,
            new_right: main,

            new_corner_tl: main,
            new_corner_tr: main,
            new_corner_bl: main,
            new_corner_br: main,

            new_merged_main: main,

            special_case: false,
            special_case_corner: false,
        }
    }
    // ------------------------------------------------------------------------
    fn changed(&mut self) -> bool {
        if
        // self.main != self.new_main
        self.main != self.new_merged_main
            || self.top != self.new_top
            || self.bottom != self.new_bottom
            || self.left != self.new_left
            || self.right != self.new_right
            || self.corner_tl != self.new_corner_tl
            || self.corner_tr != self.new_corner_tr
            || self.corner_bl != self.new_corner_bl
            || self.corner_br != self.new_corner_br
        {
            // self.main = self.new_main;
            self.main = self.new_merged_main;
            // self.new_main = self.new_merged_main;
            self.top = self.new_top;
            self.left = self.new_left;
            self.bottom = self.new_bottom;
            self.right = self.new_right;

            self.corner_tl = self.new_corner_tl;
            self.corner_tr = self.new_corner_tr;
            self.corner_bl = self.new_corner_bl;
            self.corner_br = self.new_corner_br;

            self.special_case_corner = self.main != self.corner_tl
                || self.main != self.corner_tr
                || self.main != self.corner_bl
                || self.main != self.corner_br;

            self.special_case = self.special_case_corner
                || self.main != self.top
                || self.main != self.bottom
                || self.main != self.left
                || self.main != self.right;

            true
        } else {
            false
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl MeshReduction {
    // ------------------------------------------------------------------------
    pub fn get_error_threshold(&self, triangle: &TileTriangle) -> f32 {
        // TODO reduce to less checks?
        if self.special_case {
            if triangle.a() == UVec2::ZERO
                || triangle.b() == UVec2::ZERO
                || triangle.c() == UVec2::ZERO
            {
                self.target_top
                    .min(self.target_left)
                    .min(self.target_corner_tl)
            } else if triangle.a() == UVec2::new(0, TILE_SIZE)
                || triangle.b() == UVec2::new(0, TILE_SIZE)
                || triangle.c() == UVec2::new(0, TILE_SIZE)
            {
                self.target_bottom
                    .min(self.target_left)
                    .min(self.target_corner_bl)
            } else if triangle.a() == UVec2::new(TILE_SIZE, 0)
                || triangle.b() == UVec2::new(TILE_SIZE, 0)
                || triangle.c() == UVec2::new(TILE_SIZE, 0)
            {
                self.target_top
                    .min(self.target_right)
                    .min(self.target_corner_tr)
            } else if triangle.a() == UVec2::new(TILE_SIZE, TILE_SIZE)
                || triangle.b() == UVec2::new(TILE_SIZE, TILE_SIZE)
                || triangle.c() == UVec2::new(TILE_SIZE, TILE_SIZE)
            {
                self.target_bottom
                    .min(self.target_right)
                    .min(self.target_corner_br)
            } else if triangle.a().x == 0 || triangle.b().x == 0 || triangle.c().x == 0 {
                self.target_left
            } else if triangle.a().x == TILE_SIZE
                || triangle.b().x == TILE_SIZE
                || triangle.c().x == TILE_SIZE
            {
                self.target_right
            } else if triangle.a().y == 0 || triangle.b().y == 0 || triangle.c().y == 0 {
                self.target_top
            } else if triangle.a().y == TILE_SIZE
                || triangle.b().y == TILE_SIZE
                || triangle.c().y == TILE_SIZE
            {
                self.target_bottom
            } else {
                self.target
            }
        } else {
            self.target
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<const TILE_SIZE: u32> TerrainTileId<TILE_SIZE> {
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn top(&self) -> Self {
        Self::new(self.x(), self.y().saturating_sub(1))
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn bottom(&self) -> Self {
        Self::new(self.x(), self.y() + 1)
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn left(&self) -> Self {
        Self::new(self.x().saturating_sub(1), self.y())
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn right(&self) -> Self {
        Self::new(self.x() + 1, self.y())
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn corner_top_left(&self) -> Self {
        Self::new(self.x().saturating_sub(1), self.y().saturating_sub(1))
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn corner_top_right(&self) -> Self {
        Self::new(self.x() + 1, self.y().saturating_sub(1))
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn corner_bottom_left(&self) -> Self {
        Self::new(self.x().saturating_sub(1), self.y() + 1)
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn corner_bottom_right(&self) -> Self {
        Self::new(self.x() + 1, self.y() + 1)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
