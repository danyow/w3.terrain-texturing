// ----------------------------------------------------------------------------
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            std140::{AsStd140, Std140},
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer,
            BufferInitDescriptor, BufferUsages, BindGroupLayoutEntry, ShaderStages, BindingType, TextureSampleType, SamplerBindingType, TextureViewDimension, BufferSize, BufferBindingType,
        },
        renderer::RenderDevice,
    },
};

use crate::resource::{PrepareResourceError, PreparedRenderResource, RenderResource};
use crate::texturearray::TextureArray;

use super::{TerrainMaterialParam, TerrainMaterialSet, TerrainMeshRenderPipeline};
// ----------------------------------------------------------------------------
// render cmds
// ----------------------------------------------------------------------------
/// binds the global terrain material data (texture arrays and material parameters)
pub struct SetTerrainMaterialSetBindGroup<const I: usize>;
// ----------------------------------------------------------------------------
// gpu representation of materialset and params
// ----------------------------------------------------------------------------
pub struct GpuTerrainMaterialSet {
    bind_group: BindGroup,
    _parameters: Buffer,
}
// ----------------------------------------------------------------------------
#[derive(Default, Copy, Clone, AsStd140)]
struct GpuTerrainMaterialParam {
    blend_sharpness: f32,
    slope_base_dampening: f32,
    slope_normal_dampening: f32,
    specularity_scale: f32,
    specularity: f32,
    specularity_base: f32,
    _specularity_scale_copy: f32,
    falloff: f32,
}
// ----------------------------------------------------------------------------
#[derive(Default, AsStd140)]
struct GpuMaterialParamData {
    data: [GpuTerrainMaterialParam; 31],
}
// ----------------------------------------------------------------------------
pub(super) fn materialset_bind_group_layout() -> [BindGroupLayoutEntry; 5] {
    [
        // diffuse color texture
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled: false,
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2Array,
            },
            count: None,
        },
        // diffuse color texture Sampler
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        },
        // normal texture
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled: false,
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2Array,
            },
            count: None,
        },
        // normal texture Sampler
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        },
        // materialparameter
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(GpuMaterialParamData::std140_size_static() as u64),
            },
            count: None,
        },
    ]
}
// ----------------------------------------------------------------------------
// materialset -> renderresource processing
// ----------------------------------------------------------------------------
impl RenderResource for TerrainMaterialSet {
    // In RenderStage::Extract step the asset is extracted from "app world" to
    // "render world" into an "ExtractedAsset".
    type ExtractedResource = TerrainMaterialSet;
    // in RenderStage::Prepare step the extracted asset is transformed into its
    // GPU representation "PreparedAsset"
    type PreparedResource = GpuTerrainMaterialSet;
    // defines query for ecs data in the prepare asset step
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
        material_data: Self::ExtractedResource,
        (render_device, terrain_pipeline, gpu_tex_arrays): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedResource, PrepareResourceError<Self::ExtractedResource>> {
        let (texture_view, texture_sampler) =
            if let Some(gpu_image) = gpu_tex_arrays.get(&material_data.diffuse) {
                (&gpu_image.texture_view, &gpu_image.sampler)
            } else {
                return Err(PrepareResourceError::RetryNextUpdate(material_data));
            };

        let (normal_view, normal_sampler) =
            if let Some(gpu_image) = gpu_tex_arrays.get(&material_data.normal) {
                (&gpu_image.texture_view, &gpu_image.sampler)
            } else {
                return Err(PrepareResourceError::RetryNextUpdate(material_data));
            };

        let mut param_buffer = GpuMaterialParamData::default();

        for (i, material_param) in material_data.parameter.iter().enumerate() {
            param_buffer.data[i] = GpuTerrainMaterialParam::from(material_param);
        }

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("terrain_materialset_param_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: param_buffer.as_std140().as_bytes(),
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(texture_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(normal_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(normal_sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: buffer.as_entire_binding(),
                },
            ],
            label: Some("terrain_materialset_bind_group"),
            layout: &terrain_pipeline.materialset_layout,
        });

        Ok(GpuTerrainMaterialSet {
            bind_group,
            _parameters: buffer,
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// render cmds
// ----------------------------------------------------------------------------
impl<const I: usize> EntityRenderCommand for SetTerrainMaterialSetBindGroup<I> {
    // ------------------------------------------------------------------------
    type Param = SRes<PreparedRenderResource<TerrainMaterialSet>>;
    // ------------------------------------------------------------------------
    #[inline]
    fn render<'w>(
        _view: Entity,
        _tile: Entity,
        materialset: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // called every frame
        let bindgroup = materialset
            .into_inner()
            .as_ref()
            .map(|ms| &ms.bind_group)
            .unwrap();

        pass.set_bind_group(I, bindgroup, &[]);

        RenderCommandResult::Success
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// material params
// ----------------------------------------------------------------------------
impl Default for TerrainMaterialParam {
    // ------------------------------------------------------------------------
    fn default() -> Self {
        Self {
            blend_sharpness: 0.5,
            slope_base_dampening: 0.5,
            slope_normal_dampening: 0.5,
            specularity_scale: 0.0,
            specularity: 0.0,
            specularity_base: 0.0,
            _specularity_scale_copy: 0.0,
            falloff: 0.0,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<&TerrainMaterialParam> for GpuTerrainMaterialParam {
    // ------------------------------------------------------------------------
    fn from(s: &TerrainMaterialParam) -> Self {
        Self {
            blend_sharpness: s.blend_sharpness,
            slope_base_dampening: s.slope_base_dampening,
            slope_normal_dampening: s.slope_normal_dampening,
            specularity_scale: s.specularity_scale,
            specularity: s.specularity,
            specularity_base: s.specularity_base,
            _specularity_scale_copy: s._specularity_scale_copy,
            falloff: s.falloff,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
