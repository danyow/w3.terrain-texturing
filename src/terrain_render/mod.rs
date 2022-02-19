// ----------------------------------------------------------------------------
use bevy::prelude::*;
// ----------------------------------------------------------------------------
pub struct TerrainRenderPlugin;
// ----------------------------------------------------------------------------
//TODO make proper specialized mesh type so updates just take data instead clone (?)
pub type TerrainMesh = Mesh;
// ----------------------------------------------------------------------------
mod render;
// ----------------------------------------------------------------------------
impl Plugin for TerrainRenderPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.add_plugin(render::TerrainMeshRenderPlugin);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
