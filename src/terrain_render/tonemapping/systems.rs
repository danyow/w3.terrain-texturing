// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, CachedPipelineId, RenderPipelineCache,
            SpecializedPipelines,
        },
        renderer::RenderDevice,
    },
};

use crate::resource::PreparedRenderResource;
use crate::terrain_render::{EnvironmentData, TerrainRenderSettings};

use super::pipeline::{TonemappingPipelineKey, TonemappingRenderPipeline};
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(super) struct TonemappingPipelineId(Option<CachedPipelineId>);
// ----------------------------------------------------------------------------
pub(super) struct TonemappingBindGroup(BindGroup);
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub(super) fn queue_tonemapping_info(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    environment: Res<PreparedRenderResource<EnvironmentData>>,
    tonemapping_pipeline: Res<TonemappingRenderPipeline>,
    settings: Res<TerrainRenderSettings>,
    mut pipelines: ResMut<SpecializedPipelines<TonemappingRenderPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mut pipeline_id: ResMut<TonemappingPipelineId>,
) {
    if let Some(env) = environment.as_ref() {
        let key = TonemappingPipelineKey::from_settings(&*settings);

        pipeline_id.0 = Some(pipelines.specialize(&mut pipeline_cache, &tonemapping_pipeline, key));

        let tonemapping_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: env.tonemapping_buffer.as_entire_binding(),
            }],
            label: Some("tonemapping_info_bind_group"),
            layout: &tonemapping_pipeline.info_layout,
        });

        commands.insert_resource(TonemappingBindGroup(tonemapping_bind_group));
    }
}
// ----------------------------------------------------------------------------
// helper
// ----------------------------------------------------------------------------
use std::ops::Deref;

impl Deref for TonemappingPipelineId {
    type Target = Option<CachedPipelineId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
impl Deref for TonemappingBindGroup {
    type Target = BindGroup;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
