// ----------------------------------------------------------------------------
use bevy::prelude::*;
// ----------------------------------------------------------------------------
pub struct EditorPlugin;
// ----------------------------------------------------------------------------
use camera::CameraPlugin;

use gui::GuiAction;
// ----------------------------------------------------------------------------
mod atmosphere;
mod camera;
mod config;

mod terrain_material;

mod texturearray;

mod gui;
// ----------------------------------------------------------------------------
#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum EditorState {
    Initialization,
    Editing,
}
// ----------------------------------------------------------------------------
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<config::TerrainConfig>()
            .add_state(EditorState::Editing)
            .add_plugin(texturearray::TextureArrayPlugin)
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
        EditorState::terrain_editing(app);
        // --- state systems definition END -----------------------------------
    }
}
// ----------------------------------------------------------------------------
impl EditorState {
    // ------------------------------------------------------------------------
    /// main editing state
    fn terrain_editing(app: &mut App) {
        use EditorState::Editing;

        app.add_system_set(
            SystemSet::on_update(Editing)
                .with_system(hotkeys)
                .with_system(daylight_cycle)
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
