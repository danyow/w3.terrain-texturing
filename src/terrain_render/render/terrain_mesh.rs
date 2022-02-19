// ----------------------------------------------------------------------------
// based on bevy pbr mesh pipeline and simplified to terrain mesh usecase.
// ----------------------------------------------------------------------------
use bevy::{
    ecs::{
        system::{
            lifetimeless::{Read, SQuery, SRes},
            SystemParamItem,
        },
    },
    prelude::*,
    render::{
        mesh::GpuBufferInfo,
        render_asset::RenderAssets,
        render_component::{ComponentUniforms, DynamicUniformIndex},
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            std140::AsStd140, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState,
            BufferBindingType, BufferSize, ColorTargetState, ColorWrites, CompareFunction,
            DepthBiasState, DepthStencilState, Face, FragmentState, FrontFace, MultisampleState,
            PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipelineDescriptor, ShaderStages,
            SpecializedPipeline, StencilFaceState, StencilState, TextureFormat, VertexAttribute,
            VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
        view::{ViewUniform, ViewUniformOffset, ViewUniforms},
    },
};

use crate::terrain_tiles::TerrainTileComponent;

use super::TerrainMesh;
// ----------------------------------------------------------------------------
// render cmds
// ----------------------------------------------------------------------------
pub struct DrawMesh;
pub struct SetMeshViewBindGroup<const I: usize>;
pub struct SetMeshBindGroup<const I: usize>;
// ----------------------------------------------------------------------------
// pipeline
// ----------------------------------------------------------------------------
pub struct TerrainMeshRenderPipeline {
    shader_vert: Handle<Shader>,
    shader_frag: Handle<Shader>,

    view_layout: BindGroupLayout,
    mesh_layout: BindGroupLayout,
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

        TerrainMeshRenderPipeline {
            shader_vert,
            shader_frag,

            view_layout,
            mesh_layout,
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
            layout: Some(vec![self.view_layout.clone(), self.mesh_layout.clone()]),
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
// mesh
// ----------------------------------------------------------------------------
#[derive(Component, AsStd140, Clone)]
pub struct TerrainMeshUniform {
    pub transform: Mat4,
    inverse_transpose_model: Mat4,
    lod: u32,
}
// ----------------------------------------------------------------------------
pub struct TerrainMeshBindGroup {
    value: BindGroup,
}
// ----------------------------------------------------------------------------
fn mesh_bind_group_layout() -> [BindGroupLayoutEntry; 1] {
    [BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: true,
            min_binding_size: BufferSize::new(TerrainMeshUniform::std140_size_static() as u64),
        },
        count: None,
    }]
}
// ----------------------------------------------------------------------------
fn mesh_vertex_buffer_layout(_key: TerrainMeshPipelineKey) -> VertexBufferLayout {
    // TODO: simplify. for now this is copied from simple mesh definition
    // so includes normals and UV
    let (vertex_array_stride, vertex_attributes) = (
        32,
        vec![
            // Position (GOTCHA! Vertex_Position isn't first in the buffer due to how Mesh sorts attributes (alphabetically))
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 12,
                shader_location: 0,
            },
            // Normal
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 1,
            },
            // Uv
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 24,
                shader_location: 2,
            },
        ],
    );
    VertexBufferLayout {
        array_stride: vertex_array_stride,
        step_mode: VertexStepMode::Vertex,
        attributes: vertex_attributes,
    }
}
// ----------------------------------------------------------------------------
// mesh view
// ----------------------------------------------------------------------------
pub struct TerrainMeshViewBindGroup {
    value: BindGroup,
}
// ----------------------------------------------------------------------------
fn mesh_view_bind_group_layout() -> [BindGroupLayoutEntry; 1] {
    [
        // View
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: BufferSize::new(ViewUniform::std140_size_static() as u64),
            },
            count: None,
        },
        // TODO Lights: Sunlight maybe Moonlight
    ]
}
// ----------------------------------------------------------------------------
// systems (extract)
// ----------------------------------------------------------------------------
#[allow(clippy::type_complexity)]
pub(super) fn extract_meshes(
    mut commands: Commands,
    mut previous_tile_count: Local<usize>,
    terrainmesh_query: Query<(
        Entity,
        &ComputedVisibility,
        &GlobalTransform,
        &Handle<TerrainMesh>,
        &TerrainTileComponent,
    )>,
) {
    let mut tiles = Vec::with_capacity(*previous_tile_count);
    for (entity, computed_visibility, transform, mesh_handle, tile) in terrainmesh_query.iter() {
        if !computed_visibility.is_visible {
            continue;
        }
        let transform = transform.compute_matrix();
        tiles.push((
            entity,
            (
                TerrainMeshUniform {
                    transform,
                    inverse_transpose_model: transform.inverse().transpose(),
                    lod: tile.assigned_lod() as u32,
                },
                mesh_handle.clone_weak(),
            ),
        ));
    }
    *previous_tile_count = tiles.len();
    commands.insert_or_spawn_batch(tiles);
}
// ----------------------------------------------------------------------------
// systems (queue)
// ----------------------------------------------------------------------------
pub(super) fn queue_mesh_bind_group(
    mut commands: Commands,
    mesh_pipeline: Res<TerrainMeshRenderPipeline>,
    render_device: Res<RenderDevice>,
    mesh_uniforms: Res<ComponentUniforms<TerrainMeshUniform>>,
) {
    if let Some(binding) = mesh_uniforms.uniforms().binding() {
        commands.insert_resource(TerrainMeshBindGroup {
            value: render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: binding,
                }],
                label: Some("terrain_mesh_bind_group"),
                layout: &mesh_pipeline.mesh_layout,
            }),
        });
    }
}
// ----------------------------------------------------------------------------
pub(super) fn queue_mesh_view_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mesh_pipeline: Res<TerrainMeshRenderPipeline>,
    view_uniforms: Res<ViewUniforms>,
) {
    if let (Some(view_binding),) = (view_uniforms.uniforms.binding(),) {
        let view_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_binding.clone(),
            }],
            label: Some("terrain_mesh_view_bind_group"),
            layout: &mesh_pipeline.view_layout,
        });

        commands.insert_resource(TerrainMeshViewBindGroup {
            value: view_bind_group,
        });
    }
}
// ----------------------------------------------------------------------------
// render cmds
// ----------------------------------------------------------------------------
impl<const I: usize> EntityRenderCommand for SetMeshViewBindGroup<I> {
    type Param = (
        SRes<TerrainMeshViewBindGroup>,
        SQuery<Read<ViewUniformOffset>>,
    );
    #[inline]
    fn render<'w>(
        view: Entity,
        _item: Entity,
        (mesh_view_bind_group, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let view_uniform = view_query
            .get(view)
            .map_err(|e| error!("query error {} {:?}", e, _item))
            .unwrap();
        pass.set_bind_group(
            I,
            &mesh_view_bind_group.into_inner().value,
            &[view_uniform.offset],
        );

        RenderCommandResult::Success
    }
}
// ----------------------------------------------------------------------------
impl<const I: usize> EntityRenderCommand for SetMeshBindGroup<I> {
    type Param = (
        SRes<TerrainMeshBindGroup>,
        SQuery<Read<DynamicUniformIndex<TerrainMeshUniform>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (mesh_bind_group, mesh_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_index = mesh_query.get(item).unwrap();
        pass.set_bind_group(
            I,
            &mesh_bind_group.into_inner().value,
            &[mesh_index.index()],
        );
        RenderCommandResult::Success
    }
}
// ----------------------------------------------------------------------------
impl EntityRenderCommand for DrawMesh {
    type Param = (
        SRes<RenderAssets<TerrainMesh>>,
        SQuery<Read<Handle<TerrainMesh>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (meshes, mesh_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_handle = mesh_query.get(item).unwrap();
        if let Some(gpu_mesh) = meshes.into_inner().get(mesh_handle) {
            pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
            match &gpu_mesh.buffer_info {
                GpuBufferInfo::Indexed {
                    buffer,
                    index_format,
                    count,
                } => {
                    pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                    pass.draw_indexed(0..*count, 0, 0..1);
                }
                GpuBufferInfo::NonIndexed { vertex_count } => {
                    pass.draw(0..*vertex_count, 0..1);
                }
            }
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}
// ----------------------------------------------------------------------------
