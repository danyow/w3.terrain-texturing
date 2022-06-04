// ----------------------------------------------------------------------------
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    render::{
        render_asset::RenderAssets,
        render_resource::{
            std140::AsStd140, std140::Std140, BindGroup, BindGroupDescriptor, BindGroupEntry,
            BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
            BufferInitDescriptor, BufferSize, BufferUsages, ShaderStages, StorageTextureAccess,
            TextureFormat, TextureViewDimension,
        },
        renderer::RenderDevice,
    },
};

use crate::resource::{PrepareResourceError, PreparedRenderResource, RenderResource};
use crate::texturearray::TextureArray;

use super::gpu::{GpuClipmapInfo, GpuTerrainMapInfoSettings};

use super::pipeline::ComputeShadowsPipeline;
use super::{
    ComputeSliceInfo, ComputeThreadJob, DirectionalClipmapLayerInfo, TerrainLightheightClipmap,
    TerrainMapInfo, TerrainShadowsComputeInput, TerrainShadowsLightrayInfo, CLIPMAP_SIZE,
};
// ----------------------------------------------------------------------------
// gpu representation of data required for shadow/lightheight computation
// ----------------------------------------------------------------------------
pub struct GpuTerrainLightheightClipmap {
    pub bind_group: BindGroup,
}
// ----------------------------------------------------------------------------
#[derive(AsStd140, Clone, Copy, Default)]
struct GpuDirectionalClipmapLayerInfo {
    step_1: u32,
    step_after: u32,
    ray_1: u32,
    ray_after: u32,
}
// ----------------------------------------------------------------------------
#[derive(AsStd140)]
struct GpuDirectionalClipmapInfo {
    layer: [GpuDirectionalClipmapLayerInfo; 10],
}
// ----------------------------------------------------------------------------
#[derive(AsStd140, Default, Clone, Copy, Debug)]
struct SliceInfo {
    highest_res_clipmap_level: u32,
    step_after: u32,
    schedule_id: u32,
    _pad: u32, // FIXME
}
// ----------------------------------------------------------------------------
#[derive(AsStd140)]
struct GpuTerrainShadowsComputeSlices {
    // TODO SliceInfo can be packed as one u32
    info: [SliceInfo; 10], // for 5 clipmap level max 10 slices
    count_and_lowest_level: u32,
}
// ----------------------------------------------------------------------------
#[derive(AsStd140, Clone, Copy, Default)]
struct GpuComputeLightheightThreadJobs {
    // FIXME reduce rays + clipmaplevel to one u32
    rays: u32,
    start_ray: u32,
    clipmap_level: u32,
    _pad: u32,
    // start_ray: u16,
    // rays: u8,
    // clipmap_level: u8,
}
// ----------------------------------------------------------------------------
#[derive(AsStd140)]
struct GpuTerrainShadowsThreadJobs {
    // TODO: GpuComputeLightheightThreadJobs can be packed as one u32
    // Note: it seems at the moment fixed size arrays are 16byte aligned?
    slice_jobs: [[GpuComputeLightheightThreadJobs; CLIPMAP_SIZE as usize]; 5],
}
// ----------------------------------------------------------------------------
pub struct GpuTerrainShadowsComputeInput {
    pub bind_group: BindGroup,
    _info_buffer: Buffer,
    _compute_slices_buffer: Buffer,
    _threadjobs_buffer: Buffer,
}
// ----------------------------------------------------------------------------
#[derive(AsStd140)]
struct GpuTerrainShadowsLightrayInfo {
    lightpos_offset: u32,
    interpolation_weight: f32,
    ray_height_delta: f32,
}
// ----------------------------------------------------------------------------
pub struct GpuTerrainShadowsLightray {
    pub bind_group: BindGroup,
    _buffer: Buffer,
}
// ----------------------------------------------------------------------------
pub(super) fn lightheightmap_bind_group_layout() -> [BindGroupLayoutEntry; 2] {
    [
        // lightheight clipmap
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                view_dimension: TextureViewDimension::D2Array,
                access: StorageTextureAccess::ReadWrite,
                format: TextureFormat::R16Uint,
            },
            count: None,
        },
        // map info
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(
                    GpuTerrainMapInfoSettings::std140_size_static() as u64
                ),
            },
            count: None,
        },
    ]
}
// ----------------------------------------------------------------------------
pub(super) fn compute_input_bind_group_layout() -> [BindGroupLayoutEntry; 5] {
    [
        // heightmap clipmap
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                view_dimension: TextureViewDimension::D2Array,
                access: StorageTextureAccess::ReadOnly,
                format: TextureFormat::R16Uint,
            },
            count: None,
        },
        // current clipmap infos
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(GpuClipmapInfo::std140_size_static() as u64),
            },
            count: None,
        },
        // current directional clipmap infos
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(
                    GpuDirectionalClipmapInfo::std140_size_static() as u64
                ),
            },
            count: None,
        },
        // compute clipmap slices
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(
                    GpuTerrainShadowsComputeSlices::std140_size_static() as u64,
                ),
            },
            count: None,
        },
        // compute threads schedule
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
                // BufferSize::new(
                //     GpuComputeLightheightThreadJobs::std140_size_static() as u64,
                // ),
            },
            count: None,
        },
    ]
}
// ----------------------------------------------------------------------------
pub(super) fn lightray_settings_bind_group_layout() -> [BindGroupLayoutEntry; 1] {
    [
        // lightray settings
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(
                    GpuTerrainShadowsLightrayInfo::std140_size_static() as u64,
                ),
            },
            count: None,
        },
    ]
}
// ----------------------------------------------------------------------------
// terrain lightheight clipmap -> renderresource processing
// ----------------------------------------------------------------------------
impl RenderResource for TerrainLightheightClipmap {
    // In RenderStage::Extract step the resource is extracted from "app world" to
    // "render world" into an "ExtractedResource".
    type ExtractedResource = TerrainLightheightClipmap;
    // in RenderStage::Prepare step the extracted resource is transformed into its
    // GPU representation "PreparedResource"
    type PreparedResource = GpuTerrainLightheightClipmap;
    // defines query for ecs data in the prepare resource step
    type Param = (
        SRes<RenderDevice>,
        SRes<ComputeShadowsPipeline>,
        SRes<RenderAssets<TextureArray>>,
        SRes<PreparedRenderResource<TerrainMapInfo>>,
    );
    // ------------------------------------------------------------------------
    fn extract_resource(&self) -> Self::ExtractedResource {
        self.clone()
    }
    // ------------------------------------------------------------------------
    fn prepare_resource(
        clipmap: Self::ExtractedResource,
        (render_device, terrain_pipeline, gpu_arrays, gpu_mapinfo): &mut SystemParamItem<
            Self::Param,
        >,
    ) -> Result<Self::PreparedResource, PrepareResourceError<Self::ExtractedResource>> {
        let lightheight_view = if let Some(gpu_array) = gpu_arrays.get(&clipmap.lightheight) {
            &gpu_array.texture_view
        } else {
            return Err(PrepareResourceError::RetryNextUpdate(clipmap));
        };

        // map info
        let info = if let Some(gpu_info) = gpu_mapinfo.as_ref() {
            gpu_info
        } else {
            return Err(PrepareResourceError::RetryNextUpdate(clipmap));
        };

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(lightheight_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: info.as_entire_binding(),
                },
            ],
            label: Some("terrain_lightheight_clipmap_bind_group"),
            layout: &terrain_pipeline.lightheightmap_layout,
        });

        Ok(GpuTerrainLightheightClipmap { bind_group })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl RenderResource for TerrainShadowsComputeInput {
    // In RenderStage::Extract step the resource is extracted from "app world" to
    // "render world" into an "ExtractedResource".
    type ExtractedResource = TerrainShadowsComputeInput;
    // in RenderStage::Prepare step the extracted resource is transformed into its
    // GPU representation "PreparedResource"
    type PreparedResource = GpuTerrainShadowsComputeInput;
    // defines query for ecs data in the prepare resource step
    type Param = (
        SRes<RenderDevice>,
        SRes<ComputeShadowsPipeline>,
        SRes<RenderAssets<TextureArray>>,
    );
    // ------------------------------------------------------------------------
    fn extract_resource(&self) -> Self::ExtractedResource {
        self.clone()
    }
    // ------------------------------------------------------------------------
    fn prepare_resource(
        compute_input: Self::ExtractedResource,
        (render_device, terrain_pipeline, gpu_arrays): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedResource, PrepareResourceError<Self::ExtractedResource>> {
        let heightmap_view = if let Some(gpu_array) = gpu_arrays.get(&compute_input.heightmap) {
            &gpu_array.texture_view
        } else {
            return Err(PrepareResourceError::RetryNextUpdate(compute_input));
        };

        let clipmap = &compute_input.clipmap_info;

        let info_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("compute_terrain_lightheight_clipmap_info_buffer"),
            usage: BufferUsages::UNIFORM, // | BufferUsages::COPY_DST,
            contents: GpuClipmapInfo::from(clipmap).as_std140().as_bytes(),
        });

        let directional_clipmap_info_buffer =
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("compute_terrain_lightheight_directional_clipmap_info_buffer"),
                usage: BufferUsages::UNIFORM,
                contents: GpuDirectionalClipmapInfo::from(
                    compute_input.directional_clipmap_layer.as_slice(),
                )
                .as_std140()
                .as_bytes(),
            });

        let compute_slices_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("compute_terrain_lightheight_clipmap_slices_buffer"),
            usage: BufferUsages::UNIFORM,
            contents: GpuTerrainShadowsComputeSlices::from(compute_input.compute_slices.as_slice())
                .as_std140()
                .as_bytes(),
        });

        let threadjobs_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("compute_terrain_lightheight_threadjobs_buffer"),
            usage: BufferUsages::STORAGE,
            contents: GpuTerrainShadowsThreadJobs::from(compute_input.thread_jobs.as_slice())
                .as_std140()
                .as_bytes(),
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(heightmap_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: info_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: directional_clipmap_info_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: compute_slices_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: threadjobs_buffer.as_entire_binding(),
                },
            ],
            label: Some("terrain_lightheight_compute_input_bind_group"),
            layout: &terrain_pipeline.input_layout,
        });

        Ok(GpuTerrainShadowsComputeInput {
            bind_group,
            _info_buffer: info_buffer,
            _compute_slices_buffer: compute_slices_buffer,
            _threadjobs_buffer: threadjobs_buffer,
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl RenderResource for TerrainShadowsLightrayInfo {
    // In RenderStage::Extract step the resource is extracted from "app world" to
    // "render world" into an "ExtractedResource".
    type ExtractedResource = TerrainShadowsLightrayInfo;
    // in RenderStage::Prepare step the extracted resource is transformed into its
    // GPU representation "PreparedResource"
    type PreparedResource = GpuTerrainShadowsLightray;
    // defines query for ecs data in the prepare resource step
    type Param = (SRes<RenderDevice>, SRes<ComputeShadowsPipeline>);
    // ------------------------------------------------------------------------
    fn extract_resource(&self) -> Self::ExtractedResource {
        self.clone()
    }
    // ------------------------------------------------------------------------
    fn prepare_resource(
        ray_info: Self::ExtractedResource,
        (render_device, terrain_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedResource, PrepareResourceError<Self::ExtractedResource>> {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("compute_terrain_lightheight_ray_info_buffer"),
            usage: BufferUsages::UNIFORM,
            contents: GpuTerrainShadowsLightrayInfo::from(&ray_info)
                .as_std140()
                .as_bytes(),
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("compute_terrain_lightheight_rayinfo_bind_group"),
            layout: &terrain_pipeline.lightray_layout,
        });

        Ok(GpuTerrainShadowsLightray {
            bind_group,
            _buffer: buffer,
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// converter
// ----------------------------------------------------------------------------
impl<'a> From<&'a [DirectionalClipmapLayerInfo]> for GpuDirectionalClipmapInfo {
    // ------------------------------------------------------------------------
    fn from(info: &'a [DirectionalClipmapLayerInfo]) -> Self {
        let mut layer = [GpuDirectionalClipmapLayerInfo::default(); 10];

        for (i, info) in info.iter().enumerate() {
            layer[i].step_1 = info.step_1 as u32;
            layer[i].step_after = info.step_after as u32;
            layer[i].ray_1 = info.ray_1 as u32;
            layer[i].ray_after = info.ray_after as u32;
        }

        Self { layer }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<'a> From<&'a [ComputeSliceInfo]> for GpuTerrainShadowsComputeSlices {
    // ------------------------------------------------------------------------
    fn from(slices: &'a [ComputeSliceInfo]) -> Self {
        let mut info = [SliceInfo::default(); 10];

        let mut lowest_clipmap_level = 0;

        for (i, slice) in slices.iter().enumerate() {
            info[i] = SliceInfo {
                highest_res_clipmap_level: slice.highest_res_clipmap_level,
                step_after: slice.step_after,
                schedule_id: slice.schedule_id,
                _pad: 0,
            };
            lowest_clipmap_level = lowest_clipmap_level.max(slice.highest_res_clipmap_level);
        }

        Self {
            info,
            count_and_lowest_level: (lowest_clipmap_level << 16) + (slices.len() as u32),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<'a> From<&'a [Vec<ComputeThreadJob>]> for GpuTerrainShadowsThreadJobs {
    // ------------------------------------------------------------------------
    fn from(slices: &'a [Vec<ComputeThreadJob>]) -> Self {
        let mut slice_jobs =
            [[GpuComputeLightheightThreadJobs::default(); CLIPMAP_SIZE as usize]; 5];

        for (s, slice) in slices.iter().enumerate() {
            for (thread, jobs) in slice.iter().enumerate() {
                slice_jobs[s][thread] = GpuComputeLightheightThreadJobs {
                    rays: jobs.rays,
                    start_ray: jobs.start_ray,
                    clipmap_level: jobs.clipmap_level,
                    _pad: 0,
                };
            }
        }
        Self { slice_jobs }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<'a> From<&'a TerrainShadowsLightrayInfo> for GpuTerrainShadowsLightrayInfo {
    // ------------------------------------------------------------------------
    fn from(info: &'a TerrainShadowsLightrayInfo) -> Self {
        Self {
            lightpos_offset: info.lightpos_offset,
            interpolation_weight: info.interpolation_weight,
            ray_height_delta: info.ray_height_delta,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
