// ----------------------------------------------------------------------------
use bevy::{
    ecs::schedule::StateData,
    prelude::*,
    render::{render_resource::SpecializedComputePipelines, RenderApp, RenderStage},
};

use crate::config::{TerrainConfig, CLIPMAP_SIZE};
use crate::resource::RenderResourcePlugin;
use crate::texturearray::{TextureArray, TextureArrayBuilder};

use super::gpu;
use super::{
    ClipmapInfo, ClipmapLayerInfo, EnvironmentData, HeightmapClipmap, TerrainMapInfo,
    TerrainRenderSettings, TerrainRenderSystemLabel,
};

use self::pipeline::ComputeShadowsPipeline;
use self::systems::ComputeTerrainLightheightPipelineId;

use self::resource::{
    compute_input_bind_group_layout, lightheightmap_bind_group_layout,
    lightray_settings_bind_group_layout,
};
// ----------------------------------------------------------------------------
mod compute_node;
mod pipeline;
mod precomputed;

mod resource;
mod systems;
// ----------------------------------------------------------------------------
pub struct TerrainShadowsRenderSettings {
    pub intensity: f32,
    pub falloff_smoothness: f32,
    pub falloff_scale: f32,
    pub falloff_bias: f32,
    pub interpolation_range: f32,

    pub fast_shadows: bool,
    pub recompute_frequency: u8,
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub struct TerrainLightheightClipmap {
    lightheight: Handle<TextureArray>,
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub struct TerrainShadowsComputeInput {
    map_size: u32,
    heightmap: Handle<TextureArray>,
    compute_slices: Vec<ComputeSliceInfo>,
    thread_jobs: Vec<Vec<ComputeThreadJob>>,
    directional_clipmap_layer: Vec<DirectionalClipmapLayerInfo>,
    clipmap_info: ClipmapInfo,
}
// ----------------------------------------------------------------------------
pub use self::compute_node::ComputeTerrainShadowsNode;

pub(super) use self::resource::ExtractedTerrainShadowsRenderSettings;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct TerrainShadowsComputePlugin;
// ----------------------------------------------------------------------------
impl TerrainShadowsComputePlugin {
    // ------------------------------------------------------------------------
    pub fn init<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_enter(state).with_system(init)
    }
    // ------------------------------------------------------------------------
    pub fn reset_data<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_enter(state).with_system(reset)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Plugin for TerrainShadowsComputePlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainLightheightClipmap>()
            .init_resource::<TerrainShadowsComputeInput>()
            .init_resource::<TerrainShadowsLightrayInfo>()
            .init_resource::<TerrainShadowsUpdateTracker>()
            .init_resource::<TerrainShadowsRenderSettings>()
            .add_plugin(RenderResourcePlugin::<TerrainShadowsRenderSettings>::default())
            .add_plugin(
                RenderResourcePlugin::<TerrainLightheightClipmap>::default()
                    .prepare_after(TerrainRenderSystemLabel::PrepareMapInfo),
            )
            .add_plugin(RenderResourcePlugin::<TerrainShadowsComputeInput>::default())
            .add_plugin(RenderResourcePlugin::<TerrainShadowsLightrayInfo>::default())
            .add_system(
                systems::update_compute_terrain_shadows
                    // *should* run after a potential sun ray update
                    .after("sun_position_update")
                    // important to run after a potential heightmap clipmap update
                    .after("update_clipmaps"),
            );

        app.sub_app_mut(RenderApp)
            .insert_resource(TerrainShadowsComputeTrigger::inactive())
            .init_resource::<ComputeTerrainLightheightPipelineId>()
            .init_resource::<ComputeShadowsPipeline>()
            .init_resource::<SpecializedComputePipelines<ComputeShadowsPipeline>>()
            .add_system_to_stage(
                RenderStage::Extract,
                systems::extract_compute_shadows_trigger,
            )
            .add_system_to_stage(RenderStage::Queue, systems::queue_terrain_shadows_info);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
fn init(
    mut commands: Commands,
    mut texture_arrays: ResMut<Assets<TextureArray>>,
    terrain_config: Res<TerrainConfig>,
) {
    commands.insert_resource(TerrainLightheightClipmap::new(
        CLIPMAP_SIZE,
        terrain_config.clipmap_levels(),
        texture_arrays.as_mut(),
    ));
}
// ----------------------------------------------------------------------------
fn reset(mut commands: Commands) {
    commands.insert_resource(TerrainLightheightClipmap::default());
    commands.insert_resource(TerrainShadowsComputeInput::default());
}
// ----------------------------------------------------------------------------
impl TerrainLightheightClipmap {
    // ------------------------------------------------------------------------
    fn new(clipmap_size: u32, level_count: u8, texture_arrays: &mut Assets<TextureArray>) -> Self {
        use bevy::render::render_resource::{TextureFormat, TextureUsages};
        use image::{DynamicImage::ImageLuma16, ImageBuffer};

        // texture arrays require at least 2 level to be recognized as arrays
        let level_count = level_count.max(2) as u32;

        // no mipmaps for clipmap since only the highest res will be used
        let mut builder =
            TextureArrayBuilder::new(clipmap_size, level_count, TextureFormat::R16Uint, None);

        for _ in 0..level_count {
            let data = vec![u16::MIN; (clipmap_size * clipmap_size) as usize];
            let layer_image =
                ImageLuma16(ImageBuffer::from_raw(clipmap_size, clipmap_size, data).unwrap());

            builder.add_texture(layer_image);
        }

        Self {
            lightheight: texture_arrays
                .add(builder.add_usage(TextureUsages::STORAGE_BINDING).build()),
        }
    }
    // ------------------------------------------------------------------------
    pub fn array(&self) -> &Handle<TextureArray> {
        &self.lightheight
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TerrainShadowsComputeInput {
    // ------------------------------------------------------------------------
    pub fn set_heightmap_clipmap(&mut self, heightmap: &HeightmapClipmap) {
        self.map_size = heightmap.data_size();
        self.heightmap = heightmap.array().clone();
    }
    // ------------------------------------------------------------------------
    pub fn update_clipmapinfo(&mut self, new_info: ClipmapInfo) {
        self.clipmap_info = new_info;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// plugin internal
// ----------------------------------------------------------------------------
// #[derive(Clone)]
struct TerrainShadowsUpdateTracker {
    tick: u32,
    recompute_frequency: u32,
    recompute: bool,
}
// ----------------------------------------------------------------------------
struct TerrainShadowsComputeTrigger {
    recompute: bool,
    trace_direction: LightrayDirection,
}
// ----------------------------------------------------------------------------
#[derive(Clone, Debug)]
struct TerrainShadowsLightrayInfo {
    lightpos_offset: u32,
    interpolation_weight: f32,
    ray_height_delta: f32,
    direction: LightrayDirection,
}
// ----------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LightrayDirection {
    LeftRight,
    RightLeft,
    TopBottom,
    BottomTop,
}
// ----------------------------------------------------------------------------
#[derive(Clone, Debug)]
struct ComputeThreadJob {
    rays: u32,
    start_ray: u32,
    clipmap_level: u32,
}
// ----------------------------------------------------------------------------
#[derive(Clone, Debug)]
struct ComputeSliceInfo {
    highest_res_clipmap_level: u32,
    step_after: u32,
    schedule_id: u32,
}

// ----------------------------------------------------------------------------
// Contains transformed ClipmapLayerInfo offsets independent of ray direction:
// step_1 and step_after contain map offsets [0..map_size[ in the direction of
// the ray.
#[derive(Clone)]
struct DirectionalClipmapLayerInfo {
    step_1: u16,
    step_after: u16,
    ray_1: u16,
    ray_after: u16,
}
// ----------------------------------------------------------------------------
// defaults
// ----------------------------------------------------------------------------
impl Default for TerrainShadowsUpdateTracker {
    fn default() -> Self {
        Self {
            tick: 0,
            recompute_frequency: 3,
            recompute: false,
        }
    }
}
// ----------------------------------------------------------------------------
impl Default for TerrainShadowsRenderSettings {
    fn default() -> Self {
        Self {
            intensity: 0.8,
            falloff_smoothness: 50.0,
            falloff_scale: 2000.0,
            falloff_bias: 50.0,
            interpolation_range: 250.0,
            fast_shadows: false,
            recompute_frequency: 3,
        }
    }
}
// ----------------------------------------------------------------------------
