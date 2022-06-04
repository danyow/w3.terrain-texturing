// ----------------------------------------------------------------------------
use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        render_phase::{AddRenderCommand, DrawFunctions, RenderPhase, SetItemPipeline},
        render_resource::{PipelineCache, SpecializedRenderPipelines},
        view::ExtractedView,
        RenderApp, RenderStage,
    },
};

use crate::mut_renderasset::{MutRenderAssetPlugin, MutRenderAssets};
use crate::resource::RenderResourcePlugin;

use crate::terrain_tiles::TerrainTileComponent;

use self::terrain_mesh::extract_meshes as extract_terrain_meshes;
use self::terrain_mesh::queue_mesh_bind_group as queue_terrain_mesh_bind_group;
use self::terrain_mesh::queue_mesh_view_bind_group as queue_terrain_mesh_view_bind_group;
use self::terrain_mesh::TerrainMeshUniform;

use self::pipeline::{TerrainMeshPipelineKey, TerrainMeshRenderPipeline};

use super::environment::EnvironmentData;
use super::rendergraph::Terrain3d;

use super::gpu::{GpuDirectionalLight, GpuTerrainMapInfoSettings};
use super::{
    ClipmapAssignment, ClipmapInfo, TerrainClipmap, TerrainMaterialParam, TerrainMaterialSet,
    TerrainRenderSettings,
};
// ----------------------------------------------------------------------------
mod pipeline;
mod terrain_clipmap;
mod terrain_material;
mod terrain_mesh;
// ----------------------------------------------------------------------------
pub use self::terrain_mesh::{TerrainMesh, TerrainMeshStats, TerrainMeshVertexData};
// ----------------------------------------------------------------------------
pub(super) mod gpu {
    pub use super::pipeline::{TerrainMeshPipelineKey, TerrainMeshRenderPipeline};
    pub use super::terrain_clipmap::{GpuClipmapInfo, GpuClipmapLayerInfo};
}
// ----------------------------------------------------------------------------
pub struct TerrainMeshRenderPlugin;
// ----------------------------------------------------------------------------
impl Plugin for TerrainMeshRenderPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainClipmap>()
            .add_asset::<TerrainMesh>()
            .add_plugin(UniformComponentPlugin::<TerrainMeshUniform>::default())
            .add_plugin(RenderResourcePlugin::<TerrainMaterialSet>::default())
            .add_plugin(RenderResourcePlugin::<TerrainClipmap>::default())
            .add_plugin(MutRenderAssetPlugin::<TerrainMesh>::default())
            .add_plugin(ExtractComponentPlugin::<ClipmapAssignment>::default());

        app.sub_app_mut(RenderApp)
            .add_render_command::<Terrain3d, DrawCmdTerrain>()
            .init_resource::<TerrainMeshRenderPipeline>()
            .init_resource::<SpecializedRenderPipelines<TerrainMeshRenderPipeline>>()
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
    draw_functions: Res<DrawFunctions<Terrain3d>>,
    terrain_pipeline: Res<TerrainMeshRenderPipeline>,
    settings: Res<TerrainRenderSettings>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TerrainMeshRenderPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    terrain_meshes: Res<MutRenderAssets<TerrainMesh>>,
    rendered_meshes: Query<
        (Entity, &TerrainMeshUniform, &Handle<TerrainMesh>),
        With<ClipmapAssignment>,
    >,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Terrain3d>)>,
) {
    let draw_terrain = draw_functions.read().get_id::<DrawCmdTerrain>().unwrap();

    // the specialization settings for terrain rendering are covered by ifdef
    // flags in the shader. but overlaying wireframes additionally requires a
    // different layout of the terrainmesh vertex buffer (and thus a switch
    // distorts the terrain because the updated vertexbuffer data is not
    // uploaded). to make the switch not break geometry meshes will be assigned
    // to different specialized pipelines based on availability of the data.
    let key = TerrainMeshPipelineKey::from_settings(&*settings);
    let wireframe_key = key | TerrainMeshPipelineKey::SHOW_WIREFRAME;
    let no_wireframe_key = key & !TerrainMeshPipelineKey::SHOW_WIREFRAME;

    let wireframe_pipeline =
        pipelines.specialize(&mut pipeline_cache, &terrain_pipeline, wireframe_key);
    let normal_pipeline =
        pipelines.specialize(&mut pipeline_cache, &terrain_pipeline, no_wireframe_key);

    for (view, mut terrainpass) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for (entity, mesh_uniform, mesh_handle) in rendered_meshes.iter() {
            let pipeline = if terrain_meshes
                .get(mesh_handle)
                .map(|m| m.has_barycentric_data)
                .unwrap_or_default()
            {
                wireframe_pipeline
            } else {
                normal_pipeline
            };

            terrainpass.add(Terrain3d {
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
    terrain_mesh::SetMeshViewBindGroup<0>,
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
