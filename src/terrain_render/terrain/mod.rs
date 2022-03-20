// ----------------------------------------------------------------------------
use bevy::{
    core_pipeline::Opaque3d,
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        render_phase::{AddRenderCommand, DrawFunctions, RenderPhase, SetItemPipeline},
        render_resource::{RenderPipelineCache, SpecializedPipelines},
        view::ExtractedView,
        RenderApp, RenderStage,
    },
};

use crate::resource::RenderResourcePlugin;

use crate::terrain_tiles::TerrainTileComponent;

use self::terrain_environment::queue_mesh_view_bind_group as queue_terrain_mesh_view_bind_group;
use self::terrain_mesh::extract_meshes as extract_terrain_meshes;
use self::terrain_mesh::queue_mesh_bind_group as queue_terrain_mesh_bind_group;
use self::terrain_mesh::TerrainMeshUniform;

use self::pipeline::{TerrainMeshPipelineKey, TerrainMeshRenderPipeline};

use super::{
    ClipmapAssignment, TerrainClipmap, TerrainMaterialParam, TerrainMaterialSet, TerrainMesh,
};
// ----------------------------------------------------------------------------
mod pipeline;
mod terrain_clipmap;
mod terrain_environment;
mod terrain_material;
mod terrain_mesh;
// ----------------------------------------------------------------------------
pub use self::terrain_environment::TerrainEnvironment;
// ----------------------------------------------------------------------------
pub struct TerrainMeshRenderPlugin;
// ----------------------------------------------------------------------------
impl Plugin for TerrainMeshRenderPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainClipmap>()
            .add_plugin(UniformComponentPlugin::<TerrainMeshUniform>::default())
            .add_plugin(RenderResourcePlugin::<TerrainMaterialSet>::default())
            .add_plugin(RenderResourcePlugin::<TerrainClipmap>::default())
            .add_plugin(RenderResourcePlugin::<TerrainEnvironment>::default())
            .add_plugin(ExtractComponentPlugin::<ClipmapAssignment>::default())
            //TODO remove as soon as terrain mesh is dedicated type ?
            .add_plugin(ExtractComponentPlugin::<TerrainTileComponent>::default());

        app.sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawCmdTerrain>()
            .init_resource::<TerrainMeshRenderPipeline>()
            .init_resource::<SpecializedPipelines<TerrainMeshRenderPipeline>>()
            .add_system_to_stage(RenderStage::Extract, extract_terrain_meshes)
            .add_system_to_stage(RenderStage::Queue, queue_terrain_rendering)
            .add_system_to_stage(RenderStage::Queue, queue_terrain_mesh_bind_group)
            .add_system_to_stage(RenderStage::Queue, queue_terrain_mesh_view_bind_group);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn queue_terrain_rendering(
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    terrain_pipeline: Res<TerrainMeshRenderPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedPipelines<TerrainMeshRenderPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    terrain_meshes: Query<
        (Entity, &TerrainMeshUniform),
        (
            With<Handle<TerrainMesh>>,
            With<ClipmapAssignment>,
            With<TerrainTileComponent>,
        ),
    >,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Opaque3d>)>,
) {
    let draw_terrain = draw_functions.read().get_id::<DrawCmdTerrain>().unwrap();

    let key = TerrainMeshPipelineKey::from_msaa_samples(msaa.samples);
    let pipeline = pipelines.specialize(&mut pipeline_cache, &terrain_pipeline, key);

    for (view, mut opaque_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for (entity, mesh_uniform) in terrain_meshes.iter() {
            opaque_phase.add(Opaque3d {
                entity,
                pipeline,
                draw_function: draw_terrain,
                distance: view_row_2.dot(mesh_uniform.transform.col(3)),
            });
        }
    }
}
// ----------------------------------------------------------------------------
type DrawCmdTerrain = (
    SetItemPipeline,
    terrain_environment::SetMeshViewBindGroup<0>,
    terrain_mesh::SetMeshBindGroup<1>,
    terrain_material::SetTerrainMaterialSetBindGroup<2>,
    terrain_clipmap::SetTerrainClipmapBindGroup<3>,
    terrain_mesh::DrawMesh,
);
// ----------------------------------------------------------------------------
//TODO remove as soon as terrain mesh is dedicated type ?
// extract component into renderworld. entity has to be in renderworld to be
// considered for rendering.
impl bevy::render::render_component::ExtractComponent for TerrainTileComponent {
    type Query = &'static TerrainTileComponent;
    type Filter = With<TerrainTileComponent>;

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}
// ----------------------------------------------------------------------------
// must be in renderworld to extract level into TerrainMeshUniform
impl ExtractComponent for ClipmapAssignment {
    type Query = &'static ClipmapAssignment;

    type Filter = ();

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        *item
    }
}
// ----------------------------------------------------------------------------
