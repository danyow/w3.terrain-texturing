// ----------------------------------------------------------------------------
use bevy::{
    math::vec2,
    prelude::*,
    render::render_resource::{
        CachedComputePipelineId, PipelineCache, SpecializedComputePipelines,
    },
};

use super::pipeline::{ComputeShadowsPipeline, ComputeShadowsPipelineKey};

use super::{
    EnvironmentData, LightrayDirection, TerrainConfig, TerrainRenderSettings,
    TerrainShadowsComputeInput, TerrainShadowsComputeTrigger, TerrainShadowsLightrayInfo,
    TerrainShadowsRenderSettings, TerrainShadowsUpdateTracker,
};
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(super) struct ComputeTerrainLightheightPipelineId(Option<CachedComputePipelineId>);
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
// app world
// ----------------------------------------------------------------------------
pub(super) fn update_compute_terrain_shadows(
    render_settings: Res<TerrainRenderSettings>,
    mut compute_input: ResMut<TerrainShadowsComputeInput>,
    mut lightray_info: ResMut<TerrainShadowsLightrayInfo>,
    mut shadow_update_tracker: ResMut<TerrainShadowsUpdateTracker>,
    shadow_settings: Res<TerrainShadowsRenderSettings>,

    env: Res<EnvironmentData>,
    config: Res<TerrainConfig>,
) {
    fn clamp(v: f32, min: f32, max: f32) -> f32 {
        v.max(min).min(max)
    }

    if render_settings.disable_shadows || compute_input.clipmap_info.size == 0 {
        return;
    }

    shadow_update_tracker.recompute_frequency = shadow_settings.recompute_frequency as u32;

    // increasing ticks per frame/system run in order to be able to skip
    // lightheightmap recalculation based on recalculation delay shadow setting
    shadow_update_tracker.tick += 1;

    let mut ray_trace_direction_changed = false;

    if env.is_changed() {
        let light_direction = env.sun.direction;

        let res = config.resolution();

        let plane_projected_light_direction_length =
            vec2(light_direction.x, light_direction.z).length();

        // depending on the dominant light direction axis the rays will be traced
        // horizontally (refers to heightmap clipmap) or vertically.
        // main direction defines if ray is traced from left-to-right or right-to-left
        // for horizontal traces or for vertical traces top-to-bottom or bottom-to-top.
        let (dominant_axis, other_axis, is_horizontal_ray_direction, is_main_direction) =
            if light_direction.x.abs() >= light_direction.z.abs() {
                (
                    light_direction.x,
                    light_direction.z,
                    true,
                    light_direction.x > 0.0,
                )
            } else {
                (
                    light_direction.z,
                    light_direction.x,
                    false,
                    light_direction.z > 0.0,
                )
            };

        let ray_direction = match (is_horizontal_ray_direction, is_main_direction) {
            (true, true) => LightrayDirection::LeftRight,
            (true, false) => LightrayDirection::RightLeft,
            (false, true) => LightrayDirection::TopBottom,
            (false, false) => LightrayDirection::BottomTop,
        };

        // weighting controls interpolation between two adjacent heights in
        // the previous light height slice
        let weighting = clamp(
            other_axis.abs() / dominant_axis.abs().max(f32::MIN_POSITIVE), //TODO tangens
            0.0,
            1.0,
        );

        // light_direction must be normalized !
        let (offset, w0, w1) = if other_axis >= 0.0 {
            (0, weighting, 1.0 - weighting)
        } else {
            (1, 1.0 - weighting, weighting)
        };

        // Note: logically swapping x and y component for different directions doesn't have an
        // effect on vector length
        let interpolated_grid_distance = (w0 * (vec2(1.0, 1.0 - offset as f32)) * res
            + w1 * (vec2(1.0, 1.0 - offset as f32 - 1.0)) * res)
            .length();

        let grid_step_ray_vector =
            light_direction * (interpolated_grid_distance / plane_projected_light_direction_length);

        lightray_info.lightpos_offset = offset;
        lightray_info.interpolation_weight = w0;
        lightray_info.ray_height_delta = grid_step_ray_vector.y;

        ray_trace_direction_changed = lightray_info.direction != ray_direction;
        lightray_info.direction = ray_direction;

        // TODO force recompute if !daynight cycle inactive
        shadow_update_tracker.recompute = true;
    }

    if compute_input.is_changed() || ray_trace_direction_changed {
        shadow_update_tracker.force_recompute();
        compute_input.recalculate_schedule(lightray_info.as_ref());
    }
}
// ----------------------------------------------------------------------------
pub(super) fn extract_compute_shadows_trigger(
    mut commands: Commands,
    mut shadow_update_tracker: ResMut<TerrainShadowsUpdateTracker>,
    lightray_info: Res<TerrainShadowsLightrayInfo>,
    render_settings: Res<TerrainRenderSettings>,
) {
    if !render_settings.disable_shadows && shadow_update_tracker.require_recompute() {
        shadow_update_tracker.reset_recompute_trigger();
        commands.insert_resource(TerrainShadowsComputeTrigger {
            recompute: true,
            trace_direction: lightray_info.direction,
        });
    } else {
        commands.insert_resource(TerrainShadowsComputeTrigger::inactive());
    }
}
// ----------------------------------------------------------------------------
// render world
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub(super) fn queue_terrain_shadows_info(
    recompute_trigger: Res<TerrainShadowsComputeTrigger>,

    pipeline: Res<ComputeShadowsPipeline>,
    mut pipelines: ResMut<SpecializedComputePipelines<ComputeShadowsPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut pipeline_id: ResMut<ComputeTerrainLightheightPipelineId>,
) {
    if recompute_trigger.recompute {
        let key = ComputeShadowsPipelineKey::from(recompute_trigger.trace_direction);
        pipeline_id.0 = Some(pipelines.specialize(&mut pipeline_cache, &pipeline, key));
    } else {
        pipeline_id.0 = None;
    }
}
// ----------------------------------------------------------------------------
// helper
// ----------------------------------------------------------------------------
impl TerrainShadowsUpdateTracker {
    // ------------------------------------------------------------------------
    fn force_recompute(&mut self) {
        self.recompute = true;
        self.tick = self.recompute_frequency + 1;
    }
    // ------------------------------------------------------------------------
    fn require_recompute(&self) -> bool {
        self.recompute && self.tick >= self.recompute_frequency
    }
    // ------------------------------------------------------------------------
    fn reset_recompute_trigger(&mut self) {
        self.tick = 0;
        self.recompute = false;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
use std::ops::Deref;

impl Deref for ComputeTerrainLightheightPipelineId {
    type Target = Option<CachedComputePipelineId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
