// ----------------------------------------------------------------------------
use bevy::{
    ecs::schedule::StateData,
    math::{Vec2, Vec3Swizzles},
    prelude::*,
};

use crate::config::{TerrainConfig, CLIPMAP_GRANULARITY, CLIPMAP_SIZE};

use crate::clipmap::{Clipmap, Rectangle};
use crate::texturearray::TextureArray;

use crate::texturecontrol::TextureControl;
use crate::tintmap::TintMap;

pub use crate::terrain_render::{ClipmapAssignment, TerrainClipmap};
// ----------------------------------------------------------------------------
/// Marker component for entity to be used for tracking position. Based on this
/// position clipmap layer rectangles will be calculated.
#[derive(Component)]
pub struct ClipmapAnchor;
// ----------------------------------------------------------------------------
#[derive(Default, Clone, Debug)]
pub struct LayerRectangle {
    rectangle: Rectangle,
    changed: bool,
}
// ----------------------------------------------------------------------------
/// Container for clipmap rectangle positions for all clipmap levels based on
/// tracked ClipmapAnchor position.
pub struct ClipmapTracker {
    /// current position and size of rectangle covering full resolution data for
    /// for all managed layers.
    layer_rectangles: Vec<LayerRectangle>,
    /// offset [m] to apply to world coordinates for mapping to unsigned clipmap
    /// data coordinates (applied before scaling)
    world_offset: Vec2,
    /// scaling of world coordinates for mapping to clipmap coordinates (applied)
    /// after offset)
    /// [heightmap px / meter]
    world_resolution: f32,
    /// full res data size (width == height)
    data_size: u32,
    /// last position from anchor. used to check if anything changed to skip
    /// update loop.
    last_pos: Vec2,

    forced_update: bool,
}
// ----------------------------------------------------------------------------
/// [Resource] Clipmap for terrain texturing information (background and overlay
/// texture id, background textrure scaling, slope blending). Stores full
/// resolution data and can generate rectangle views of downscaled versions as
/// required by different clipmap levels.
#[derive(Default)]
pub struct TextureControlClipmap(Clipmap<CLIPMAP_SIZE, TextureControl>);
// ----------------------------------------------------------------------------
/// [Resource] Clipmap for terrain tint information (darkening or screen blend
/// for RGB channels layered over used terrain texture color). Stores full
/// resolution data and can generate rectangle views of downscaled versions as
/// required by different clipmap levels.
#[derive(Default)]
pub struct TintClipmap(Clipmap<CLIPMAP_SIZE, TintMap>);
// ----------------------------------------------------------------------------
/// Plugin for generating a clipmap with multiple resolution views of different
/// terrain data (e.g. texturing and tint coloring) for specific positions based
/// on a tracked ClipmapAnchor component position.
pub struct TerrainClipmapPlugin;
// ----------------------------------------------------------------------------
impl TerrainClipmapPlugin {
    // ------------------------------------------------------------------------
    pub fn init_tracker<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_enter(state).with_system(init_clipmap_tracker)
    }
    // ------------------------------------------------------------------------
    pub fn update_tracker<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_update(state).with_system(update_clipmaps)
    }
    // ------------------------------------------------------------------------
    pub fn reset_data<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_enter(state).with_system(remove_clipmaps)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Plugin for TerrainClipmapPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.init_resource::<TextureControlClipmap>()
            .init_resource::<TintClipmap>()
            .insert_resource(ClipmapTracker::new(CLIPMAP_SIZE, 1));
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl LayerRectangle {
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn rectangle(&self) -> &Rectangle {
        &self.rectangle
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
mod tracker;
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
pub fn enable_caching(
    mut texture_clipmap: ResMut<TextureControlClipmap>,
    mut tint_clipmap: ResMut<TintClipmap>,
) {
    texture_clipmap.enable_cache();
    tint_clipmap.enable_cache();
}
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn update_clipmaps(
    mut tracker: ResMut<ClipmapTracker>,
    mut texture_arrays: ResMut<Assets<TextureArray>>,
    anchor_query: Query<(&Transform, &ClipmapAnchor)>,
    mut assignment_query: Query<&mut ClipmapAssignment>,
    texture_clipmap: Res<TextureControlClipmap>,
    tint_clipmap: Res<TintClipmap>,
    mut terrain_clipmap: ResMut<TerrainClipmap>,
    // dbg
    mut editor_events: EventWriter<crate::EditorEvent>,
) {
    let update_required = if let Ok((anchor, _)) = anchor_query.get_single() {
        tracker.lazy_update(anchor.translation.xz())
    } else {
        false
    };

    if update_required {
        // update all clipmaps
        for (level, layer) in tracker.layers().filter(|(_, l)| l.changed) {
            let level = level as u8;

            texture_clipmap.update_layer(level, layer.rectangle(), texture_arrays.deref_mut());
            tint_clipmap.update_layer(level, layer.rectangle(), texture_arrays.deref_mut());

            // update debug ui
            // TODO hide behind a cfg/feature?
            for (label, handle) in [
                (texture_clipmap.label(), texture_clipmap.array()),
                (tint_clipmap.label(), tint_clipmap.array()),
            ] {
                editor_events.send(crate::EditorEvent::Debug(crate::DebugEvent::ClipmapUpdate(
                    label.to_string(),
                    level as u8,
                    handle.clone_weak(),
                )));
            }
        }

        // at least one clipmap level changed -> update clipmap rendering info...
        terrain_clipmap.update_clipmapinfo(tracker.info());

        // ... and all assignments
        for mut assignment in assignment_query.iter_mut() {
            let new = tracker.map_to_level(assignment.min, assignment.max);
            if assignment.level != new {
                assignment.level = new;
            }
        }
    }
}
// ----------------------------------------------------------------------------
fn init_clipmap_tracker(
    terrain_config: ResMut<TerrainConfig>,
    mut clipmap_tracker: ResMut<ClipmapTracker>,
) {
    // clipmap tracker must be updated with new dimensions before tint/texture
    // clipmaps are build (first time generation)
    *clipmap_tracker =
        ClipmapTracker::new(terrain_config.map_size(), terrain_config.clipmap_levels())
            .set_position_mapping(terrain_config.resolution(), terrain_config.map_offset());
    clipmap_tracker.update(Vec2::ZERO);
}
// ----------------------------------------------------------------------------
fn remove_clipmaps(mut commands: Commands) {
    commands.insert_resource(TintClipmap::default());
    commands.insert_resource(TextureControlClipmap::default());

    commands.insert_resource(TerrainClipmap::default());
    commands.insert_resource(ClipmapTracker::new(CLIPMAP_SIZE, 1));
}
// ----------------------------------------------------------------------------
// utils
// ----------------------------------------------------------------------------
use std::ops::{Deref, DerefMut};

impl Deref for TextureControlClipmap {
    type Target = Clipmap<CLIPMAP_SIZE, TextureControl>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
impl Deref for TintClipmap {
    type Target = Clipmap<CLIPMAP_SIZE, TintMap>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
impl DerefMut for TextureControlClipmap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
// ----------------------------------------------------------------------------
impl DerefMut for TintClipmap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
// ----------------------------------------------------------------------------
impl From<Clipmap<CLIPMAP_SIZE, TextureControl>> for TextureControlClipmap {
    fn from(c: Clipmap<CLIPMAP_SIZE, TextureControl>) -> Self {
        TextureControlClipmap(c)
    }
}
// ----------------------------------------------------------------------------
impl From<Clipmap<CLIPMAP_SIZE, TintMap>> for TintClipmap {
    fn from(c: Clipmap<CLIPMAP_SIZE, TintMap>) -> Self {
        TintClipmap(c)
    }
}
// ----------------------------------------------------------------------------
