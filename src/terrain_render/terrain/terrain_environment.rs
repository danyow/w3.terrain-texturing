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
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            std140::AsStd140, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutEntry,
            BindingType, BufferBindingType, BufferSize, ShaderStages,
        },
        renderer::RenderDevice,
        view::{ViewUniform, ViewUniformOffset, ViewUniforms},
    },
};

use super::pipeline::TerrainMeshRenderPipeline;
// ----------------------------------------------------------------------------
// render cmds
// ----------------------------------------------------------------------------
pub struct SetMeshViewBindGroup<const I: usize>;
// ----------------------------------------------------------------------------
// mesh view
// ----------------------------------------------------------------------------
pub struct TerrainMeshViewBindGroup {
    value: BindGroup,
}
// ----------------------------------------------------------------------------
pub(super) fn mesh_view_bind_group_layout() -> [BindGroupLayoutEntry; 1] {
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
// systems (queue)
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
