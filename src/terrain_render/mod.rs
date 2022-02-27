// ----------------------------------------------------------------------------
use bevy::{prelude::*, reflect::TypeUuid};

use crate::texturearray::TextureArray;
// ----------------------------------------------------------------------------
pub struct TerrainRenderPlugin;
// ----------------------------------------------------------------------------
//TODO make proper specialized mesh type so updates just take data instead clone (?)
pub type TerrainMesh = Mesh;
// ----------------------------------------------------------------------------
#[derive(Default, Clone, TypeUuid)]
#[uuid = "867a207f-7ada-4fdd-8319-df7d383fa6ff"]
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
mod terrain;
// ----------------------------------------------------------------------------
impl Plugin for TerrainRenderPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.add_plugin(terrain::TerrainMeshRenderPlugin);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// material params
// ----------------------------------------------------------------------------
impl Default for TerrainMaterialParam {
    // ------------------------------------------------------------------------
    fn default() -> Self {
        // TODO check defaults
        Self {
            blend_sharpness: 0.0,
            slope_base_dampening: 0.0,
            slope_normal_dampening: 0.5,
            specularity_scale: 0.0,
            specularity: 0.0,
            specularity_base: 0.0,
            _specularity_scale_copy: 0.0,
            falloff: 0.0,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
