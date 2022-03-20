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
            std140::{AsStd140, Std140},
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutEntry, BindingType,
            Buffer, BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages,
            ShaderStages,
        },
        renderer::RenderDevice,
        view::{ViewUniform, ViewUniformOffset, ViewUniforms},
    },
};

use crate::resource::{PrepareResourceError, PreparedRenderResource, RenderResource};

use super::pipeline::TerrainMeshRenderPipeline;
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct DirectionalLight {
    pub color: Color,
    pub brightness: f32,
    pub direction: Vec3,
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub struct TerrainEnvironment {
    pub sun: DirectionalLight,
}
// ----------------------------------------------------------------------------
// render cmds
// ----------------------------------------------------------------------------
pub struct SetMeshViewBindGroup<const I: usize>;
// ----------------------------------------------------------------------------
// gpu representation of environment params
// ----------------------------------------------------------------------------
#[derive(AsStd140, Clone)]
pub struct GpuDirectionalLight {
    color: Vec3,
    brightness: f32,
    direction: Vec3,
}
// ----------------------------------------------------------------------------
pub struct GpuTerrainEnvironment {
    sun_buffer: Buffer,
}
// ----------------------------------------------------------------------------
// mesh view
// ----------------------------------------------------------------------------
pub struct TerrainMeshViewBindGroup {
    value: BindGroup,
}
// ----------------------------------------------------------------------------
pub(super) fn mesh_view_bind_group_layout() -> [BindGroupLayoutEntry; 2] {
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
    environment: Res<PreparedRenderResource<TerrainEnvironment>>,
) {
    if let (Some(view_binding), Some(env)) =
        (view_uniforms.uniforms.binding(), environment.as_ref())
    {
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
// terrain environment -> renderresource processing
// ----------------------------------------------------------------------------
impl RenderResource for TerrainEnvironment {
    // In RenderStage::Extract step the resource is extracted from "app world" to
    // "render world" into an "ExtractedResource".
    type ExtractedResource = TerrainEnvironment;
    // in RenderStage::Prepare step the extracted resource is transformed into its
    // GPU representation "PreparedResource"
    type PreparedResource = GpuTerrainEnvironment;
    // defines query for ecs data in the prepare resource step
    type Param = SRes<RenderDevice>;
    // ------------------------------------------------------------------------
    fn extract_resource(&self) -> Self::ExtractedResource {
        self.clone()
    }
    // ------------------------------------------------------------------------
    fn prepare_resource(
        environment: Self::ExtractedResource,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedResource, PrepareResourceError<Self::ExtractedResource>> {
        let sun = &environment.sun;
        let sun = GpuDirectionalLight {
            color: Vec3::from_slice(&sun.color.as_linear_rgba_f32()),
            brightness: sun.brightness,
            direction: sun.direction,
        };

        let sun_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("sunlight_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: sun.as_std140().as_bytes(),
        });

        Ok(GpuTerrainEnvironment { sun_buffer })
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
impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            color: Color::rgb(1.0, 1.0, 1.0),
            brightness: 0.5,
            direction: Vec3::new(0.0, 1.0, 0.0),
        }
    }
}
// ----------------------------------------------------------------------------
