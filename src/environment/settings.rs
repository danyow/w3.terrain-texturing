// ----------------------------------------------------------------------------
use crate::terrain_render::FogState;

use super::interpolation::{ColorInterpolation, ScalarInterpolation};

use super::{EnvironmentConfig, TimeOfDay};
// ----------------------------------------------------------------------------
/// environment settings prepared for interpolated sampling
pub struct EnvironmentSettings {
    pub sun: SunSettings,
    pub fog: FogSettings,
}
// ----------------------------------------------------------------------------
pub struct SunSettings {
    pub color: ColorInterpolation,
}
// ----------------------------------------------------------------------------
pub struct FogSettings {
    appear_distance: ScalarInterpolation,
    appear_range: ScalarInterpolation,
    color_front: ColorInterpolation,
    color_middle: ColorInterpolation,
    color_back: ColorInterpolation,
    density: ScalarInterpolation,
    final_exp: ScalarInterpolation,
    distance_clamp: ScalarInterpolation,
    vertical_offset: ScalarInterpolation,
    vertical_density: ScalarInterpolation,
    vertical_density_light_front: ScalarInterpolation,
    vertical_density_light_back: ScalarInterpolation,
    vertical_density_rim_range: ScalarInterpolation,
    custom_color: ColorInterpolation,
    custom_color_start: ScalarInterpolation,
    custom_color_range: ScalarInterpolation,
    custom_amount_scale: ScalarInterpolation,
    custom_amount_scale_start: ScalarInterpolation,
    custom_amount_scale_range: ScalarInterpolation,
    aerial_color_front: ColorInterpolation,
    aerial_color_middle: ColorInterpolation,
    aerial_color_back: ColorInterpolation,
    aerial_final_exp: ScalarInterpolation,
}
// ----------------------------------------------------------------------------
impl FogSettings {
    // ------------------------------------------------------------------------
    pub fn sample(&self, time: &TimeOfDay) -> FogState {
        FogState {
            appear_distance: self.appear_distance.sample(time),
            appear_range: self.appear_range.sample(time),
            color_front: self.color_front.sample(time),
            color_middle: self.color_middle.sample(time),
            color_back: self.color_back.sample(time),
            density: self.density.sample(time),
            final_exp: self.final_exp.sample(time),
            distance_clamp: self.distance_clamp.sample(time),
            vertical_offset: self.vertical_offset.sample(time),
            vertical_density: self.vertical_density.sample(time),
            vertical_density_light_front: self.vertical_density_light_front.sample(time),
            vertical_density_light_back: self.vertical_density_light_back.sample(time),
            vertical_density_rim_range: self.vertical_density_rim_range.sample(time),
            custom_color: self.custom_color.sample(time),
            custom_color_start: self.custom_color_start.sample(time),
            custom_color_range: self.custom_color_range.sample(time),
            custom_amount_scale: self.custom_amount_scale.sample(time),
            custom_amount_scale_start: self.custom_amount_scale_start.sample(time),
            custom_amount_scale_range: self.custom_amount_scale_range.sample(time),
            aerial_color_front: self.aerial_color_front.sample(time),
            aerial_color_middle: self.aerial_color_middle.sample(time),
            aerial_color_back: self.aerial_color_back.sample(time),
            aerial_final_exp: self.aerial_final_exp.sample(time),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// config -> settings
// ----------------------------------------------------------------------------
impl From<EnvironmentConfig> for EnvironmentSettings {
    // ------------------------------------------------------------------------
    fn from(conf: EnvironmentConfig) -> Self {
        Self {
            sun: SunSettings {
                color: ColorInterpolation::from(conf.sun.color),
            },
            fog: FogSettings {
                appear_distance: ScalarInterpolation::from(conf.fog.appear_distance),
                appear_range: ScalarInterpolation::from(conf.fog.appear_range),
                color_front: ColorInterpolation::from(conf.fog.color_front),
                color_middle: ColorInterpolation::from(conf.fog.color_middle),
                color_back: ColorInterpolation::from(conf.fog.color_back),
                density: ScalarInterpolation::from(conf.fog.density),
                final_exp: ScalarInterpolation::from(conf.fog.final_exp),
                distance_clamp: ScalarInterpolation::from(conf.fog.distance_clamp),
                vertical_offset: ScalarInterpolation::from(conf.fog.vertical_offset),
                vertical_density: ScalarInterpolation::from(conf.fog.vertical_density),
                vertical_density_light_front: ScalarInterpolation::from(
                    conf.fog.vertical_density_light_front,
                ),
                vertical_density_light_back: ScalarInterpolation::from(
                    conf.fog.vertical_density_light_back,
                ),
                vertical_density_rim_range: ScalarInterpolation::from(
                    conf.fog.vertical_density_rim_range,
                ),
                custom_color: ColorInterpolation::from(conf.fog.custom_color),
                custom_color_start: ScalarInterpolation::from(conf.fog.custom_color_start),
                custom_color_range: ScalarInterpolation::from(conf.fog.custom_color_range),
                custom_amount_scale: ScalarInterpolation::from(conf.fog.custom_amount_scale),
                custom_amount_scale_start: ScalarInterpolation::from(
                    conf.fog.custom_amount_scale_start,
                ),
                custom_amount_scale_range: ScalarInterpolation::from(
                    conf.fog.custom_amount_scale_range,
                ),
                aerial_color_front: ColorInterpolation::from(conf.fog.aerial_color_front),
                aerial_color_middle: ColorInterpolation::from(conf.fog.aerial_color_middle),
                aerial_color_back: ColorInterpolation::from(conf.fog.aerial_color_back),
                aerial_final_exp: ScalarInterpolation::from(conf.fog.aerial_final_exp),
            },
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
