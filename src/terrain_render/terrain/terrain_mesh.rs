// ----------------------------------------------------------------------------
// based on bevy pbr mesh pipeline and simplified to terrain mesh usecase.
// ----------------------------------------------------------------------------
use bevy::{
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    prelude::*,
    render::{
        mesh::GpuBufferInfo,
        render_asset::RenderAssets,
        render_component::{ComponentUniforms, DynamicUniformIndex},
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            std140::AsStd140, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutEntry,
            BindingType, BufferBindingType, BufferSize, ShaderStages, VertexAttribute,
            VertexBufferLayout, VertexFormat, VertexStepMode,
        },
        renderer::RenderDevice,
    },
};

use crate::terrain_tiles::TerrainTileComponent;

use super::pipeline::{TerrainMeshPipelineKey, TerrainMeshRenderPipeline};
use super::{ClipmapAssignment, TerrainMesh};
// ----------------------------------------------------------------------------
// render cmds
// ----------------------------------------------------------------------------
pub struct DrawMesh;
pub struct SetMeshBindGroup<const I: usize>;
// ----------------------------------------------------------------------------
// mesh
// ----------------------------------------------------------------------------
#[derive(Component, AsStd140, Clone)]
pub struct TerrainMeshUniform {
    pub transform: Mat4,
    inverse_transpose_model: Mat4,
    clipmap_and_lod: u32,
}
// ----------------------------------------------------------------------------
pub struct TerrainMeshBindGroup {
    value: BindGroup,
}
// ----------------------------------------------------------------------------
pub(super) fn mesh_bind_group_layout() -> [BindGroupLayoutEntry; 1] {
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
pub(super) fn mesh_vertex_buffer_layout(_key: TerrainMeshPipelineKey) -> VertexBufferLayout {
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
        &ClipmapAssignment,
        &TerrainTileComponent,
    )>,
) {
    let mut tiles = Vec::with_capacity(*previous_tile_count);
    for (entity, computed_visibility, transform, mesh_handle, clipmap_assignment, tile) in
        terrainmesh_query.iter()
    {
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
                    clipmap_and_lod: clipmap_assignment.level as u32
                        | (tile.assigned_lod() as u32) << 16,
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
// render cmds
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
