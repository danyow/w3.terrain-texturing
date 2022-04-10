// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_graph::RenderGraph, render_resource::SpecializedPipelines, RenderApp, RenderStage,
    },
};

use self::brush_pass::BrushPointerNode;
use self::pipeline::{BrushPointerPipelineKey, BrushPointerRenderPipeline};
use self::pointer::{
    BrushPointerPipelineId, GpuBrushPointer, GpuBrushPointerInfo, GpuBrushPointerResult,
};

use super::pipeline::terrain_3d_graph;
use super::pipeline::TerrainPassNode;
// ----------------------------------------------------------------------------
mod brush_pass;
mod pipeline;
mod pointer;
// ----------------------------------------------------------------------------
pub mod node {
    pub const BRUSH_POINTER_PASS: &str = "brush_pointer_pass";
}
// ----------------------------------------------------------------------------
use async_channel::{Receiver, Sender};
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct BrushPointer {
    pub active: bool,
    pub pos: Vec2,
    pub radius: f32,
    pub ring_width: f32,
    pub color: Color,
    pub max_visibility: f32,

    pub click_primary: bool,
    pub click_secondary: bool,
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub enum BrushPointerEventData {
    Centered(MouseButton, Vec2, f32),
}
// ----------------------------------------------------------------------------
pub struct BrushPointerEventReceiver(Receiver<BrushPointerEventData>);
// ----------------------------------------------------------------------------
struct BrushPointerResultDispatcher(Sender<BrushPointerEventData>);
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct BrushPointerRenderPlugin;
// ----------------------------------------------------------------------------
impl Plugin for BrushPointerRenderPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.init_resource::<BrushPointer>();

        // channel to push compute task results to app world
        let (result_sender, result_receiver) = async_channel::unbounded();

        app.insert_resource(BrushPointerEventReceiver(result_receiver));

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<Option<GpuBrushPointer>>()
            .init_resource::<Option<GpuBrushPointerResult>>()
            .init_resource::<BrushPointerPipelineId>()
            .init_resource::<BrushPointerRenderPipeline>()
            .init_resource::<SpecializedPipelines<BrushPointerRenderPipeline>>()
            .insert_resource(BrushPointerResultDispatcher(result_sender))
            .add_system_to_stage(RenderStage::Extract, pointer::extract_brush_pointer_info)
            .add_system_to_stage(RenderStage::Prepare, pointer::prepare_brush_pointer_info)
            .add_system_to_stage(RenderStage::Prepare, pointer::prepare_brush_pointer_result)
            .add_system_to_stage(RenderStage::Queue, pointer::queue_brush_pointer_info)
            // result can only be checked *after* the command queue was submitted!
            // so this has to be *after* the render phase
            .add_system_to_stage(RenderStage::Cleanup, pointer::check_brush_pointer_result);

        let pointer_node = BrushPointerNode::new(&mut render_app.world);

        let mut render_graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        let terrain_3d_graph = render_graph
            .get_sub_graph_mut(terrain_3d_graph::NAME)
            .unwrap();

        let input_node_id = terrain_3d_graph.input_node().unwrap().id;

        terrain_3d_graph.add_node(node::BRUSH_POINTER_PASS, pointer_node);
        terrain_3d_graph
            .add_node_edge(terrain_3d_graph::node::MAIN_PASS, node::BRUSH_POINTER_PASS)
            .unwrap();

        terrain_3d_graph
            .add_slot_edge(
                input_node_id,
                terrain_3d_graph::input::VIEW_ENTITY,
                node::BRUSH_POINTER_PASS,
                TerrainPassNode::IN_VIEW,
            )
            .unwrap();

        terrain_3d_graph
            .add_slot_edge(
                terrain_3d_graph::node::MAIN_PASS,
                TerrainPassNode::OUT_WORLD_POS,
                node::BRUSH_POINTER_PASS,
                BrushPointerNode::IN_WORLD_POS,
            )
            .unwrap();
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
use std::ops::Deref;

impl Deref for BrushPointerEventReceiver {
    type Target = Receiver<BrushPointerEventData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
