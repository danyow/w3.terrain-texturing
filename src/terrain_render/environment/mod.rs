// ----------------------------------------------------------------------------
use bevy::prelude::{App, Plugin};

use crate::resource::RenderResourcePlugin;
// ----------------------------------------------------------------------------
mod fog;
mod resource;
// ----------------------------------------------------------------------------
pub use self::fog::FogNode;
pub use self::resource::FogState;
// ----------------------------------------------------------------------------
pub struct EnvironmentDataPlugin;
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub struct EnvironmentData {
    pub sun: DirectionalLight,
    pub fog: FogState,
    pub tonemapping: Tonemapping,
}
// ----------------------------------------------------------------------------
pub use self::resource::{DirectionalLight, Tonemapping};
// ----------------------------------------------------------------------------
pub(super) use self::resource::{GpuDirectionalLight, GpuFogSettings, GpuTonemappingInfo};
// ----------------------------------------------------------------------------
impl Plugin for EnvironmentDataPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.init_resource::<EnvironmentData>()
            .add_plugin(RenderResourcePlugin::<EnvironmentData>::default())
            .add_plugin(fog::EnvironmentFogPlugin);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
