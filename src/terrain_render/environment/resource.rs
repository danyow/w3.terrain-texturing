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
// gpu representation of environment params
// ----------------------------------------------------------------------------
#[derive(AsStd140, Clone)]
pub struct GpuDirectionalLight {
    color: Vec3,
    direction: Vec3,
}
// ----------------------------------------------------------------------------
pub type GpuTonemappingInfo = Tonemapping;
// ----------------------------------------------------------------------------
pub struct GpuTerrainEnvironment {
    pub sun_buffer: Buffer,
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

        let tonemapping = &environment.tonemapping;
        let tonemapping = tonemapping.clone();

        let tonemapping_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("tonemapping_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: tonemapping.as_std140().as_bytes(),
        });

        Ok(GpuTerrainEnvironment {
            sun_buffer,
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
