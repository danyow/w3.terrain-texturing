// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        camera::{ActiveCamera, Camera3d},
        render_graph::{Node, NodeRunError, RenderGraphContext, SlotValue},
        renderer::RenderContext,
    },
};

use super::terrain_3d_graph;
// ----------------------------------------------------------------------------
pub struct TerrainPassDriverNode;
// ----------------------------------------------------------------------------
impl Node for TerrainPassDriverNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if let Some(camera_3d) = world.resource::<ActiveCamera<Camera3d>>().get() {
            graph.run_sub_graph(terrain_3d_graph::NAME, vec![SlotValue::Entity(camera_3d)])?;
        }

        Ok(())
    }
}
// ----------------------------------------------------------------------------
