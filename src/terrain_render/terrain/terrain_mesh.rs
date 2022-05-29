// ----------------------------------------------------------------------------
// based on bevy pbr mesh pipeline and simplified to terrain mesh usecase.
// ----------------------------------------------------------------------------
use bevy::{
    core::cast_slice,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::{GpuBufferInfo, Indices},
        render_asset::PrepareAssetError,
        render_component::{ComponentUniforms, DynamicUniformIndex},
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            std140::AsStd140, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutEntry,
            BindingType, Buffer, BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages,
            ShaderStages, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
        },
        renderer::RenderDevice,
        view::{ViewUniform, ViewUniformOffset, ViewUniforms},
    },
};

use crate::mut_renderasset::{MutRenderAsset, MutRenderAssets};
use crate::resource::PreparedRenderResource;

use crate::terrain_render::TerrainMapInfo;
use crate::terrain_tiles::TerrainTileComponent;

use super::pipeline::{TerrainMeshPipelineKey, TerrainMeshRenderPipeline};
use super::{ClipmapAssignment, EnvironmentData, GpuDirectionalLight, GpuTerrainMapInfoSettings};
// ----------------------------------------------------------------------------
// mesh
// ----------------------------------------------------------------------------
#[derive(TypeUuid)]
#[uuid = "dd81109b-f363-4c59-be19-5038df017247"]
pub struct TerrainMesh {
    vertex_data: Option<TerrainMeshVertexData>,
    indices: Option<Indices>,
    stats: TerrainMeshStats,
}
// ----------------------------------------------------------------------------
pub enum TerrainMeshVertexData {
    PositionAndNormal(Vec<[f32; 4]>),
    WithBarycentricCoordinates(Vec<[f32; 5]>),
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub struct TerrainMeshStats {
    pub vertices: u32,
    pub triangles: u32,
    pub data_bytes: usize,
}
// ----------------------------------------------------------------------------
// render cmds
// ----------------------------------------------------------------------------
pub struct DrawMesh;
pub struct SetMeshBindGroup<const I: usize>;
pub struct SetMeshViewBindGroup<const I: usize>;
// ----------------------------------------------------------------------------
/// The GPU-representation of a [`TerrainMesh`].
/// Consists of a vertex data buffer and index data buffer.
// #[derive(Debug, Clone)]
pub struct GpuTerrainMesh {
    /// Contains all attribute data for each vertex.
    vertex_buffer: Buffer,
    buffer_info: GpuBufferInfo,
    pub has_barycentric_data: bool,
}
// ----------------------------------------------------------------------------
// mesh uniform
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
pub(super) fn mesh_vertex_buffer_layout(key: TerrainMeshPipelineKey) -> VertexBufferLayout {
    TerrainMeshVertexData::vertex_buffer_layout(key)
}
// ----------------------------------------------------------------------------
impl TerrainMeshVertexData {
    // ------------------------------------------------------------------------
    const fn size(&self) -> usize {
        use TerrainMeshVertexData::*;
        match self {
            PositionAndNormal(_) => 4 * 4,
            WithBarycentricCoordinates(_) => 5 * 4,
        }
    }
    // ------------------------------------------------------------------------
    fn vertex_buffer_layout(key: TerrainMeshPipelineKey) -> VertexBufferLayout {
        let (vertex_array_stride, vertex_attributes) =
            if key.contains(TerrainMeshPipelineKey::SHOW_WIREFRAME) {
                (
                    Self::WithBarycentricCoordinates(Vec::default()).size(),
                    vec![
                        // Position
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        // Normal (11:10:11 compressed into u32)
                        VertexAttribute {
                            format: VertexFormat::Uint32,
                            offset: 12,
                            shader_location: 1,
                        },
                        // Barycentric coords encoded as vertex no. in a u32
                        VertexAttribute {
                            format: VertexFormat::Uint32,
                            offset: 16,
                            shader_location: 2,
                        },
                    ],
                )
            } else {
                (
                    Self::PositionAndNormal(Vec::default()).size(),
                    vec![
                        // Position
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        // Normal (11:10:11 compressed into u32)
                        VertexAttribute {
                            format: VertexFormat::Uint32,
                            offset: 12,
                            shader_location: 1,
                        },
                    ],
                )
            };

        VertexBufferLayout {
            array_stride: vertex_array_stride as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: vertex_attributes,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// mesh view
// ----------------------------------------------------------------------------
pub struct TerrainMeshViewBindGroup {
    value: BindGroup,
}
// ----------------------------------------------------------------------------
pub(super) fn mesh_view_bind_group_layout() -> [BindGroupLayoutEntry; 3] {
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
        // Sun
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::FRAGMENT | ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(GpuDirectionalLight::std140_size_static() as u64),
            },
            count: None,
        },
        // Sun
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::FRAGMENT | ShaderStages::VERTEX,
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
pub(super) fn queue_mesh_view_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mesh_pipeline: Res<TerrainMeshRenderPipeline>,
    view_uniforms: Res<ViewUniforms>,
    environment: Res<PreparedRenderResource<EnvironmentData>>,
    map_info: Res<PreparedRenderResource<TerrainMapInfo>>,
) {
    if let (Some(view_binding), Some(env), Some(map_info)) = (
        view_uniforms.uniforms.binding(),
        environment.as_ref(),
        map_info.as_ref(),
    ) {
        let view_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_binding.clone(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: env.sun_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: map_info.as_entire_binding(),
                },
            ],
            label: Some("terrain_mesh_view_bind_group"),
            layout: &mesh_pipeline.view_layout,
        });

        commands.insert_resource(TerrainMeshViewBindGroup {
            value: view_bind_group,
        });
    }
}
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
// terrain mesh
// ----------------------------------------------------------------------------
impl TerrainMesh {
    // ------------------------------------------------------------------------
    pub fn new(vertex_data: TerrainMeshVertexData, indices: Indices) -> Self {
        Self {
            stats: TerrainMeshStats {
                vertices: vertex_data.len() as u32,
                triangles: indices.triangles(),
                data_bytes: vertex_data.buffer_size() + indices.buffer_size(),
            },
            vertex_data: Some(vertex_data),
            indices: Some(indices),
        }
    }
    // ------------------------------------------------------------------------
    pub fn stats(&self) -> &TerrainMeshStats {
        &self.stats
    }
    // ------------------------------------------------------------------------
    pub fn pending_upload(&self) -> bool {
        self.vertex_data.is_some()
    }
    // ------------------------------------------------------------------------
    fn get_vertex_buffer_bytes(&self) -> &[u8] {
        use TerrainMeshVertexData::*;

        match self
            .vertex_data
            .as_ref()
            .expect("missing terrain mesh vertex buffer")
        {
            PositionAndNormal(data) => cast_slice(data),
            WithBarycentricCoordinates(data) => cast_slice(data),
        }
    }
    // ------------------------------------------------------------------------
    /// Computes and returns the index data of the mesh as bytes.
    /// This is used to transform the index data into a GPU friendly format.
    fn get_index_buffer_bytes(&self) -> &[u8] {
        match self.indices() {
            Indices::U16(indices) => cast_slice(&indices[..]),
            Indices::U32(indices) => cast_slice(&indices[..]),
        }
    }
    // ------------------------------------------------------------------------
    /// Retrieves the vertex `indices` of the mesh.
    #[inline(always)]
    fn indices(&self) -> &Indices {
        self.indices.as_ref().expect("missing terrain mesh indices")
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn has_barycentric_data(&self) -> bool {
        matches!(
            self.vertex_data,
            Some(TerrainMeshVertexData::WithBarycentricCoordinates(_))
        )
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// terrainmesh -> renderasst processing
// ----------------------------------------------------------------------------
impl MutRenderAsset for TerrainMesh {
    type ExtractedAsset = TerrainMesh;

    type PreparedAsset = GpuTerrainMesh;

    type Param = SRes<RenderDevice>;
    // ------------------------------------------------------------------------
    fn pending_update(&self) -> bool {
        self.pending_upload()
    }
    // ------------------------------------------------------------------------
    fn extract_asset(&mut self) -> Self::ExtractedAsset {
        TerrainMesh {
            vertex_data: self.vertex_data.take(),
            indices: self.indices.take(),
            stats: TerrainMeshStats::default(),
        }
    }
    // ------------------------------------------------------------------------
    fn prepare_asset(
        mesh: Self::ExtractedAsset,
        render_device: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("Mesh Vertex Buffer"),
            contents: mesh.get_vertex_buffer_bytes(),
        });

        let buffer_info = GpuBufferInfo::Indexed {
            buffer: render_device.create_buffer_with_data(&BufferInitDescriptor {
                usage: BufferUsages::INDEX,
                contents: mesh.get_index_buffer_bytes(),
                label: Some("Mesh Index Buffer"),
            }),
            count: mesh.indices().len() as u32,
            index_format: mesh.indices().into(),
        };

        Ok(GpuTerrainMesh {
            vertex_buffer,
            buffer_info,
            has_barycentric_data: mesh.has_barycentric_data(),
        })
    }
    // ------------------------------------------------------------------------
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
        SRes<MutRenderAssets<TerrainMesh>>,
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
// stats helper
// ----------------------------------------------------------------------------
trait MeshIndicesStats {
    // ------------------------------------------------------------------------
    fn triangles(&self) -> u32;
    // ------------------------------------------------------------------------
    fn buffer_size(&self) -> usize;
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl MeshIndicesStats for Indices {
    // ------------------------------------------------------------------------
    fn triangles(&self) -> u32 {
        self.len() as u32 / 3
    }
    // ------------------------------------------------------------------------
    fn buffer_size(&self) -> usize {
        match self {
            Indices::U16(v) => v.len() * 2,
            Indices::U32(v) => v.len() * 4,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TerrainMeshVertexData {
    // ------------------------------------------------------------------------
    fn len(&self) -> usize {
        use TerrainMeshVertexData::*;
        match self {
            PositionAndNormal(d) => d.len(),
            WithBarycentricCoordinates(d) => d.len(),
        }
    }
    // ------------------------------------------------------------------------
    fn buffer_size(&self) -> usize {
        use TerrainMeshVertexData::*;
        match self {
            PositionAndNormal(d) => d.len() * self.size(),
            WithBarycentricCoordinates(d) => d.len() * self.size(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
use std::ops::Add;

impl<'a, 'b> Add<&'b TerrainMeshStats> for &'a TerrainMeshStats {
    type Output = TerrainMeshStats;

    fn add(self, other: &TerrainMeshStats) -> TerrainMeshStats {
        TerrainMeshStats {
            vertices: self.vertices + other.vertices,
            triangles: self.triangles + other.triangles,
            data_bytes: self.data_bytes + other.data_bytes,
        }
    }
}
// ----------------------------------------------------------------------------
