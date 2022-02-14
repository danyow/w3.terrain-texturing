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
    LoadTerrainMaterialSet,
}
// ----------------------------------------------------------------------------
#[derive(Debug, Copy, Clone)]
pub enum AsyncTaskStartEvent {
    LoadTerrainMaterialSet,
}
// ----------------------------------------------------------------------------
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum AsyncTaskFinishedEvent {
    TerrainMaterialSetLoaded,
}
// ----------------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct LoadTerrainMaterialSet;
// ----------------------------------------------------------------------------
// #[derive(Debug)]
// pub struct LoadTerrainMaterialTexture(crate::MaterialSlot, crate::loader::TextureType, String);
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
pub(crate) use async_cmds::AsyncCmdsPlugin;
pub(crate) use async_cmds::AsyncCommandManager;
pub(crate) use async_cmds::start_async_operations;
pub(crate) use async_cmds::poll_async_task_state;
// ----------------------------------------------------------------------------
mod async_cmds;
// ----------------------------------------------------------------------------
use enum_dispatch::enum_dispatch;
// ----------------------------------------------------------------------------
