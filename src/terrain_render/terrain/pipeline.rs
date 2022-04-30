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

use super::TerrainRenderSettings;
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
    pub struct TerrainMeshPipelineKey: u32 {
        const NONE                  = 0b0000_0000_0000_0000;
        const FLAT_SHADING          = 0b0000_0000_0000_0001;
        const SHOW_WIREFRAME        = 0b0000_0000_0000_0010;
        const SHOW_CLIPMAP_LEVEL    = 0b0000_0000_0000_0100;

        const HIDE_OVERLAY_TEXTURE  = 0b0000_0000_0001_0000;
        const HIDE_BKGRND_TEXTURE   = 0b0000_0000_0010_0000;
        const IGNORE_TINT_MAP       = 0b0000_0000_0100_0000;

        // exclusive: will always override
        const EXCLUSIVE_OVERRIDE    = 0b1000_0000_0000_0000;

        // Note: order is important (see shader_defs check)
        const SHOW_FRAGMENT_NORMAL  = 0b1000_0000_0001_0000;
        const SHOW_COMBINED_NORMAL  = 0b1000_0000_0010_0000;
        const SHOW_BLEND_VALUE      = 0b1000_0000_0011_0000;
        const SHOW_UV_SCALING       = 0b1000_0000_0100_0000;
        const SHOW_TINT_MAP         = 0b1000_0000_0101_0000;
    }
}
// ----------------------------------------------------------------------------
impl TerrainMeshPipelineKey {
    // ------------------------------------------------------------------------
    pub fn from_settings(settings: &TerrainRenderSettings) -> Self {
        let mut flags = TerrainMeshPipelineKey::NONE;

        // exclusive override
        if settings.show_fragment_normals {
            flags = TerrainMeshPipelineKey::SHOW_FRAGMENT_NORMAL;
        } else if settings.show_combined_normals {
            flags = TerrainMeshPipelineKey::SHOW_COMBINED_NORMAL;
        } else if settings.show_blend_threshold {
            flags = TerrainMeshPipelineKey::SHOW_BLEND_VALUE;
        } else if settings.show_bkgrnd_scaling {
            flags = TerrainMeshPipelineKey::SHOW_UV_SCALING;
        } else if settings.show_tint_map {
            flags = TerrainMeshPipelineKey::SHOW_TINT_MAP;
        } else {
            // combined
            if settings.use_flat_shading {
                flags |= TerrainMeshPipelineKey::FLAT_SHADING;
            }
            if settings.overlay_clipmap_level {
                flags |= TerrainMeshPipelineKey::SHOW_CLIPMAP_LEVEL;
            }
            if settings.ignore_overlay_texture {
                flags |= TerrainMeshPipelineKey::HIDE_OVERLAY_TEXTURE;
            }
            if settings.ignore_bkgrnd_texture {
                flags |= TerrainMeshPipelineKey::HIDE_BKGRND_TEXTURE;
            }
            if settings.ignore_tint_map {
                flags |= TerrainMeshPipelineKey::IGNORE_TINT_MAP;
            }
        }

        flags
    }
    // ------------------------------------------------------------------------
    fn shader_defs(&self) -> Vec<String> {
        let mut flags = Vec::default();

        if self.contains(Self::EXCLUSIVE_OVERRIDE) {
            // note: order is backwards!
            if self.contains(Self::SHOW_TINT_MAP) {
                return vec!["SHOW_TINT_MAP".to_string()];
            }
            if self.contains(Self::SHOW_BLEND_VALUE) {
                return vec!["SHOW_BLEND_VALUE".to_string()];
            }
            if self.contains(Self::SHOW_UV_SCALING) {
                return vec!["SHOW_UV_SCALING".to_string()];
            }
            if self.contains(Self::SHOW_COMBINED_NORMAL) {
                return vec!["SHOW_COMBINED_NORMAL".to_string()];
            }
            if self.contains(Self::SHOW_FRAGMENT_NORMAL) {
                return vec!["SHOW_FRAGMENT_NORMAL".to_string()];
            }
        } else {
            if self.contains(Self::FLAT_SHADING) {
                flags.push("FLAT_SHADING".to_string());
            }
            if self.contains(Self::SHOW_WIREFRAME) {
                flags.push("SHOW_WIREFRAME".to_string());
            }
            if self.contains(Self::SHOW_CLIPMAP_LEVEL) {
                flags.push("SHOW_CLIPMAP_LEVEL".to_string());
            }
            if self.contains(Self::HIDE_OVERLAY_TEXTURE) {
                flags.push("HIDE_OVERLAY_TEXTURE".to_string());
            }
            if self.contains(Self::HIDE_BKGRND_TEXTURE) {
                flags.push("HIDE_BKGRND_TEXTURE".to_string());
            }
            if self.contains(Self::IGNORE_TINT_MAP) {
                flags.push("IGNORE_TINT_MAP".to_string());
            }
        }

        flags
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
        let shader_defs = key.shader_defs();

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader_vert.clone(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![mesh_vertex_buffer_layout(key)],
            },
            fragment: Some(FragmentState {
                shader: self.shader_frag.clone(),
                entry_point: "fragment".into(),
                shader_defs,
                targets: vec![
                    // diffuse
                    ColorTargetState {
                        format: TextureFormat::bevy_default(),
                        blend,
                        write_mask: ColorWrites::ALL,
                    },
                    // world position
                    ColorTargetState {
                        // Note: 16Float results in visible brush blocking
                        format: TextureFormat::Rgba32Float,
                        blend,
                        write_mask: ColorWrites::ALL,
                    },
                ],
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
