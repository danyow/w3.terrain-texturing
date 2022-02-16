//
// cmds are per "complex" operation that may involve multiple (complicated)
// sub tasks and/or be async
//
// ----------------------------------------------------------------------------
// Note: the order of the derive and enum_dispatch is important: setting it
// *after* hash + eq ensures only distinc tasks are queued (payload like path
// is ignored - which is what is desired here)
#[derive(Hash, PartialEq, Eq)]
#[enum_dispatch]
#[derive(Debug)]
pub enum AsyncTask {
    LoadHeightmap,
    GenerateHeightmapNormals,
    GenerateTerrainTiles,
    GenerateTerrainMeshErrorMaps,
    GenerateTerrainMeshes,
    LoadTerrainMaterialSet,
    WaitForTerrainLoaded,
}
// ----------------------------------------------------------------------------
#[derive(Debug, Copy, Clone)]
pub enum AsyncTaskStartEvent {
    LoadHeightmap,
    GenerateHeightmapNormals,
    GenerateTerrainTiles,
    GenerateTerrainMeshErrorMaps,
    GenerateTerrainMeshes,
    LoadTerrainMaterialSet,
    WaitForTerrainLoaded,
}
// ----------------------------------------------------------------------------
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum AsyncTaskFinishedEvent {
    HeightmapLoaded,
    HeightmapNormalsGenerated,
    TerrainTilesGenerated,
    TerrainMeshErrorMapsGenerated,
    TerrainMeshesGenerated,
    TerrainLoaded,
    TerrainMaterialSetLoaded,
}
// ----------------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct LoadHeightmap;
// ----------------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct GenerateHeightmapNormals;
// ----------------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct GenerateTerrainTiles;
// ----------------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct GenerateTerrainMeshErrorMaps;
// ----------------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct GenerateTerrainMeshes;
// ----------------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct LoadTerrainMaterialSet;
// ----------------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct WaitForTerrainLoaded;
// ----------------------------------------------------------------------------
// #[derive(Debug)]
// pub struct LoadTerrainMaterialTexture(crate::MaterialSlot, crate::loader::TextureType, String);
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
pub(crate) use async_cmds::poll_async_task_state;
pub(crate) use async_cmds::start_async_operations;
pub(crate) use async_cmds::AsyncCmdsPlugin;
pub(crate) use async_cmds::AsyncCommandManager;
// ----------------------------------------------------------------------------
mod async_cmds;
// ----------------------------------------------------------------------------
use enum_dispatch::enum_dispatch;
// ----------------------------------------------------------------------------
