// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{render_resource::SpecializedRenderPipelines, RenderApp, RenderStage},
};

use super::environment::GpuTonemappingInfo;

use self::{
    pipeline::TonemappingRenderPipeline,
    systems::{TonemappingBindGroup, TonemappingPipelineId},
};
// ----------------------------------------------------------------------------
mod pipeline;
mod render_node;

mod systems;
// ----------------------------------------------------------------------------
pub use self::render_node::TonemappingNode;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct TonemappingPlugin;
// ----------------------------------------------------------------------------
impl Plugin for TonemappingPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<Option<TonemappingBindGroup>>()
            .init_resource::<TonemappingPipelineId>()
            .init_resource::<TonemappingRenderPipeline>()
            .init_resource::<SpecializedRenderPipelines<TonemappingRenderPipeline>>()
            .add_system_to_stage(RenderStage::Queue, systems::queue_tonemapping_info);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
