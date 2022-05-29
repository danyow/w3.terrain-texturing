// ----------------------------------------------------------------------------
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    render::{
        render_resource::{
            std140::{AsStd140, Std140},
            Buffer, BufferInitDescriptor, BufferUsages,
        },
        renderer::RenderDevice,
    },
};

use crate::resource::{PrepareResourceError, RenderResource};

use super::TerrainMapInfo;
// ----------------------------------------------------------------------------
// gpu representation of terrain map settings
// ----------------------------------------------------------------------------
#[derive(AsStd140, Clone)]
pub(super) struct GpuTerrainMapInfoSettings {
    size_and_clipmap_level_count: u32,
    resolution: f32,
    height_min: f32,
    height_max: f32,
    height_scaling: f32,
}
// ----------------------------------------------------------------------------
pub struct GpuTerrainMapInfo(Buffer);
// ----------------------------------------------------------------------------
// terrain environment -> renderresource processing
// ----------------------------------------------------------------------------
impl RenderResource for TerrainMapInfo {
    // In RenderStage::Extract step the resource is extracted from "app world" to
    // "render world" into an "ExtractedResource".
    type ExtractedResource = TerrainMapInfo;
    // in RenderStage::Prepare step the extracted resource is transformed into its
    // GPU representation "PreparedResource"
    type PreparedResource = GpuTerrainMapInfo;
    // defines query for ecs data in the prepare resource step
    type Param = SRes<RenderDevice>;
    // ------------------------------------------------------------------------
    fn extract_resource(&self) -> Self::ExtractedResource {
        self.clone()
    }
    // ------------------------------------------------------------------------
    fn prepare_resource(
        map_info: Self::ExtractedResource,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedResource, PrepareResourceError<Self::ExtractedResource>> {
        let settings_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("terrain_map_info_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: GpuTerrainMapInfoSettings::from(map_info).as_std140().as_bytes(),
        });

        Ok(GpuTerrainMapInfo(settings_buffer))
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<TerrainMapInfo> for GpuTerrainMapInfoSettings {
    // ------------------------------------------------------------------------
    fn from(info: TerrainMapInfo) -> Self {
        Self {
            size_and_clipmap_level_count: info.map_size << 16 & info.clipmap_level_count as u32,
            resolution: info.resolution,
            height_min: info.height_min,
            height_max: info.height_max,
            height_scaling: (info.height_max - info.height_min) / u16::MAX as f32,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl std::ops::Deref for GpuTerrainMapInfo {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
// ----------------------------------------------------------------------------
