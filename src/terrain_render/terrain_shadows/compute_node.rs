// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_graph::{self, Node},
        render_resource::{ComputePassDescriptor, PipelineCache},
        renderer::RenderContext,
    },
};

use crate::resource::PreparedRenderResource;

use super::systems::ComputeTerrainLightheightPipelineId;
use super::{TerrainLightheightClipmap, TerrainShadowsComputeInput, TerrainShadowsLightrayInfo};
// ----------------------------------------------------------------------------
pub struct ComputeTerrainShadowsNode;
// ----------------------------------------------------------------------------
impl ComputeTerrainShadowsNode {
    // ------------------------------------------------------------------------
    pub fn new(_world: &mut World) -> Self {
        Self {}
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Node for ComputeTerrainShadowsNode {
    // ------------------------------------------------------------------------
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipelineid = world
            .get_resource::<ComputeTerrainLightheightPipelineId>()
            .unwrap();

        if pipelineid.is_none() {
            return Ok(());
        }

        let pipeline_cache = world.get_resource::<PipelineCache>().unwrap();
        let pipeline = match pipeline_cache
            .get_compute_pipeline(pipelineid.expect("cached compute terrain shadow pipeline"))
        {
            Some(pipeline) => pipeline,
            None => return Ok(()),
        };

        let lightheightmap =
            world.get_resource::<PreparedRenderResource<TerrainLightheightClipmap>>();
        let lightray_info =
            world.get_resource::<PreparedRenderResource<TerrainShadowsLightrayInfo>>();
        let compute_input =
            world.get_resource::<PreparedRenderResource<TerrainShadowsComputeInput>>();

        let lightheightmap = match lightheightmap.unwrap() {
            Some(lightheightmap) => lightheightmap,
            None => return Ok(()),
        };
        let compute_input = match compute_input.unwrap() {
            Some(compute_input) => compute_input,
            None => return Ok(()),
        };
        let lightray_info = match lightray_info.unwrap() {
            Some(lightray_info) => lightray_info,
            None => return Ok(()),
        };

        let mut pass = render_context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, &lightheightmap.bind_group, &[]);
        pass.set_bind_group(1, &compute_input.bind_group, &[]);
        pass.set_bind_group(2, &lightray_info.bind_group, &[]);

        pass.set_pipeline(pipeline);

        pass.dispatch(1, 1, 1);

        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
