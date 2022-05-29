//FIXME this is used for a constant sized array in clipmap gpu buffer. can / should this be dynamic?
const MAX_SUPPORTED_CLIPMAP_LEVEL: u8 = 10;
// ----------------------------------------------------------------------------
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    math::{UVec2, Vec2},
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            std140::AsStd140, std140::Std140, BindGroup, BindGroupDescriptor, BindGroupEntry,
            BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
            BufferInitDescriptor, BufferSize, BufferUsages, SamplerBindingType, ShaderStages,
            StorageTextureAccess, TextureFormat, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
    },
};

use crate::resource::{PrepareResourceError, PreparedRenderResource, RenderResource};
use crate::texturearray::TextureArray;

use super::gpu::TerrainMeshRenderPipeline;
use super::{ClipmapInfo, TerrainClipmap};
// ----------------------------------------------------------------------------
// render cmds
// ----------------------------------------------------------------------------
// binds the terrain clipmap data (bkgrnd, overlay, blendcontrol, tintmap)
pub(super) struct SetTerrainClipmapBindGroup<const I: usize>;
// ----------------------------------------------------------------------------
// gpu representation of clipmap data
// ----------------------------------------------------------------------------
pub struct GpuTerrainClipmap {
    pub bind_group: BindGroup,
    _buffer: Buffer,
}
// ----------------------------------------------------------------------------
#[derive(AsStd140)]
pub struct GpuClipmapInfo {
    world_offset: Vec2,
    world_res: f32,
    size: f32,
    info: [GpuClipmapLayerInfo; MAX_SUPPORTED_CLIPMAP_LEVEL as usize],
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone, Copy, Debug, AsStd140)]
pub struct GpuClipmapLayerInfo {
    map_offset: UVec2,
    resolution: f32,
    size: f32,
}
// ----------------------------------------------------------------------------
pub(super) fn clipmap_bind_group_layout() -> [BindGroupLayoutEntry; 4] {
    [
        // texturing controlmap
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::StorageTexture {
                view_dimension: TextureViewDimension::D2Array,
                access: StorageTextureAccess::ReadOnly,
                format: TextureFormat::R16Uint,
            },
            count: None,
        },
        // tintmap texture
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled: false,
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2Array,
            },
            count: None,
        },
        // tintmap texture sampler
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        },
        // current clipmap infos
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::FRAGMENT | ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(GpuClipmapInfo::std140_size_static() as u64),
            },
            count: None,
        },
    ]
}
// ----------------------------------------------------------------------------
// terrain clipmap -> renderresource processing
// ----------------------------------------------------------------------------
impl RenderResource for TerrainClipmap {
    // In RenderStage::Extract step the resource is extracted from "app world" to
    // "render world" into an "ExtractedResource".
    type ExtractedResource = TerrainClipmap;
    // in RenderStage::Prepare step the extracted resource is transformed into its
    // GPU representation "PreparedResource"
    type PreparedResource = GpuTerrainClipmap;
    // defines query for ecs data in the prepare resource step
    type Param = (
        SRes<RenderDevice>,
        SRes<TerrainMeshRenderPipeline>,
        SRes<RenderAssets<TextureArray>>,
    );
    // ------------------------------------------------------------------------
    fn extract_resource(&self) -> Self::ExtractedResource {
        self.clone()
    }
    // ------------------------------------------------------------------------
    fn prepare_resource(
        terrain_clipmap: Self::ExtractedResource,
        (render_device, terrain_pipeline, gpu_arrays): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedResource, PrepareResourceError<Self::ExtractedResource>> {
        let texture_view = if let Some(gpu_array) = gpu_arrays.get(&terrain_clipmap.texture) {
            &gpu_array.texture_view
        } else {
            return Err(PrepareResourceError::RetryNextUpdate(terrain_clipmap));
        };

        let (tint_view, tint_sampler) =
            if let Some(gpu_array) = gpu_arrays.get(&terrain_clipmap.tint) {
                (&gpu_array.texture_view, &gpu_array.sampler)
            } else {
                return Err(PrepareResourceError::RetryNextUpdate(terrain_clipmap));
            };

        let clipmap_info = &terrain_clipmap.clipmap;

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("clipmap_info_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: GpuClipmapInfo::from(clipmap_info).as_std140().as_bytes(),
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(tint_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(tint_sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: buffer.as_entire_binding(),
                },
            ],
            label: Some("clipmap_bind_group"),
            layout: &terrain_pipeline.clipmap_layout,
        });

        Ok(GpuTerrainClipmap {
            bind_group,
            _buffer: buffer,
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// render cmds
// ----------------------------------------------------------------------------
impl<const I: usize> EntityRenderCommand for SetTerrainClipmapBindGroup<I> {
    // ------------------------------------------------------------------------
    type Param = SRes<PreparedRenderResource<TerrainClipmap>>;
    // ------------------------------------------------------------------------
    #[inline]
    fn render<'w>(
        _view: Entity,
        _tile: Entity,
        clipmap: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // since clipmap depends on texture arrays it may not be always ready as
        // fast as the mesh generation
        if let Some(clipmap) = clipmap.into_inner() {
            pass.set_bind_group(I, &clipmap.bind_group, &[]);
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// converter
// ----------------------------------------------------------------------------
impl<'a> From<&'a ClipmapInfo> for GpuClipmapInfo {
    // ------------------------------------------------------------------------
    fn from(clipmap_info: &'a ClipmapInfo) -> Self {
        let mut layer_infos =
            [GpuClipmapLayerInfo::default(); MAX_SUPPORTED_CLIPMAP_LEVEL as usize];

        for (i, layer) in clipmap_info.info.iter().enumerate() {
            layer_infos[i].map_offset = layer.map_offset;
            layer_infos[i].resolution = layer.resolution;
            layer_infos[i].size = layer.size;
        }

        GpuClipmapInfo {
            world_offset: clipmap_info.world_offset,
            world_res: clipmap_info.world_res,
            size: clipmap_info.size as f32,
            info: layer_infos,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
