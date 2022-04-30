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

use super::TerrainEnvironment;
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct DirectionalLight {
    pub color: Color,
    pub brightness: f32,
    pub direction: Vec3,
}
// ----------------------------------------------------------------------------
// gpu representation of environment params
// ----------------------------------------------------------------------------
#[derive(AsStd140, Clone)]
pub struct GpuDirectionalLight {
    color: Vec3,
    brightness: f32,
    direction: Vec3,
}
// ----------------------------------------------------------------------------
pub struct GpuTerrainEnvironment {
    pub sun_buffer: Buffer,
}
// ----------------------------------------------------------------------------
// terrain environment -> renderresource processing
// ----------------------------------------------------------------------------
impl RenderResource for TerrainEnvironment {
    // In RenderStage::Extract step the resource is extracted from "app world" to
    // "render world" into an "ExtractedResource".
    type ExtractedResource = TerrainEnvironment;
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
            color: Vec3::from_slice(&sun.color.as_linear_rgba_f32()),
            brightness: sun.brightness,
            direction: sun.direction,
        };

        let sun_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("sunlight_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: sun.as_std140().as_bytes(),
        });

        Ok(GpuTerrainEnvironment {
            sun_buffer,
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
            brightness: 0.5,
            direction: Vec3::new(0.0, 1.0, 0.0),
        }
    }
}
// ----------------------------------------------------------------------------
