// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, CachedPipelineId, RenderPipelineCache,
            SpecializedPipelines,
        },
        renderer::RenderDevice,
        view::ViewUniforms,
    },
};

use crate::resource::PreparedRenderResource;
use crate::terrain_render::EnvironmentData;

use super::pipeline::FogRenderPipeline;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(super) struct FogPipelineId(Option<CachedPipelineId>);
// ----------------------------------------------------------------------------
pub(super) struct FogBindGroup(BindGroup);
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub(super) fn queue_fog_info(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    environment: Res<PreparedRenderResource<EnvironmentData>>,
    fog_pipeline: Res<FogRenderPipeline>,
    view_uniforms: Res<ViewUniforms>,
    mut pipelines: ResMut<SpecializedPipelines<FogRenderPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mut pipeline_id: ResMut<FogPipelineId>,
) {
    if let (Some(view_bindung), Some(env)) =
        (view_uniforms.uniforms.binding(), environment.as_ref())
    {
        pipeline_id.0 = Some(pipelines.specialize(&mut pipeline_cache, &fog_pipeline, ()));

        let fog_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_bindung.clone(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: env.sun_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: env.fog_buffer.as_entire_binding(),
                },
            ],
            label: Some("env_fog_info_bind_group"),
            layout: &fog_pipeline.info_layout,
        });

        commands.insert_resource(FogBindGroup(fog_bind_group));
    }
}
// ----------------------------------------------------------------------------
// helper
// ----------------------------------------------------------------------------
use std::ops::Deref;

impl Deref for FogPipelineId {
    type Target = Option<CachedPipelineId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
impl Deref for FogBindGroup {
    type Target = BindGroup;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
