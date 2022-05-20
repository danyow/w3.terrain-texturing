// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{render_resource::SpecializedRenderPipelines, RenderApp, RenderStage},
};

use super::GpuFogSettings;

use self::pipeline::FogRenderPipeline;
use self::systems::{FogBindGroup, FogPipelineId};
// ----------------------------------------------------------------------------
mod pipeline;
mod render_node;

mod systems;
// ----------------------------------------------------------------------------
pub use self::render_node::FogNode;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct EnvironmentFogPlugin;
// ----------------------------------------------------------------------------
impl Plugin for EnvironmentFogPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<Option<FogBindGroup>>()
            .init_resource::<FogPipelineId>()
            .init_resource::<FogRenderPipeline>()
            .init_resource::<SpecializedRenderPipelines<FogRenderPipeline>>()
            .add_system_to_stage(RenderStage::Queue, systems::queue_fog_info);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
