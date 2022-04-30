// ----------------------------------------------------------------------------
use bevy::prelude::{App, Plugin};

use crate::resource::RenderResourcePlugin;
// ----------------------------------------------------------------------------
mod resource;
// ----------------------------------------------------------------------------
pub struct EnvironmentDataPlugin;
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub struct EnvironmentData {
    pub sun: DirectionalLight,
    pub tonemapping: Tonemapping,
}
// ----------------------------------------------------------------------------
pub use self::resource::{DirectionalLight, Tonemapping};
// ----------------------------------------------------------------------------
pub(super) use self::resource::{GpuDirectionalLight, GpuTonemappingInfo};
// ----------------------------------------------------------------------------
impl Plugin for EnvironmentDataPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.init_resource::<EnvironmentData>()
            .add_plugin(RenderResourcePlugin::<EnvironmentData>::default());
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
