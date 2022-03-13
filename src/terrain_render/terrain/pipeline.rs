// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroupLayout, BindGroupLayoutDescriptor, BlendState, ColorTargetState, ColorWrites,
            CompareFunction, DepthBiasState, DepthStencilState, Face, FragmentState, FrontFace,
            MultisampleState, PolygonMode, PrimitiveState, PrimitiveTopology,
            RenderPipelineDescriptor, SpecializedPipeline, StencilFaceState, StencilState,
            TextureFormat, VertexState,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
    },
};

use super::terrain_clipmap::clipmap_bind_group_layout;
use super::terrain_material::materialset_bind_group_layout;
use super::terrain_mesh::{
    mesh_bind_group_layout, mesh_vertex_buffer_layout, mesh_view_bind_group_layout,
};
// ----------------------------------------------------------------------------
// pipeline
// ----------------------------------------------------------------------------
pub struct TerrainMeshRenderPipeline {
    shader_vert: Handle<Shader>,
    shader_frag: Handle<Shader>,

    pub(super) view_layout: BindGroupLayout,
    pub(super) mesh_layout: BindGroupLayout,
    pub(super) materialset_layout: BindGroupLayout,
    pub(super) clipmap_layout: BindGroupLayout,
}
// ----------------------------------------------------------------------------
impl FromWorld for TerrainMeshRenderPipeline {
    // ------------------------------------------------------------------------
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        // needs to be activated before shaders are loaded!
        asset_server.watch_for_changes().unwrap();

        let shader_vert = asset_server.load("shaders/terrain_vert.wgsl");
        let shader_frag = asset_server.load("shaders/terrain_frag.wgsl");

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("terrain_mesh_view_layout"),
            entries: &mesh_view_bind_group_layout(),
        });

        let mesh_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("terrain_mesh_layout"),
            entries: &mesh_bind_group_layout(),
        });

        let materialset_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("terrain_material_layout"),
                entries: &materialset_bind_group_layout(),
            });

        let clipmap_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("terrain_clipmap_layout"),
            entries: &clipmap_bind_group_layout(),
        });

        TerrainMeshRenderPipeline {
            shader_vert,
            shader_frag,

            view_layout,
            mesh_layout,
            materialset_layout,
            clipmap_layout,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
bitflags::bitflags! {
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    /// MSAA uses the highest 6 bits for the MSAA sample count - 1 to support up to 64x MSAA.
    pub struct TerrainMeshPipelineKey: u32 {
        const NONE               = 0;
        const MSAA_RESERVED_BITS = TerrainMeshPipelineKey::MSAA_MASK_BITS << TerrainMeshPipelineKey::MSAA_SHIFT_BITS;
    }
}
// ----------------------------------------------------------------------------
impl TerrainMeshPipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;
    // ------------------------------------------------------------------------
    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = ((msaa_samples - 1) & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        TerrainMeshPipelineKey::from_bits(msaa_bits).unwrap()
    }
    // ------------------------------------------------------------------------
    fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl SpecializedPipeline for TerrainMeshRenderPipeline {
    // ------------------------------------------------------------------------
    type Key = TerrainMeshPipelineKey;
    // ------------------------------------------------------------------------
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        // Note: transparency and opaque passes - terrain is always opaque
        let label = "terrain_mesh_pipeline".into();
        let blend = Some(BlendState::REPLACE);
        let depth_write_enabled = true;

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader_vert.clone(),
                entry_point: "vertex".into(),
                shader_defs: Vec::default(),
                buffers: vec![mesh_vertex_buffer_layout(key)],
            },
            fragment: Some(FragmentState {
                shader: self.shader_frag.clone(),
                entry_point: "fragment".into(),
                shader_defs: Vec::default(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend,
                    write_mask: ColorWrites::ALL,
                }],
            }),
            layout: Some(vec![
                self.view_layout.clone(),
                self.mesh_layout.clone(),
                self.materialset_layout.clone(),
                self.clipmap_layout.clone(),
            ]),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some(label),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
