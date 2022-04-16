// ----------------------------------------------------------------------------
use bevy::prelude::*;

use crate::clipmap::Rectangle;
use crate::texturearray::TextureArray;

use crate::terrain_clipmap::{TextureControlClipmap, TintClipmap};
// ----------------------------------------------------------------------------
pub struct TerrainRenderPlugin;
// ----------------------------------------------------------------------------
pub use brush::{BrushPointer, BrushPointerEventData, BrushPointerEventReceiver};

pub use terrain::TerrainEnvironment;
pub use terrain::{TerrainMesh, TerrainMeshStats, TerrainMeshVertexData};
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
mod pipeline;
mod terrain;
// ----------------------------------------------------------------------------
impl Plugin for TerrainRenderPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainMaterialSet>()
            .init_resource::<TerrainEnvironment>()
            .add_plugin(pipeline::TerrainRenderGraphPlugin)
            .add_plugin(terrain::TerrainMeshRenderPlugin)
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
            resolution: rectangle.size.x as f32 / clipmap_size as f32,
            size: rectangle.size.x as f32,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
