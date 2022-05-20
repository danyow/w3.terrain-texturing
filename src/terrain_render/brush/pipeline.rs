// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_resource::{
            std140::AsStd140, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
            BindingType, BlendState, BufferBindingType, BufferSize, ColorTargetState, ColorWrites,
            Face, FragmentState, FrontFace, MultisampleState, PolygonMode, PrimitiveState,
            PrimitiveTopology, RenderPipelineDescriptor, SamplerBindingType, ShaderStages,
            SpecializedRenderPipeline, TextureFormat, TextureSampleType, TextureViewDimension,
            VertexState,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
    },
};

use super::{GpuBrushPointer, GpuBrushPointerInfo};
// ----------------------------------------------------------------------------
pub struct BrushPointerRenderPipeline {
    shader_vert: Handle<Shader>,
    shader_frag: Handle<Shader>,

    pub info_layout: BindGroupLayout,
    pub input_layout: BindGroupLayout,
    pub result_layout: BindGroupLayout,
}
// ----------------------------------------------------------------------------
impl FromWorld for BrushPointerRenderPipeline {
    // ------------------------------------------------------------------------
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let shader_vert = asset_server.load("shaders/brush_vert.wgsl");
        let shader_frag = asset_server.load("shaders/brush_frag.wgsl");

        let info_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("brushpointer_info_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(
                        GpuBrushPointerInfo::std140_size_static() as u64
                    ),
                },
                count: None,
            }],
        });

        let result_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("brushpointer_result_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let input_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("brushpointer_input_layout"),
            entries: &[
                // world pos
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
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        Self {
            shader_vert,
            shader_frag,
            info_layout,
            input_layout,
            result_layout,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct BrushPointerPipelineKey {
    request_result: bool,
}
// ----------------------------------------------------------------------------
impl BrushPointerPipelineKey {
    // ------------------------------------------------------------------------
    pub(super) fn from_brush(brush: &GpuBrushPointer) -> Self {
        Self {
            request_result: brush.request_result,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl SpecializedRenderPipeline for BrushPointerRenderPipeline {
    // ------------------------------------------------------------------------
    type Key = BrushPointerPipelineKey;
    // ------------------------------------------------------------------------
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let label = "brush_pipeline".into();
        let blend = Some(BlendState::ALPHA_BLENDING);

        // if result was requested (e.g. any mouse click) store it in special buffer
        let (shader_defs, layout) = if key.request_result {
            (
                vec!["STORE_RESULT".to_string()],
                vec![
                    self.input_layout.clone(),
                    self.info_layout.clone(),
                    self.result_layout.clone(),
                ],
            )
        } else {
            (
                Vec::default(),
                vec![self.input_layout.clone(), self.info_layout.clone()],
            )
        };

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader_vert.clone(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                // empty buffer. vertex shader will generate full screen triangle
                buffers: Vec::default(),
            },
            fragment: Some(FragmentState {
                shader: self.shader_frag.clone(),
                entry_point: "fragment".into(),
                shader_defs,
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend,
                    write_mask: ColorWrites::ALL,
                }],
            }),
            layout: Some(layout),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                // no multisampling!
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some(label),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
