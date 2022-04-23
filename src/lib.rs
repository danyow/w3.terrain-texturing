// ----------------------------------------------------------------------------
use bevy::{app::AppExit, prelude::*, render::render_resource::TextureFormat, tasks::Task};
use bevy_egui::EguiContext;
// ----------------------------------------------------------------------------
pub struct EditorPlugin;
// ----------------------------------------------------------------------------
use camera::CameraPlugin;

use cmds::AsyncTaskFinishedEvent;
use gui::UiImages;

use crate::environment::EnvironmentPlugin;
use crate::heightmap::HeightmapPlugin;
use crate::terrain_clipmap::TerrainClipmapPlugin;
use crate::terrain_material::MaterialSetPlugin;
use crate::terrain_painting::TerrainPaintingPlugin;
use crate::terrain_tiles::TerrainTilesGeneratorPlugin;
// ----------------------------------------------------------------------------
mod atmosphere;
mod config;
mod loader;

mod heightmap;
mod terrain_clipmap;
mod terrain_material;
mod terrain_tiles;
mod texturecontrol;
mod tintmap;

mod environment;
mod terrain_painting;
mod terrain_render;

mod camera;
mod clipmap;
mod compute;
mod mut_renderasset;
mod resource;
mod shapes;
mod texturearray;

mod cmds;
mod gui;
// ----------------------------------------------------------------------------
#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
enum EditorState {
    Initialization,
    NoTerrainData,
    TerrainLoading,
    Editing,
    FreeCam,
}
// ----------------------------------------------------------------------------
/// events triggered by editor and not user (e.g. to update something in GUI)
enum EditorEvent {
    TerrainTextureUpdated(terrain_material::TextureUpdatedEvent),
    ProgressTrackingStart(cmds::TrackedTaskname, Vec<cmds::TrackedProgress>),
    ProgressTrackingUpdate(cmds::TrackedProgress),
    ToggleGuiVisibility,
    StateChange(EditorState),
    Debug(DebugEvent),
}
// ----------------------------------------------------------------------------
enum DebugEvent {
    ClipmapUpdate(String, u8, Handle<texturearray::TextureArray>),
}
// ----------------------------------------------------------------------------
#[derive(Default)]
struct DefaultResources {
    logo: Handle<Image>,
    placeholder_texture: Handle<Image>,
}
// ----------------------------------------------------------------------------
// sync loader of essential files
fn setup_default_assets(
    mut egui_ctx: ResMut<EguiContext>,
    mut ui_images: ResMut<UiImages>,
    mut resources: ResMut<DefaultResources>,
    mut textures: ResMut<Assets<Image>>,
) -> Result<(), String> {
    use bevy::render::render_resource::{Extent3d, TextureDimension};

    info!("startup_system: setup_default_assets");

    let logo_resolution = 150;
    let texture_resolution = 1024;

    let logo_data = loader::LoaderPlugin::load_png_data(
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        logo_resolution,
        "assets/logo.png",
    )?;

    let logo = Image::new(
        Extent3d {
            width: logo_resolution,
            height: logo_resolution,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        logo_data,
        TextureFormat::Rgba8UnormSrgb,
    );

    resources.logo = textures.add(logo);

    ui_images.set(&mut egui_ctx, "logo", resources.logo.clone_weak());

    // default material texture placeholder
    let default_texture_data = loader::LoaderPlugin::load_png_data(
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        texture_resolution,
        "assets/placeholder_texture.png",
    )?;

    resources.placeholder_texture = textures.add(Image::new(
        Extent3d {
            width: texture_resolution,
            height: texture_resolution,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        default_texture_data,
        TextureFormat::Rgba8UnormSrgb,
    ));

    info!("startup_system: setup_default_assets.done");
    Ok(())
}
// ----------------------------------------------------------------------------
fn handle_setup_errors(
    In(result): In<Result<(), String>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    match result {
        Ok(_) => {}
        Err(msg) => {
            error!("failed to initialize default resources. {}", msg);
            app_exit_events.send(AppExit);
        }
    }
}
// ----------------------------------------------------------------------------
fn finish_initialization(mut app_state: ResMut<State<EditorState>>) {
    app_state.overwrite_set(EditorState::NoTerrainData).unwrap();
}
// ----------------------------------------------------------------------------
fn signal_editor_state_change(
    app_state: Res<State<EditorState>>,
    mut editor_events: EventWriter<EditorEvent>,
) {
    editor_events.send(EditorEvent::StateChange(*app_state.current()));
}
// ----------------------------------------------------------------------------
type TaskResult = Task<Result<TaskResultData, String>>;
// ----------------------------------------------------------------------------
enum TaskResultData {
    HeightmapData(heightmap::TerrainHeightMap),
    TextureControl(texturecontrol::TextureControl),
    TintMap(tintmap::TintMap),
}
// ----------------------------------------------------------------------------
fn setup_terrain_loading(
    terrain_config: Res<config::TerrainConfig>,
    mut mesh_settings: ResMut<terrain_tiles::TerrainMeshSettings>,
    mut editor_events: EventWriter<EditorEvent>,
    mut task_manager: ResMut<cmds::AsyncCommandManager>,
) {
    // auto setup some default lod count and error levels based on terrain size
    mesh_settings.setup_defaults_from_size(terrain_config.map_size());

    // queue loading tasks
    task_manager.add_new(cmds::WaitForTerrainLoaded::default().into());
    task_manager.add_new(cmds::LoadHeightmap::default().into());
    task_manager.add_new(cmds::LoadTextureMap::default().into());
    task_manager.add_new(cmds::LoadTintMap::default().into());
    task_manager.add_new(cmds::LoadTerrainMaterialSet::default().into());

    // bigger terrains may take > 10s of loading. show a progress bar by tracking
    // all longer running events
    editor_events.send(EditorEvent::ProgressTrackingStart(
        "Loading Terrain".into(),
        vec![
            cmds::TrackedProgress::LoadHeightmap(false),
            cmds::TrackedProgress::LoadTextureMap(false),
            cmds::TrackedProgress::LoadTintMap(false),
            cmds::TrackedProgress::GeneratedHeightmapNormals(0, 1),
            cmds::TrackedProgress::GenerateTerrainTiles(false),
            cmds::TrackedProgress::GeneratedTerrainErrorMaps(0, terrain_config.tile_count()),
            cmds::TrackedProgress::GeneratedTerrainMeshes(0, terrain_config.tile_count()),
        ],
    ));
}
// ----------------------------------------------------------------------------
fn watch_loading(
    mut app_state: ResMut<State<EditorState>>,
    mut tasks_finished: EventReader<AsyncTaskFinishedEvent>,
) {
    use AsyncTaskFinishedEvent::TerrainLoaded;
    if tasks_finished.iter().any(|t| matches!(t, TerrainLoaded)) {
        info!("terrain loaded.");
        app_state.overwrite_set(EditorState::Editing).ok();
    }
}
// ----------------------------------------------------------------------------
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DefaultResources>()
            .init_resource::<config::TerrainConfig>()
            .add_event::<EditorEvent>()
            .add_state(EditorState::Initialization)
            .add_plugin(compute::GpuComputeTaskPlugin)
            .add_plugin(cmds::AsyncCmdsPlugin)
            .add_plugin(texturearray::TextureArrayPlugin)
            .add_plugin(heightmap::HeightmapPlugin)
            .add_plugin(terrain_clipmap::TerrainClipmapPlugin)
            .add_plugin(terrain_material::MaterialSetPlugin)
            .add_plugin(terrain_tiles::TerrainTilesGeneratorPlugin)
            .add_plugin(terrain_render::TerrainRenderPlugin)
            .add_plugin(terrain_painting::TerrainPaintingPlugin)
            .insert_resource(camera::CameraSettings {
                rotation_sensitivity: 0.00015, // default: 0.00012
                movement_speed: 122.0,         // default: 12.0
                speed_modifier: 3.0,
            })
            .add_plugin(CameraPlugin)
            .add_plugin(gui::EditorUiPlugin)
            .insert_resource(atmosphere::AtmosphereMat::default())
            .add_plugin(atmosphere::AtmospherePlugin { dynamic: true })
            .add_plugin(environment::EnvironmentPlugin)
            .add_system(global_hotkeys);

        // --- state systems definition ---------------------------------------
        EditorState::initialization(app);
        EditorState::no_terrain_data(app);
        EditorState::terrain_loading(app);
        EditorState::terrain_editing(app);
        EditorState::free_cam(app);
        // --- state systems definition END -----------------------------------
    }
}
// ----------------------------------------------------------------------------
impl EditorState {
    // ------------------------------------------------------------------------
    /// init of default resources/placeholders etc. with explicit ordering
    fn initialization(app: &mut App) {
        app.add_startup_system(
            setup_default_assets
                .chain(handle_setup_errors)
                .label("default_resources"),
        )
        .add_startup_system(
            terrain_material::setup_default_materialset
                .label("default_materialset")
                .after("default_resources"),
        )
        .add_startup_system(
            gui::initialize_ui
                .label("init_ui")
                .after("default_materialset"),
        )
        // there is no update phase in initialization, just transit to next state
        .add_startup_system(finish_initialization.after("init_ui"))
        // plugins
        .add_startup_system_set(EnvironmentPlugin::startup());
    }
    // ------------------------------------------------------------------------
    /// close project / unload terrain state
    fn no_terrain_data(app: &mut App) {
        use EditorState::NoTerrainData;

        app.add_system_set(
            SystemSet::on_enter(NoTerrainData).with_system(signal_editor_state_change),
        );
        app.add_system_set(
            SystemSet::on_resume(NoTerrainData).with_system(signal_editor_state_change),
        );

        app // plugins
            .add_system_set(TerrainClipmapPlugin::reset_data(NoTerrainData))
            .add_system_set(TerrainTilesGeneratorPlugin::reset_data(NoTerrainData))
            .add_system_set(MaterialSetPlugin::setup_default_materialset(NoTerrainData))
            .add_system_set(EnvironmentPlugin::activate_dynamic_updates(NoTerrainData));
    }
    // ------------------------------------------------------------------------
    /// load project / terrain data state
    fn terrain_loading(app: &mut App) {
        use EditorState::TerrainLoading;

        app.add_system_set(
            SystemSet::on_enter(TerrainLoading)
                .with_system(signal_editor_state_change)
                .with_system(setup_terrain_loading),
        )
        // clipmap tracker must be intialized with new config data
        // before loading starts
        .add_system_set(TerrainClipmapPlugin::init_tracker(TerrainLoading));

        app.add_system_set(
            SystemSet::on_update(TerrainLoading)
                .with_system(cmds::start_async_operations)
                .with_system(cmds::poll_async_task_state)
                .with_system(watch_loading),
        )
        // plugins
        .add_system_set(MaterialSetPlugin::terrain_material_loading(TerrainLoading))
        .add_system_set(HeightmapPlugin::generate_heightmap_normals(TerrainLoading))
        .add_system_set(TerrainTilesGeneratorPlugin::lazy_generation(TerrainLoading));
    }
    // ------------------------------------------------------------------------
    /// main editing state
    fn terrain_editing(app: &mut App) {
        use EditorState::Editing;

        app.add_system_set(SystemSet::on_enter(Editing).with_system(signal_editor_state_change));
        app.add_system_set(SystemSet::on_resume(Editing).with_system(signal_editor_state_change));

        // required for triggering operations like mesh regeneration or reloading
        // parts of some other data without reloading complete terrain
        app.add_system_set(
            SystemSet::on_update(Editing)
                .with_system(cmds::start_async_operations)
                .with_system(cmds::poll_async_task_state),
        );

        app // plugins
            .add_system_set(EnvironmentPlugin::activate_dynamic_updates(Editing))
            .add_system_set(MaterialSetPlugin::terrain_material_loading(Editing))
            .add_system_set(TerrainClipmapPlugin::update_tracker(Editing))
            .add_system_set(TerrainTilesGeneratorPlugin::lazy_generation(Editing))
            .add_system_set(TerrainPaintingPlugin::process_brush_operations(Editing));
    }
    // ------------------------------------------------------------------------
    /// stacked state with active free cam (editing on hold)
    fn free_cam(app: &mut App) {
        use EditorState::FreeCam;

        app.add_system_set(SystemSet::on_enter(FreeCam).with_system(signal_editor_state_change))
            .add_system_set(
                SystemSet::on_enter(FreeCam).with_system(terrain_clipmap::enable_caching),
            );

        app // plugins
            .add_system_set(CameraPlugin::start_free_camera(FreeCam))
            .add_system_set(CameraPlugin::active_free_camera(FreeCam))
            .add_system_set(CameraPlugin::stop_free_camera(FreeCam))
            .add_system_set(EnvironmentPlugin::activate_dynamic_updates(FreeCam))
            .add_system_set(TerrainClipmapPlugin::update_tracker(FreeCam))
            .add_system_set(TerrainTilesGeneratorPlugin::lazy_generation(FreeCam));
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[allow(clippy::single_match)]
fn global_hotkeys(
    keys: Res<Input<KeyCode>>,
    mut app_state: ResMut<State<EditorState>>,
    mut event: EventWriter<EditorEvent>,
) {
    use EditorState::*;

    for key in keys.get_just_pressed() {
        match app_state.current() {
            FreeCam => match key {
                KeyCode::F12 => event.send(EditorEvent::ToggleGuiVisibility),
                KeyCode::LControl => app_state.overwrite_pop().unwrap(),
                _ => {}
            },
            Editing => match key {
                KeyCode::F12 => event.send(EditorEvent::ToggleGuiVisibility),
                KeyCode::LControl => app_state.overwrite_push(FreeCam).unwrap(),
                _ => (),
            },
            TerrainLoading => match key {
                KeyCode::F12 => event.send(EditorEvent::ToggleGuiVisibility),
                _ => (),
            },
            NoTerrainData => match key {
                KeyCode::F12 => event.send(EditorEvent::ToggleGuiVisibility),
                KeyCode::LControl => app_state.overwrite_push(FreeCam).unwrap(),
                _ => (),
            },
            Initialization => {}
        }
    }
}
// ----------------------------------------------------------------------------
