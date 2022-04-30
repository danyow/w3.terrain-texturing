// ----------------------------------------------------------------------------
use std::sync::Mutex;

use bevy::{
    prelude::*,
    render::{
        render_graph::{self, Node, SlotInfo, SlotType},
        render_phase::TrackedRenderPass,
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, LoadOp, Operations,
            RenderPassDescriptor, RenderPipelineCache, SamplerDescriptor, TextureViewId,
        },
        renderer::RenderContext,
        view::{ExtractedView, ViewTarget},
    },
};

use super::{pipeline::BrushPointerRenderPipeline, GpuBrushPointerResult};
use super::{BrushPointerPipelineId, GpuBrushPointer};
// ----------------------------------------------------------------------------
pub struct BrushPointerNode {
    query: QueryState<&'static ViewTarget, With<ExtractedView>>,
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}
// ----------------------------------------------------------------------------
impl BrushPointerNode {
    // ------------------------------------------------------------------------
    pub const IN_VIEW: &'static str = "view";
    pub const IN_WORLD_POS: &'static str = "in_world_pos";
    // ------------------------------------------------------------------------
    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
            cached_texture_bind_group: Mutex::new(None),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Node for BrushPointerNode {
    // ------------------------------------------------------------------------
    fn input(&self) -> Vec<SlotInfo> {
        vec![
            SlotInfo::new(BrushPointerNode::IN_WORLD_POS, SlotType::TextureView),
            SlotInfo::new(BrushPointerNode::IN_VIEW, SlotType::Entity),
        ]
    }
    // ------------------------------------------------------------------------
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }
    // ------------------------------------------------------------------------
    fn run(
        &self,
        graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let brushpointer = world.get_resource::<Option<GpuBrushPointer>>().unwrap();
        let brushpointer_result = world
            .get_resource::<Option<GpuBrushPointerResult>>()
            .unwrap();

        if brushpointer.is_none() {
            return Ok(());
        }
        if brushpointer_result.is_none() {
            return Ok(());
        }

        let brushpointer = brushpointer.as_ref().unwrap();
        let brushpointer_result = brushpointer_result.as_ref().unwrap();

        let render_pipeline_cache = world.get_resource::<RenderPipelineCache>().unwrap();
        let brush_pointer_pipeline = world.get_resource::<BrushPointerRenderPipeline>().unwrap();
        let pipelineid = world.get_resource::<BrushPointerPipelineId>().unwrap();

        let pipeline =
            match render_pipeline_cache.get(pipelineid.expect("cached brush_pointer pipeline")) {
                Some(pipeline) => pipeline,
                None => return Ok(()),
            };

        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let world_pos_view = graph.get_input_texture(Self::IN_WORLD_POS)?;

        let mut cached_bind_group = self.cached_texture_bind_group.lock().unwrap();
        let input_bind_group = match &mut *cached_bind_group {
            Some((id, bind_group)) if world_pos_view.id() == *id => bind_group,
            cached_bind_group => {
                let sampler = render_context
                    .render_device
                    .create_sampler(&SamplerDescriptor::default());

                let bind_group =
                    render_context
                        .render_device
                        .create_bind_group(&BindGroupDescriptor {
                            label: Some("brush_pointer_in_world_pos"),
                            layout: &brush_pointer_pipeline.input_layout,
                            entries: &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: BindingResource::TextureView(world_pos_view),
                                },
                                BindGroupEntry {
                                    binding: 1,
                                    resource: BindingResource::Sampler(&sampler),
                                },
                            ],
                        });

                let (_, bind_group) = cached_bind_group.insert((world_pos_view.id(), bind_group));
                bind_group
            }
        };

        let target = match self.query.get_manual(world, view_entity) {
            Ok(query) => query,
            Err(_) => return Ok(()), // No window
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("brush_pointer_pass"),
            // brush will be overlayed -> load the colors of current result
            color_attachments: &[target.get_color_attachment(Operations {
                load: LoadOp::Load,
                store: true,
            })],
            depth_stencil_attachment: None,
        };

        {
            let render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);

            let mut tracked_pass = TrackedRenderPass::new(render_pass);

            tracked_pass.set_render_pipeline(pipeline);
            tracked_pass.set_bind_group(0, input_bind_group, &[]);
            tracked_pass.set_bind_group(1, &brushpointer.bind_group, &[]);

            // result is only interestng if it was requested (e.g. by a mouse click)
            if brushpointer.request_result {
                tracked_pass.set_bind_group(2, &brushpointer_result.bind_group, &[]);
            }

            // 3 empty vertices. vertex shader will generate appropriate
            // (full screen) triangle
            tracked_pass.draw(0..3, 0..1);
        }

        // result is only interestng if it was requested (e.g. by a mouse click)
        if brushpointer.request_result {
            render_context.command_encoder.copy_buffer_to_buffer(
                &brushpointer_result.result_buffer,
                0,
                &brushpointer_result.staging_buffer,
                0,
                brushpointer_result.staging_buffer_size,
            );
        }

        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
