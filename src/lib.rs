// ----------------------------------------------------------------------------
use bevy::{app::AppExit, prelude::*, render::render_resource::TextureFormat, tasks::Task};
use bevy_egui::EguiContext;
// ----------------------------------------------------------------------------
pub struct EditorPlugin;
// ----------------------------------------------------------------------------
use camera::CameraPlugin;

use cmds::AsyncTaskFinishedEvent;
use gui::{GuiAction, UiImages};
// ----------------------------------------------------------------------------
mod atmosphere;
mod camera;
mod config;
mod loader;

mod terrain_material;

mod resource;
mod texturearray;

mod cmds;
mod gui;
// ----------------------------------------------------------------------------
#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum EditorState {
    Initialization,
    TerrainLoading,
    Editing,
}
// ----------------------------------------------------------------------------
#[derive(Default)]
struct DefaultResources {
    logo: Handle<Image>,
    // placeholder_texture: Handle<Image>,
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
    // resources.placeholder_texture = textures.add();

    ui_images.set(&mut egui_ctx, "logo", resources.logo.clone_weak());

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
type TaskResult = Task<Result<TaskResultData, String>>;
// ----------------------------------------------------------------------------
enum TaskResultData {}
// ----------------------------------------------------------------------------
fn setup_terrain_loading(mut task_manager: ResMut<cmds::AsyncCommandManager>) {
    task_manager.add_new(cmds::WaitForTerrainLoaded::default().into());
    task_manager.add_new(cmds::LoadTerrainMaterialSet::default().into());
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
            .add_state(EditorState::Initialization)
            .add_startup_system(
                setup_default_assets
                    .chain(handle_setup_errors)
                    .label("default_resources"),
            )
            .add_plugin(cmds::AsyncCmdsPlugin)
            .add_plugin(texturearray::TextureArrayPlugin)
            .add_plugin(terrain_material::MaterialSetPlugin)
            .insert_resource(camera::CameraSettings {
                rotation_sensitivity: 0.00015, // default: 0.00012
                movement_speed: 122.0,         // default: 12.0
                speed_modifier: 3.0,
            })
            .add_plugin(CameraPlugin)
            .add_plugin(gui::EditorUiPlugin)
            .insert_resource(atmosphere::AtmosphereMat::default())
            .add_plugin(atmosphere::AtmospherePlugin { dynamic: true })
            .init_resource::<SunSettings>()
            .add_startup_system(setup_lighting_environment);

        // --- state systems definition ---------------------------------------
        EditorState::initialization(app);
        EditorState::terrain_loading(app);
        EditorState::terrain_editing(app);
        // --- state systems definition END -----------------------------------
    }
}
// ----------------------------------------------------------------------------
impl EditorState {
    // ------------------------------------------------------------------------
    /// init of default resources/placeholders etc.
    fn initialization(app: &mut App) {
        use EditorState::Initialization;

        app.add_system_set(
            terrain_material::MaterialSetPlugin::setup_default_materialset(Initialization),
        );
    }
    // ------------------------------------------------------------------------
    /// load project / terrain data state
    fn terrain_loading(app: &mut App) {
        use EditorState::TerrainLoading;

        app.add_system_set(SystemSet::on_enter(TerrainLoading).with_system(setup_terrain_loading));

        app.add_system_set(
            SystemSet::on_update(TerrainLoading)
                //TODO the following or a slimmed down version should probably be available in all states
                .with_system(cmds::start_async_operations)
                .with_system(cmds::poll_async_task_state)
                .with_system(watch_loading),
        )
        // plugins
        .add_system_set(CameraPlugin::active_free_camera(TerrainLoading));
    }
    // ------------------------------------------------------------------------
    /// main editing state
    fn terrain_editing(app: &mut App) {
        use EditorState::Editing;

        app.add_system_set(
            SystemSet::on_update(Editing)
                .with_system(hotkeys)
                .with_system(daylight_cycle),
        )
        // plugins
        .add_system_set(CameraPlugin::active_free_camera(Editing));
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[allow(clippy::single_match)]
fn hotkeys(keys: Res<Input<KeyCode>>, mut gui_event: EventWriter<GuiAction>) {
    for key in keys.get_just_pressed() {
        match key {
            KeyCode::F12 => gui_event.send(GuiAction::ToggleFullscreen),
            _ => (),
        }
    }
}
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// atmosphere tests (TODO rework)
// ----------------------------------------------------------------------------
// Marker for updating the position of the light, not needed unless we have multiple lights
#[derive(Component)]
struct Sun;
// ----------------------------------------------------------------------------
struct SunSettings {
    cycle_active: bool,
    cycle_speed: f32,
    pos: f32,
    distance: f32,
}
// ----------------------------------------------------------------------------
impl Default for SunSettings {
    fn default() -> Self {
        Self {
            cycle_active: true,
            cycle_speed: 4.0,
            pos: 0.25,
            distance: 10.0,
        }
    }
}
// ----------------------------------------------------------------------------
fn daylight_cycle(
    mut sky_mat: ResMut<atmosphere::AtmosphereMat>,
    mut settings: ResMut<SunSettings>,
    mut query: Query<&mut Transform, With<Sun>>,
    time: Res<Time>,
) {
    if let Some(mut light_trans) = query.iter_mut().next() {
        use std::f32::consts::PI;

        let basepos = Vec3::new(0.0, 0.0, 0.0);
        let mut pos = (light_trans.translation - basepos) / ((11.0 - settings.distance) * 10000.0);

        if settings.cycle_active {
            let t = time.time_since_startup().as_millis() as f32
                / ((11.0 - settings.cycle_speed) * 500.0);
            pos.y = t.sin();
            pos.z = t.cos();
            settings.pos = (t / (2.0 * PI)) % 1.0;
        } else {
            let current = 2.0 * PI * settings.pos;
            pos.y = current.sin();
            pos.z = current.cos();
        }

        sky_mat.set_sun_position(pos);

        light_trans.translation = basepos + pos * (settings.distance * 10000.0);
    }
}
// ----------------------------------------------------------------------------
// Simple environment
fn setup_lighting_environment(mut commands: Commands) {
    info!("startup_system: setup_lighting_environment");
    // Our Sun
    commands
        .spawn()
        .insert(GlobalTransform::default())
        .insert(Transform::default())
        .insert(Sun);
}
// ----------------------------------------------------------------------------
