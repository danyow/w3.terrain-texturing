// ----------------------------------------------------------------------------
use bevy::prelude::*;

use crate::clipmap::Rectangle;
use crate::resource::RenderResourcePlugin;
use crate::texturearray::TextureArray;

use crate::terrain_clipmap::{TextureControlClipmap, TintClipmap};
// ----------------------------------------------------------------------------
pub struct TerrainRenderPlugin;
// ----------------------------------------------------------------------------
pub use brush::{BrushPointer, BrushPointerEventData, BrushPointerEventReceiver};

pub use environment::{DirectionalLight, EnvironmentData, FogState};

pub use terrain::{TerrainMesh, TerrainMeshStats, TerrainMeshVertexData};
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub struct TerrainMapInfo {
    pub map_size: u32,
    pub resolution: f32,
    pub height_min: f32,
    pub height_max: f32,
    pub clipmap_level_count: u8,
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub struct TerrainRenderSettings {
    pub use_flat_shading: bool,
    pub overlay_wireframe: bool,
    pub overlay_clipmap_level: bool,

    pub ignore_overlay_texture: bool,
    pub ignore_bkgrnd_texture: bool,
    pub ignore_tint_map: bool,
    pub disable_fog: bool,

    pub show_fragment_normals: bool,
    pub show_combined_normals: bool,
    pub show_blend_threshold: bool,
    pub show_bkgrnd_scaling: bool,
    pub show_tint_map: bool,
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
/// handles to all diffuse and normal textures and parameter settings
pub struct TerrainMaterialSet {
    pub diffuse: Handle<TextureArray>,
    pub normal: Handle<TextureArray>,
    pub parameter: [TerrainMaterialParam; 31],
}
// ----------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct TerrainMaterialParam {
    pub blend_sharpness: f32,
    pub slope_base_dampening: f32,
    pub slope_normal_dampening: f32,
    pub specularity_scale: f32,
    pub specularity: f32,
    pub specularity_base: f32,
    pub _specularity_scale_copy: f32,
    pub falloff: f32,
}
// ----------------------------------------------------------------------------
#[derive(Component, Clone, Copy)]
pub struct ClipmapAssignment {
    pub level: u8,
    /// inclusive
    pub min: Vec2,
    /// exclusive
    pub max: Vec2,
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub struct TerrainClipmap {
    texture: Handle<TextureArray>,
    tint: Handle<TextureArray>,
    clipmap: ClipmapInfo,
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub struct ClipmapInfo {
    world_offset: Vec2,
    world_res: f32,
    size: u32,
    info: Vec<ClipmapLayerInfo>,
    info_last: Vec<ClipmapLayerInfo>,
}
// ----------------------------------------------------------------------------
#[derive(Clone, Default)]
pub struct ClipmapLayerInfo {
    map_offset: UVec2,
    resolution: f32,
    size: f32,
}
// ----------------------------------------------------------------------------
mod brush;
mod environment;
mod rendergraph;
mod terrain;
mod terrain_info;
mod tonemapping;
// ----------------------------------------------------------------------------
impl Plugin for TerrainRenderPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainRenderSettings>()
            .init_resource::<TerrainMaterialSet>()
            .init_resource::<TerrainMapInfo>()
            .add_plugin(RenderResourcePlugin::<TerrainMapInfo>::default())
            .add_plugin(environment::EnvironmentDataPlugin)
            .add_plugin(rendergraph::TerrainRenderGraphPlugin)
            .add_plugin(terrain::TerrainMeshRenderPlugin)
            .add_plugin(tonemapping::TonemappingPlugin)
            .add_plugin(brush::BrushPointerRenderPlugin);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TerrainClipmap {
    // ------------------------------------------------------------------------
    pub fn update_clipmapinfo(&mut self, new_info: ClipmapInfo) {
        let last_info = self.clipmap.info.clone();
        self.clipmap = new_info;
        self.clipmap.info_last = last_info;
    }
    // ------------------------------------------------------------------------
    pub fn set_texture_clipmap(&mut self, clipmap: &TextureControlClipmap) {
        self.texture = clipmap.array().clone();
    }
    // ------------------------------------------------------------------------
    pub fn set_tint_clipmap(&mut self, clipmap: &TintClipmap) {
        self.tint = clipmap.array().clone();
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ClipmapAssignment {
    // ------------------------------------------------------------------------
    pub fn new(level: u8, center: Vec2, size: Vec2) -> Self {
        Self {
            level,
            min: center - 0.5 * size,
            max: center + 0.5 * size,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ClipmapInfo {
    // ------------------------------------------------------------------------
    pub fn new(
        world_offset: Vec2,
        world_res: f32,
        clipmap_size: u32,
        info: Vec<ClipmapLayerInfo>,
    ) -> Self {
        Self {
            world_offset,
            world_res,
            size: clipmap_size,
            info,
            info_last: Vec::default(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ClipmapLayerInfo {
    // ------------------------------------------------------------------------
    pub fn new(rectangle: &Rectangle, clipmap_size: u32) -> Self {
        Self {
            map_offset: rectangle.pos,
            // WORKAROUND:
            // accessing clipmap at max coords results in a visible border
            // because of rounding errors (?) -> arbitrarily reduce the range
            resolution: rectangle.size.x as f32 / (clipmap_size - 1) as f32,
            size: rectangle.size.x as f32,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TerrainRenderSettings {
    // ------------------------------------------------------------------------
    pub fn exclusive_view_active(&self) -> bool {
        self.show_fragment_normals
            || self.show_combined_normals
            || self.show_blend_threshold
            || self.show_bkgrnd_scaling
            || self.show_tint_map
    }
    // ------------------------------------------------------------------------
    pub fn reset_exclusive_view(&mut self) {
        self.show_fragment_normals = false;
        self.show_combined_normals = false;
        self.show_blend_threshold = false;
        self.show_bkgrnd_scaling = false;
        self.show_tint_map = false;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
