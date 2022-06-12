// ----------------------------------------------------------------------------
use bevy::{
    core_pipeline::Opaque3d,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    pbr::{MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        render_asset::RenderAssets,
        render_component::{ExtractComponent, ExtractComponentPlugin},
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::*,
        renderer::RenderDevice,
        view::{ExtractedView, NoFrustumCulling},
        RenderApp, RenderStage,
    },
};
use bytemuck::{Pod, Zeroable};

use super::shapes;
use super::PathInterpolation;
// ----------------------------------------------------------------------------
pub struct CameraPathVisualizationPlugin;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(super) struct VisualizedPathInterpolation(Option<PathInterpolation>);
// ----------------------------------------------------------------------------
impl Plugin for CameraPathVisualizationPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<PathVisualizationArrowData>::default())
            .add_system(respawn_path)
            .init_resource::<VisualizedPathInterpolation>()
            .init_resource::<PathVisualization>();

        app.sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawCustom>()
            .init_resource::<CustomPipeline>()
            .init_resource::<SpecializedMeshPipelines<CustomPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_path_visualization)
            .add_system_to_stage(RenderStage::Prepare, prepare_path_visualization_buffers);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl VisualizedPathInterpolation {
    // ------------------------------------------------------------------------
    pub fn is_active(&self) -> bool {
        self.0.is_some()
    }
    // ------------------------------------------------------------------------
    pub fn set(&mut self, interpolation: PathInterpolation) {
        self.0 = Some(interpolation)
    }
    // ------------------------------------------------------------------------
    pub fn remove(&mut self) {
        self.0 = None;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(Component, Deref)]
struct PathVisualizationArrowData(Vec<InstanceData>);
// ----------------------------------------------------------------------------
impl ExtractComponent for PathVisualizationArrowData {
    type Query = &'static PathVisualizationArrowData;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        PathVisualizationArrowData(item.0.clone())
    }
}
// ----------------------------------------------------------------------------
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    c1: Vec4,
    c2: Vec4,
    c3: Vec4,
    c4: Vec4,
    color: [f32; 4],
}
// ----------------------------------------------------------------------------
#[derive(Default)]
struct PathVisualization {
    path: Option<Entity>,
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
fn respawn_path(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    path_interpolation: Res<VisualizedPathInterpolation>,
    mut visualization: ResMut<PathVisualization>,
) {
    if path_interpolation.is_changed() {
        if let Some(visu_entity) = visualization.path {
            commands.entity(visu_entity).despawn();
            visualization.path = None;
        }

        if let Some(path_interpolation) = path_interpolation.0.as_ref() {
            let arrows = 20 * path_interpolation.keypoint_count();

            let instance_data = (0..arrows)
                .map(|i| {
                    let t = i as f32 / arrows as f32;
                    let (pos, rot) = path_interpolation.sample(t);
                    InstanceData::from((pos, rot, Color::hsla(t * 360., 0.5, 0.5, 1.0)))
                })
                .collect();

            let id = commands
                .spawn()
                .insert_bundle((
                    meshes.add(Mesh::from(shapes::CameraVisualization::new(10.0))),
                    Transform::from_xyz(0.0, 0.0, 0.0),
                    GlobalTransform::default(),
                    PathVisualizationArrowData(instance_data),
                    Visibility::default(),
                    ComputedVisibility::default(),
                    // NOTE: Frustum culling is done based on the Aabb of the Mesh and the GlobalTransform.
                    // As the cube is at the origin, if its Aabb moves outside the view frustum, all the
                    // instanced cubes will be culled.
                    // The InstanceMaterialData contains the 'GlobalTransform' information for this custom
                    // instancing, and that is not taken into account with the built-in frustum culling.
                    // We must disable the built-in frustum culling by adding the `NoFrustumCulling` marker
                    // component to avoid incorrect culling.
                    NoFrustumCulling,
                ))
                .id();

            visualization.path = Some(id);
        }
    }
}
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn queue_path_visualization(
    transparent_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    custom_pipeline: Res<CustomPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    meshes: Res<RenderAssets<Mesh>>,
    material_meshes: Query<
        (Entity, &MeshUniform, &Handle<Mesh>),
        (With<Handle<Mesh>>, With<PathVisualizationArrowData>),
    >,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Opaque3d>)>,
) {
    let draw_custom = transparent_3d_draw_functions
        .read()
        .get_id::<DrawCustom>()
        .unwrap();

    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

    for (view, mut transparent_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for (entity, mesh_uniform, mesh_handle) in material_meshes.iter() {
            if let Some(mesh) = meshes.get(mesh_handle) {
                let key =
                    msaa_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
                let pipeline = pipelines
                    .specialize(&mut pipeline_cache, &custom_pipeline, key, &mesh.layout)
                    .unwrap();
                transparent_phase.add(Opaque3d {
                    entity,
                    pipeline,
                    draw_function: draw_custom,
                    distance: view_row_2.dot(mesh_uniform.transform.col(3)),
                });
            }
        }
    }
}
// ----------------------------------------------------------------------------
#[derive(Component)]
pub struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}
// ----------------------------------------------------------------------------
fn prepare_path_visualization_buffers(
    mut commands: Commands,
    query: Query<(Entity, &PathVisualizationArrowData)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, instance_data) in query.iter() {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(instance_data.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            length: instance_data.len(),
        });
    }
}
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
pub struct CustomPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
}

impl FromWorld for CustomPipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        asset_server.watch_for_changes().unwrap();
        let shader = asset_server.load("shaders/path_visualization.wgsl");

        let mesh_pipeline = world.get_resource::<MeshPipeline>().unwrap();

        CustomPipeline {
            shader,
            mesh_pipeline: mesh_pipeline.clone(),
        }
    }
}

impl SpecializedMeshPipeline for CustomPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                // shader locations 0-2 are taken up by Position, Normal and UV attributes
                VertexAttribute {
                    // col1
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 3,
                },
                VertexAttribute {
                    // col2
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size(),
                    shader_location: 4,
                },
                VertexAttribute {
                    // col3
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 2,
                    shader_location: 5,
                },
                VertexAttribute {
                    // col4
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 3,
                    shader_location: 6,
                },
                VertexAttribute {
                    // color
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 4,
                    shader_location: 7,
                },
            ],
        });
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
        ]);

        Ok(descriptor)
    }
}

type DrawCustom = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawMeshInstanced,
);

pub struct DrawMeshInstanced;

impl EntityRenderCommand for DrawMeshInstanced {
    type Param = (
        SRes<RenderAssets<Mesh>>,
        SQuery<Read<Handle<Mesh>>>,
        SQuery<Read<InstanceBuffer>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (meshes, mesh_query, instance_buffer_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_handle = mesh_query.get(item).unwrap();
        let instance_buffer = instance_buffer_query.get_inner(item).unwrap();

        let gpu_mesh = match meshes.into_inner().get(mesh_handle) {
            Some(gpu_mesh) => gpu_mesh,
            None => return RenderCommandResult::Failure,
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, 0..instance_buffer.length as u32);
            }
            GpuBufferInfo::NonIndexed { vertex_count } => {
                pass.draw(0..*vertex_count, 0..instance_buffer.length as u32);
            }
        }
        RenderCommandResult::Success
    }
}
// ----------------------------------------------------------------------------
//
// ----------------------------------------------------------------------------
impl From<(Vec3, Quat, Color)> for InstanceData {
    // ------------------------------------------------------------------------
    fn from((pos, rot, color): (Vec3, Quat, Color)) -> Self {
        let mat = Mat4::from_rotation_translation(rot, pos);
        InstanceData {
            c1: mat.x_axis,
            c2: mat.y_axis,
            c3: mat.z_axis,
            c4: mat.w_axis,
            color: color.as_rgba_f32(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
