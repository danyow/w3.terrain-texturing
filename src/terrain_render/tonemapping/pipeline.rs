// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_resource::{
            std140::AsStd140, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
            BindingType, BufferBindingType, BufferSize, ColorTargetState, ColorWrites,
            FragmentState, MultisampleState, PrimitiveState, RenderPipelineDescriptor,
            ShaderStages, SpecializedPipeline, TextureFormat, TextureSampleType,
            TextureViewDimension, VertexState,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
    },
};

use super::GpuTonemappingInfo;
// ----------------------------------------------------------------------------
pub struct TonemappingRenderPipeline {
    shader_vert: Handle<Shader>,
    shader_frag: Handle<Shader>,

    pub(super) input_layout: BindGroupLayout,
    pub(super) info_layout: BindGroupLayout,
}
// ----------------------------------------------------------------------------
impl FromWorld for TonemappingRenderPipeline {
    // ------------------------------------------------------------------------
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let shader_vert = asset_server.load("shaders/tonemapping_vert.wgsl");
        let shader_frag = asset_server.load("shaders/tonemapping_frag.wgsl");

        let input_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("tonemapping_input_layout"),
            entries: &[
                // hdr texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        // multisample must be deactivated to load data from texture
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let info_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("tonemapping_info_layout"),
            entries: &[
                // Tonemapping settings
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            GpuTonemappingInfo::std140_size_static() as u64
                        ),
                    },
                    count: None,
                },
            ],
        });

        Self {
            shader_vert,
            shader_frag,
            input_layout,
            info_layout,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl SpecializedPipeline for TonemappingRenderPipeline {
    type Key = ();
    // ------------------------------------------------------------------------
    fn specialize(&self, _: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("tonemapping_pipeline".into()),
            layout: Some(vec![self.input_layout.clone(), self.info_layout.clone()]),
            vertex: VertexState {
                shader: self.shader_vert.clone(),
                entry_point: "vertex".into(),
                shader_defs: vec![],
                // empty buffer. vertex shader will generate full screen triangle
                buffers: Vec::default(),
            },
            fragment: Some(FragmentState {
                shader: self.shader_frag.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState {
                // no multisampling!
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        }
    }
}
// ----------------------------------------------------------------------------
