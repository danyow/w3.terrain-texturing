// ----------------------------------------------------------------------------
use bevy::prelude::*;
// ----------------------------------------------------------------------------
pub struct EditorPlugin;
// ----------------------------------------------------------------------------
use camera::CameraPlugin;
// ----------------------------------------------------------------------------
mod atmosphere;
mod camera;
// ----------------------------------------------------------------------------
#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum EditorState {
    Initialization,
    Editing,
}
// ----------------------------------------------------------------------------
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_state(EditorState::Editing)
            .insert_resource(camera::CameraSettings {
                rotation_sensitivity: 0.00015, // default: 0.00012
                movement_speed: 122.0,         // default: 12.0
                speed_modifier: 3.0,
            })
            .add_plugin(CameraPlugin)
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

        app.add_system_set(SystemSet::on_update(Editing).with_system(daylight_cycle))
            // plugins
            .add_system_set(CameraPlugin::active_free_camera(Editing));
    }
    // ------------------------------------------------------------------------
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
fn setup_lighting_environment(
    mut commands: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("startup_system: setup_lighting_environment");
    // Our Sun
    commands
        .spawn()
        // .insert_bundle(bevy::pbr::PbrBundle {
        //     mesh: meshes.add(Mesh::from(bevy::prelude::shape::Icosphere {
        //         radius: 100.0,
        //         // radius: -10.0,
        //         subdivisions: 5
        //     })),
        //     material: materials.add(StandardMaterial {
        //         emissive: Color::rgb(1.0, 1.0, 0.79),
        //         ..Default::default()
        //     }),
        //     ..Default::default()
        // })
        .insert(GlobalTransform::default())
        .insert(Transform::default())
        .insert(Sun);
}
// ----------------------------------------------------------------------------
