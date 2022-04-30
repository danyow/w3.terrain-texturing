// ----------------------------------------------------------------------------
use std::sync::Mutex;

use bevy::{
    prelude::*,
    render::{
        render_graph::{self, Node, SlotInfo, SlotType},
        render_phase::TrackedRenderPass,
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, LoadOp, Operations,
            RenderPassDescriptor, RenderPipelineCache, TextureViewId,
        },
        renderer::RenderContext,
        view::{ExtractedView, ViewTarget},
    },
};

use super::{pipeline::TonemappingRenderPipeline, systems::{TonemappingPipelineId, TonemappingBindGroup}};
// ----------------------------------------------------------------------------
pub struct TonemappingNode {
    query: QueryState<&'static ViewTarget, With<ExtractedView>>,
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}
// ----------------------------------------------------------------------------
impl TonemappingNode {
    // ------------------------------------------------------------------------
    pub const IN_VIEW: &'static str = "in_view";
    pub const IN_HDR_VIEW: &'static str = "in_hdr_view";
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
impl Node for TonemappingNode {
    // ------------------------------------------------------------------------
    fn input(&self) -> Vec<SlotInfo> {
        vec![
            SlotInfo::new(TonemappingNode::IN_HDR_VIEW, SlotType::TextureView),
            SlotInfo::new(TonemappingNode::IN_VIEW, SlotType::Entity),
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
        let tonemapping_bind_group = world.get_resource::<TonemappingBindGroup>();

        if tonemapping_bind_group.is_none() {
            return Ok(());
        }
        let tonemapping_bind_group = tonemapping_bind_group.unwrap();

        let render_pipeline_cache = world.get_resource::<RenderPipelineCache>().unwrap();
        let tonemapping_pipeline = world.get_resource::<TonemappingRenderPipeline>().unwrap();
        let pipelineid = world.get_resource::<TonemappingPipelineId>().unwrap();

        if pipelineid.is_none() {
            return Ok(());
        }

        let pipeline =
            match render_pipeline_cache.get(pipelineid.expect("cached tonemapping pipeline")) {
                Some(pipeline) => pipeline,
                None => return Ok(()),
            };

        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let hdr_view = graph.get_input_texture(Self::IN_HDR_VIEW)?;

        let mut cached_bind_group = self.cached_texture_bind_group.lock().unwrap();
        let input_bind_group = match &mut *cached_bind_group {
            Some((id, bind_group)) if hdr_view.id() == *id => bind_group,
            cached_bind_group => {
                let bind_group =
                    render_context
                        .render_device
                        .create_bind_group(&BindGroupDescriptor {
                            label: Some("tonemapping_in_hdr_view"),
                            layout: &tonemapping_pipeline.input_layout,
                            entries: &[BindGroupEntry {
                                binding: 0,
                                resource: BindingResource::TextureView(hdr_view),
                            }],
                        });

                let (_, bind_group) = cached_bind_group.insert((hdr_view.id(), bind_group));
                bind_group
            }
        };

        let target = match self.query.get_manual(world, view_entity) {
            Ok(query) => query,
            Err(_) => return Ok(()), // No window
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("tonemapping_pass"),
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
            tracked_pass.set_bind_group(1, tonemapping_bind_group, &[]);

            // 3 empty vertices. vertex shader will generate appropriate
            // (full screen) triangle
            tracked_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
