// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        camera::{CameraPlugin, ExtractedCameraNames},
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
        let extracted_cameras = world.get_resource::<ExtractedCameraNames>().unwrap();
        if let Some(camera_3d) = extracted_cameras.entities.get(CameraPlugin::CAMERA_3D) {
            graph.run_sub_graph(terrain_3d_graph::NAME, vec![SlotValue::Entity(*camera_3d)])?;
        }

        Ok(())
    }
}
// ----------------------------------------------------------------------------
