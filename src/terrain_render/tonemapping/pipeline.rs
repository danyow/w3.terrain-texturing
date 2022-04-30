// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
            ColorTargetState, ColorWrites, FragmentState, MultisampleState, PrimitiveState,
            RenderPipelineDescriptor, ShaderStages, SpecializedPipeline, TextureFormat,
            TextureSampleType, TextureViewDimension, VertexState,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
    },
};
// ----------------------------------------------------------------------------
pub struct TonemappingRenderPipeline {
    shader_vert: Handle<Shader>,
    shader_frag: Handle<Shader>,

    pub(super) input_layout: BindGroupLayout,
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

        Self {
            shader_vert,
            shader_frag,
            input_layout,
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
            layout: Some(vec![self.input_layout.clone()]),
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
