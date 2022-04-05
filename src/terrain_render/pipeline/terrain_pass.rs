// ----------------------------------------------------------------------------
use bevy::{
    core::FloatOrd,
    prelude::*,
    render::{
        camera::{ActiveCameras, CameraPlugin},
        render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
        render_phase::{
            CachedPipelinePhaseItem, DrawFunctionId, DrawFunctions, EntityPhaseItem, PhaseItem,
            RenderPhase, TrackedRenderPass,
        },
        render_resource::{
            CachedPipelineId, Extent3d, LoadOp, Operations, RenderPassColorAttachment,
            RenderPassDepthStencilAttachment, RenderPassDescriptor, TextureDescriptor,
            TextureDimension, TextureFormat, TextureUsages, TextureView,
        },
        renderer::{RenderContext, RenderDevice},
        texture::TextureCache,
        view::{ExtractedView, ViewDepthTexture, ViewTarget},
    },
};
// ----------------------------------------------------------------------------
#[derive(Component)]
struct TerrainPassRenderTargets {
    pub world_pos_view: TextureView,
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
pub(super) fn extract_camera_phases(mut commands: Commands, active_cameras: Res<ActiveCameras>) {
    if let Some(camera_3d) = active_cameras.get(CameraPlugin::CAMERA_3D) {
        if let Some(entity) = camera_3d.entity {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<Terrain3d>::default());
        }
    }
}
// ----------------------------------------------------------------------------
pub(super) fn prepare_rendertargets(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views_3d: Query<(Entity, &ExtractedView), With<RenderPhase<Terrain3d>>>,
) {
    for (entity, view) in views_3d.iter() {
        // Note: all rendertargets must have the same sample_count. since it
        // should be possible to load texel from the targets multisampling must
        // be disabled
        let sample_count = 1;
        let cached_world_pos = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("terrain_world_pos"),
                size: Extent3d {
                    depth_or_array_layers: 1,
                    width: view.width as u32,
                    height: view.height as u32,
                },
                mip_level_count: 1,
                sample_count,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba32Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            },
        );
        commands.entity(entity).insert(TerrainPassRenderTargets {
            world_pos_view: cached_world_pos.default_view,
        });
    }
}
// ----------------------------------------------------------------------------
// PhaseItem
// ----------------------------------------------------------------------------
pub struct Terrain3d {
    pub distance: f32,
    pub pipeline: CachedPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}
// ----------------------------------------------------------------------------
impl PhaseItem for Terrain3d {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}
// ----------------------------------------------------------------------------
impl EntityPhaseItem for Terrain3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }
}
// ----------------------------------------------------------------------------
impl CachedPipelinePhaseItem for Terrain3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedPipelineId {
        self.pipeline
    }
}
// ----------------------------------------------------------------------------
// RenderNode
// ----------------------------------------------------------------------------
pub struct TerrainPassNode {
    query: QueryState<
        (
            &'static RenderPhase<Terrain3d>,
            &'static ViewTarget,
            &'static ViewDepthTexture,
            &'static TerrainPassRenderTargets,
        ),
        With<ExtractedView>,
    >,
}
// ----------------------------------------------------------------------------
impl TerrainPassNode {
    // ------------------------------------------------------------------------
    pub const IN_VIEW: &'static str = "view";
    pub const OUT_WORLD_POS: &'static str = "out_world_pos";
    // ------------------------------------------------------------------------
    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Node for TerrainPassNode {
    // ------------------------------------------------------------------------
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::IN_VIEW, SlotType::Entity)]
    }
    // ------------------------------------------------------------------------
    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::OUT_WORLD_POS, SlotType::TextureView)]
    }
    // ------------------------------------------------------------------------
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }
    // ------------------------------------------------------------------------
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (terrain3d_phase, target, depth, render_targets) =
            match self.query.get_manual(world, view_entity) {
                Ok(query) => query,
                Err(_) => return Ok(()), // No window
            };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("terrain_pass_3d"),
            color_attachments: &[
                target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }),
                RenderPassColorAttachment {
                    view: &render_targets.world_pos_view,
                    resolve_target: None,
                    // terrain meshes are not rendered full screen so clear target
                    ops: Operations {
                        // alpha channel will be set to 1.0 if fragment is used
                        load: LoadOp::Clear(Color::rgba(0.0, 0.0, 0.0, 0.0).into()),
                        store: true,
                    },
                },
            ],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &depth.view,
                // NOTE: The opaque main pass loads the depth buffer and possibly overwrites it
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: true,
                }),
                stencil_ops: None,
            }),
        };

        let draw_functions = world.get_resource::<DrawFunctions<Terrain3d>>().unwrap();

        let render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);
        let mut draw_functions = draw_functions.write();
        let mut tracked_pass = TrackedRenderPass::new(render_pass);
        for item in &terrain3d_phase.items {
            let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
            draw_function.draw(world, &mut tracked_pass, view_entity, item);
        }

        // -- set output textures for subsequent nodes
        graph
            .set_output(Self::OUT_WORLD_POS, render_targets.world_pos_view.clone())
            .unwrap();

        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
