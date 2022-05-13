// ----------------------------------------------------------------------------
use std::sync::Mutex;

use bevy::{
    prelude::*,
    render::{
        render_graph::{self, Node, SlotInfo, SlotType},
        render_phase::TrackedRenderPass,
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, LoadOp, Operations,
            RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineCache, TextureViewId,
        },
        renderer::RenderContext,
        view::{ExtractedView, ViewUniformOffset},
    },
};

use crate::terrain_render::rendergraph::TerrainPassRenderTargets;

use super::{
    pipeline::FogRenderPipeline,
    systems::{FogBindGroup, FogPipelineId},
};
// ----------------------------------------------------------------------------
pub struct FogNode {
    query: QueryState<
        (
            &'static TerrainPassRenderTargets,
            &'static ViewUniformOffset,
        ),
        With<ExtractedView>,
    >,
    cached_texture_bind_group: Mutex<Option<(TextureViewId, TextureViewId, BindGroup)>>,
}
// ----------------------------------------------------------------------------
impl FogNode {
    // ------------------------------------------------------------------------
    pub const IN_VIEW: &'static str = "in_view";
    pub const IN_HDR_VIEW: &'static str = "in_hdr_view";
    pub const IN_WORLD_POS: &'static str = "in_world_pos";
    pub const OUT_HDR_VIEW: &'static str = "out_hdr_view";
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
impl Node for FogNode {
    // ------------------------------------------------------------------------
    fn input(&self) -> Vec<SlotInfo> {
        vec![
            SlotInfo::new(FogNode::IN_HDR_VIEW, SlotType::TextureView),
            SlotInfo::new(FogNode::IN_WORLD_POS, SlotType::TextureView),
            SlotInfo::new(FogNode::IN_VIEW, SlotType::Entity),
        ]
    }
    // ------------------------------------------------------------------------
    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::OUT_HDR_VIEW, SlotType::TextureView)]
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
        let fog_bind_group = world.get_resource::<FogBindGroup>();

        if fog_bind_group.is_none() {
            return Ok(());
        }
        let fog_bind_group = fog_bind_group.unwrap();

        let render_pipeline_cache = world.get_resource::<RenderPipelineCache>().unwrap();
        let fog_pipeline = world.get_resource::<FogRenderPipeline>().unwrap();
        let pipelineid = world.get_resource::<FogPipelineId>().unwrap();

        if pipelineid.is_none() {
            return Ok(());
        }

        let pipeline = match render_pipeline_cache.get(pipelineid.expect("cached fog pipeline")) {
            Some(pipeline) => pipeline,
            None => return Ok(()),
        };

        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let hdr_view = graph.get_input_texture(Self::IN_HDR_VIEW)?;
        let world_pos_view = graph.get_input_texture(Self::IN_WORLD_POS)?;

        let mut cached_bind_group = self.cached_texture_bind_group.lock().unwrap();
        let input_bind_group = match &mut *cached_bind_group {
            Some((hdr_id, pos_id, bind_group))
                if hdr_view.id() == *hdr_id && world_pos_view.id() == *pos_id =>
            {
                bind_group
            }
            cached_bind_group => {
                let bind_group =
                    render_context
                        .render_device
                        .create_bind_group(&BindGroupDescriptor {
                            label: Some("env_fog_in_hdr_view"),
                            layout: &fog_pipeline.input_layout,
                            entries: &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: BindingResource::TextureView(hdr_view),
                                },
                                BindGroupEntry {
                                    binding: 1,
                                    resource: BindingResource::TextureView(world_pos_view),
                                },
                            ],
                        });

                let (_, _, bind_group) =
                    cached_bind_group.insert((hdr_view.id(), world_pos_view.id(), bind_group));
                bind_group
            }
        };

        let (render_targets, view_uniform) = match self.query.get_manual(world, view_entity) {
            Ok(query) => query,
            Err(_) => return Ok(()), // No window
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("env_fog_pass"),
            color_attachments: &[RenderPassColorAttachment {
                view: &render_targets.hdr_view_2,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        };

        {
            let render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);

            let mut tracked_pass = TrackedRenderPass::new(render_pass);

            tracked_pass.set_render_pipeline(pipeline);
            tracked_pass.set_bind_group(0, input_bind_group, &[]);
            tracked_pass.set_bind_group(1, fog_bind_group, &[view_uniform.offset]);

            // 3 empty vertices. vertex shader will generate appropriate
            // (full screen) triangle
            tracked_pass.draw(0..3, 0..1);
        }

        // -- set output textures for subsequent nodes
        graph
            .set_output(Self::OUT_HDR_VIEW, render_targets.hdr_view_2.clone())
            .unwrap();

        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
