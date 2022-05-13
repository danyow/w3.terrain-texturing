// ----------------------------------------------------------------------------
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    render::{
        render_resource::{
            std140::{AsStd140, Std140},
            Buffer, BufferInitDescriptor, BufferUsages,
        },
        renderer::RenderDevice,
    },
};

use crate::resource::{PrepareResourceError, RenderResource};

use super::EnvironmentData;
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct DirectionalLight {
    pub color: Color,
    pub direction: Vec3,
}
// ----------------------------------------------------------------------------
#[derive(AsStd140, Clone)]
pub struct Tonemapping {
    pub luminance_min: f32,
    pub luminance_max: f32,
    pub luminance_limit_shape: f32,
    pub shoulder_strength: f32,
    pub linear_strength: f32,
    pub linear_angle: f32,
    pub toe_strength: f32,
    pub toe_numerator: f32,
    pub toe_denumerator: f32,
    pub exposure_scale: f32,
    pub post_scale: f32,
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub struct FogState {
    pub appear_distance: f32,
    pub appear_range: f32,
    pub color_front: Color,
    pub color_middle: Color,
    pub color_back: Color,
    pub density: f32,
    pub final_exp: f32,
    pub distance_clamp: f32,
    pub vertical_offset: f32,
    pub vertical_density: f32,
    pub vertical_density_light_front: f32,
    pub vertical_density_light_back: f32,
    //pub sky_density_scale: f32,
    //pub clouds_density_scale: f32,
    //pub sky_vertical_density_light_front_scale: f32,
    //pub sky_vertical_density_light_back_scale: f32,
    pub vertical_density_rim_range: f32,
    pub custom_color: Color,
    pub custom_color_start: f32,
    pub custom_color_range: f32,
    pub custom_amount_scale: f32,
    pub custom_amount_scale_start: f32,
    pub custom_amount_scale_range: f32,
    pub aerial_color_front: Color,
    pub aerial_color_middle: Color,
    pub aerial_color_back: Color,
    pub aerial_final_exp: f32,
}
// ----------------------------------------------------------------------------
// gpu representation of environment params
// ----------------------------------------------------------------------------
#[derive(AsStd140, Clone)]
pub struct GpuDirectionalLight {
    color: Vec3,
    direction: Vec3,
}
// ----------------------------------------------------------------------------
#[derive(AsStd140)]
struct GpuFogColor {
    front: Vec3,
    middle: Vec3,
    back: Vec3,
    final_exp: f32,
}
// ----------------------------------------------------------------------------
#[derive(AsStd140)]
struct GpuCustomFogSettings {
    color: Vec3,
    color_scale: f32,
    color_bias: f32,
    amount: f32,
    amount_scale: f32,
    amount_bias: f32,
}
// ----------------------------------------------------------------------------
#[derive(AsStd140)]
struct GpuVerticalFogDensity {
    offset: f32,
    front: f32,
    back: f32,
    rim_range: f32,
}
// ----------------------------------------------------------------------------
#[derive(AsStd140)]
pub struct GpuFogSettings {
    appear_distance: f32,
    appear_scale: f32,
    distance_clamp: f32,
    density: f32,
    vertical_density: GpuVerticalFogDensity,
    color: GpuFogColor,
    aerial_color: GpuFogColor,
    custom: GpuCustomFogSettings,
}
// ----------------------------------------------------------------------------
pub type GpuTonemappingInfo = Tonemapping;
// ----------------------------------------------------------------------------
pub struct GpuTerrainEnvironment {
    pub sun_buffer: Buffer,
    pub fog_buffer: Buffer,
    pub tonemapping_buffer: Buffer,
}
// ----------------------------------------------------------------------------
// terrain environment -> renderresource processing
// ----------------------------------------------------------------------------
impl RenderResource for EnvironmentData {
    // In RenderStage::Extract step the resource is extracted from "app world" to
    // "render world" into an "ExtractedResource".
    type ExtractedResource = EnvironmentData;
    // in RenderStage::Prepare step the extracted resource is transformed into its
    // GPU representation "PreparedResource"
    type PreparedResource = GpuTerrainEnvironment;
    // defines query for ecs data in the prepare resource step
    type Param = SRes<RenderDevice>;
    // ------------------------------------------------------------------------
    fn extract_resource(&self) -> Self::ExtractedResource {
        self.clone()
    }
    // ------------------------------------------------------------------------
    fn prepare_resource(
        environment: Self::ExtractedResource,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedResource, PrepareResourceError<Self::ExtractedResource>> {
        let sun = &environment.sun;
        let sun = GpuDirectionalLight {
            color: Vec3::from_slice(&sun.color.as_rgba_f32()),
            direction: sun.direction,
        };

        let sun_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("sunlight_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: sun.as_std140().as_bytes(),
        });

        let fogsettings = GpuFogSettings::from(&environment.fog);

        let fog_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("fog_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: fogsettings.as_std140().as_bytes(),
        });

        let tonemapping = &environment.tonemapping;
        let tonemapping = tonemapping.clone();

        let tonemapping_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("tonemapping_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: tonemapping.as_std140().as_bytes(),
        });

        Ok(GpuTerrainEnvironment {
            sun_buffer,
            fog_buffer,
            tonemapping_buffer,
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// Defaults
// ----------------------------------------------------------------------------
impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            color: Color::rgb(1.0, 1.0, 1.0),
            direction: Vec3::new(0.0, -1.0, 0.0),
        }
    }
}
// ----------------------------------------------------------------------------
#[allow(clippy::excessive_precision)]
impl Default for Tonemapping {
    fn default() -> Self {
        Self {
            // taken from default KM env
            luminance_min: 0.0,
            luminance_max: 1.6964925528,
            luminance_limit_shape: 0.3946479857,

            shoulder_strength: 0.3000000119,
            linear_strength: 0.3000000119,
            linear_angle: 0.1000000015,
            toe_strength: 0.200000003,
            toe_numerator: 0.0099999998,
            toe_denumerator: 0.3000000119,

            exposure_scale: 0.7504349947,
            post_scale: 1.1663999557,
        }
    }
}
// ----------------------------------------------------------------------------
// Conversion
// ----------------------------------------------------------------------------
impl<'a> From<&'a FogState> for GpuFogSettings {
    fn from(s: &'a FogState) -> Self {
        Self {
            appear_distance: s.appear_distance,
            appear_scale: 1.0 / s.appear_range,
            distance_clamp: s.distance_clamp,
            density: s.density,
            vertical_density: GpuVerticalFogDensity {
                offset: s.vertical_offset,
                front: -(s.vertical_density / s.vertical_density_light_front),
                back: -(s.vertical_density / s.vertical_density_light_back),
                rim_range: s.vertical_density_rim_range,
            },
            color: GpuFogColor {
                front: Vec3::from_slice(&s.color_front.as_rgba_f32()),
                middle: Vec3::from_slice(&s.color_middle.as_rgba_f32()),
                back: Vec3::from_slice(&s.color_back.as_rgba_f32()),
                final_exp: s.final_exp,
            },
            aerial_color: GpuFogColor {
                front: Vec3::from_slice(&s.aerial_color_front.as_rgba_f32()),
                middle: Vec3::from_slice(&s.aerial_color_middle.as_rgba_f32()),
                back: Vec3::from_slice(&s.aerial_color_back.as_rgba_f32()),
                final_exp: s.aerial_final_exp,
            },
            custom: GpuCustomFogSettings {
                color: Vec3::from_slice(&s.custom_color.as_rgba_f32()),
                color_scale: 1.0 / s.custom_color_range,
                color_bias: -(s.custom_color_start / s.custom_color_range),
                amount: s.custom_amount_scale,
                amount_scale: 1.0 / s.custom_amount_scale_range,
                amount_bias: -(s.custom_amount_scale_start / s.custom_amount_scale_range),
            },
        }
    }
}
// ----------------------------------------------------------------------------
