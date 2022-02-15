// ----------------------------------------------------------------------------
use bevy::prelude::*;
// ----------------------------------------------------------------------------
pub struct HeightmapPlugin;
// ----------------------------------------------------------------------------
impl Plugin for HeightmapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainHeightMap>();
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
