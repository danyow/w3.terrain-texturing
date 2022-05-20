// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{render_resource::SpecializedRenderPipelines, RenderApp, RenderStage},
};

use self::pipeline::{BrushPointerPipelineKey, BrushPointerRenderPipeline};
use self::pointer::{
    BrushPointerPipelineId, GpuBrushPointer, GpuBrushPointerInfo, GpuBrushPointerResult,
};
// ----------------------------------------------------------------------------
mod pipeline;
mod pointer;
mod render_node;
// ----------------------------------------------------------------------------
use async_channel::{Receiver, Sender};
// ----------------------------------------------------------------------------
pub use self::render_node::BrushPointerNode;
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
            .init_resource::<SpecializedRenderPipelines<BrushPointerRenderPipeline>>()
            .insert_resource(BrushPointerResultDispatcher(result_sender))
            .add_system_to_stage(RenderStage::Extract, pointer::extract_brush_pointer_info)
            .add_system_to_stage(RenderStage::Prepare, pointer::prepare_brush_pointer_info)
            .add_system_to_stage(RenderStage::Prepare, pointer::prepare_brush_pointer_result)
            .add_system_to_stage(RenderStage::Queue, pointer::queue_brush_pointer_info)
            // result can only be checked *after* the command queue was submitted!
            // so this has to be *after* the render phase
            .add_system_to_stage(RenderStage::Cleanup, pointer::check_brush_pointer_result);
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
