// ----------------------------------------------------------------------------
mod terrain_pass;
mod terrain_pass_driver;
// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_graph::{EmptyNode, RenderGraph, SlotInfo, SlotType},
        render_phase::{sort_phase_system, DrawFunctions},
        RenderApp, RenderStage,
    },
};

use self::terrain_pass_driver::TerrainPassDriverNode;
// ----------------------------------------------------------------------------
pub use self::terrain_pass::{Terrain3d, TerrainPassNode, TerrainPassRenderTargets};
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct TerrainRenderGraphPlugin;
// ----------------------------------------------------------------------------
pub mod node {
    pub const TERRAIN_PASS_DEPENDENCIES: &str = "terrain_pass_dependencies";
    pub const TERRAIN_PASS_DRIVER: &str = "terrain_pass_driver";
}
pub mod terrain_3d_graph {
    pub const NAME: &str = "terrain_3d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }
    pub mod node {
        pub const MAIN_PASS: &str = "terrain_pass";
        pub const ENV_FOG_PASS: &str = "env_fog_pass";
        pub const TONEMAPPING_PASS: &str = "tonemapping_pass";
        pub const BRUSH_POINTER_PASS: &str = "brush_pointer_pass";
    }
}
// ----------------------------------------------------------------------------
impl Plugin for TerrainRenderGraphPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<DrawFunctions<Terrain3d>>()
            .add_system_to_stage(RenderStage::Extract, terrain_pass::extract_camera_phases)
            .add_system_to_stage(RenderStage::Prepare, terrain_pass::prepare_rendertargets)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Terrain3d>);

        let terrain_pass_node_3d = TerrainPassNode::new(&mut render_app.world);

        let mut render_graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        let mut terrain_3d_graph = RenderGraph::default();

        terrain_3d_graph.add_node(terrain_3d_graph::node::MAIN_PASS, terrain_pass_node_3d);
        let input_node_id = terrain_3d_graph.set_input(vec![SlotInfo::new(
            terrain_3d_graph::input::VIEW_ENTITY,
            SlotType::Entity,
        )]);

        terrain_3d_graph
            .add_slot_edge(
                input_node_id,
                terrain_3d_graph::input::VIEW_ENTITY,
                terrain_3d_graph::node::MAIN_PASS,
                TerrainPassNode::IN_VIEW,
            )
            .unwrap();

        render_graph.add_sub_graph(terrain_3d_graph::NAME, terrain_3d_graph);

        render_graph.add_node(node::TERRAIN_PASS_DEPENDENCIES, EmptyNode);
        render_graph.add_node(node::TERRAIN_PASS_DRIVER, TerrainPassDriverNode);
        render_graph
            .add_node_edge(node::TERRAIN_PASS_DEPENDENCIES, node::TERRAIN_PASS_DRIVER)
            .unwrap();

        render_graph
            .add_node_edge(
                bevy::core_pipeline::node::CLEAR_PASS_DRIVER,
                node::TERRAIN_PASS_DRIVER,
            )
            .unwrap();
        render_graph
            .add_node_edge(
                node::TERRAIN_PASS_DRIVER,
                bevy::core_pipeline::node::MAIN_PASS_DRIVER,
            )
            .unwrap();

        add_environment_fog_node(render_app);

        add_tonemapping_node(render_app);

        add_brushpointer_overlay_node(render_app);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
fn add_environment_fog_node(render_app: &mut App) {
    use super::environment::FogNode;

    use self::terrain_3d_graph::input;
    use self::terrain_3d_graph::node;

    let fog_node = FogNode::new(&mut render_app.world);

    let mut render_graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
    let terrain_3d_graph = render_graph
        .get_sub_graph_mut(terrain_3d_graph::NAME)
        .unwrap();

    let input_node_id = terrain_3d_graph.input_node().unwrap().id;

    terrain_3d_graph.add_node(node::ENV_FOG_PASS, fog_node);
    terrain_3d_graph
        .add_node_edge(terrain_3d_graph::node::MAIN_PASS, node::ENV_FOG_PASS)
        .unwrap();

    terrain_3d_graph
        .add_slot_edge(
            input_node_id,
            input::VIEW_ENTITY,
            node::ENV_FOG_PASS,
            FogNode::IN_VIEW,
        )
        .unwrap();

    terrain_3d_graph
        .add_slot_edge(
            node::MAIN_PASS,
            TerrainPassNode::OUT_HDR_VIEW,
            node::ENV_FOG_PASS,
            FogNode::IN_HDR_VIEW,
        )
        .unwrap();

    terrain_3d_graph
        .add_slot_edge(
            node::MAIN_PASS,
            TerrainPassNode::OUT_WORLD_POS,
            node::ENV_FOG_PASS,
            FogNode::IN_WORLD_POS,
        )
        .unwrap();
}
// ----------------------------------------------------------------------------
fn add_tonemapping_node(render_app: &mut App) {
    use super::environment::FogNode;
    use super::tonemapping::TonemappingNode;

    use self::terrain_3d_graph::input;
    use self::terrain_3d_graph::node;

    let tonemapping_node = TonemappingNode::new(&mut render_app.world);

    let mut render_graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
    let terrain_3d_graph = render_graph
        .get_sub_graph_mut(terrain_3d_graph::NAME)
        .unwrap();

    let input_node_id = terrain_3d_graph.input_node().unwrap().id;

    terrain_3d_graph.add_node(node::TONEMAPPING_PASS, tonemapping_node);
    terrain_3d_graph
        .add_node_edge(terrain_3d_graph::node::ENV_FOG_PASS, node::TONEMAPPING_PASS)
        .unwrap();

    terrain_3d_graph
        .add_slot_edge(
            input_node_id,
            input::VIEW_ENTITY,
            node::TONEMAPPING_PASS,
            TonemappingNode::IN_VIEW,
        )
        .unwrap();

    terrain_3d_graph
        .add_slot_edge(
            node::ENV_FOG_PASS,
            FogNode::OUT_HDR_VIEW,
            node::TONEMAPPING_PASS,
            TonemappingNode::IN_HDR_VIEW,
        )
        .unwrap();
}
// ----------------------------------------------------------------------------
fn add_brushpointer_overlay_node(render_app: &mut App) {
    use super::brush::BrushPointerNode;

    use self::terrain_3d_graph::input;
    use self::terrain_3d_graph::node;

    let pointer_node = BrushPointerNode::new(&mut render_app.world);

    let mut render_graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
    let terrain_3d_graph = render_graph
        .get_sub_graph_mut(terrain_3d_graph::NAME)
        .unwrap();

    terrain_3d_graph.add_node(node::BRUSH_POINTER_PASS, pointer_node);

    // dependency
    let input_node_id = terrain_3d_graph.input_node().unwrap().id;
    terrain_3d_graph
        .add_node_edge(node::TONEMAPPING_PASS, node::BRUSH_POINTER_PASS)
        .unwrap();

    terrain_3d_graph
        .add_slot_edge(
            input_node_id,
            input::VIEW_ENTITY,
            node::BRUSH_POINTER_PASS,
            BrushPointerNode::IN_VIEW,
        )
        .unwrap();
    terrain_3d_graph
        .add_slot_edge(
            node::MAIN_PASS,
            TerrainPassNode::OUT_WORLD_POS,
            node::BRUSH_POINTER_PASS,
            BrushPointerNode::IN_WORLD_POS,
        )
        .unwrap();
}
// ----------------------------------------------------------------------------
