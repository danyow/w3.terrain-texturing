// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::render_resource::{CachedPipelineId, RenderPipelineCache, SpecializedPipelines},
};

use super::pipeline::TonemappingRenderPipeline;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(super) struct TonemappingPipelineId(Option<CachedPipelineId>);
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
pub(super) fn queue_tonemapping_info(
    tonemapping_pipeline: Res<TonemappingRenderPipeline>,
    mut pipelines: ResMut<SpecializedPipelines<TonemappingRenderPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mut pipeline_id: ResMut<TonemappingPipelineId>,
) {
    pipeline_id.0 = Some(pipelines.specialize(&mut pipeline_cache, &tonemapping_pipeline, ()));
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
