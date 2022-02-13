// ----------------------------------------------------------------------------
// based on bevy_flycam (https://github.com/sburris0/bevy_flycam)
// ----------------------------------------------------------------------------
use bevy::prelude::*;

use bevy::app::{Events, ManualEventReader};
use bevy::core::Time;
use bevy::ecs::schedule::StateData;
use bevy::input::mouse::MouseMotion;
use bevy::input::Input;
use bevy::math::{Quat, Vec3};
use bevy::render::camera::{
    Camera, CameraProjection, PerspectiveCameraBundle, PerspectiveProjection,
};
use bevy::render::primitives::Frustum;
use bevy::render::view::VisibleEntities;
use bevy::window::{Window, Windows};
// ----------------------------------------------------------------------------
pub struct CameraPlugin;
// ----------------------------------------------------------------------------
pub struct CameraSettings {
    pub rotation_sensitivity: f32,
    pub movement_speed: f32,
    pub speed_modifier: f32,
}
// ----------------------------------------------------------------------------
#[derive(Default)]
struct CameraState {
    reader_motion: ManualEventReader<MouseMotion>,
    pitch: f32,
    yaw: f32,
}
// ----------------------------------------------------------------------------
#[derive(Component)]
pub struct FreeCam;
// ----------------------------------------------------------------------------
impl CameraPlugin {
    // ------------------------------------------------------------------------
    /// normal active cam operation
    pub fn active_free_camera<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_update(state)
            .with_system(camera_movement)
            .with_system(camera_mouse_rotation)
            .with_system(toggle_freecam)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Plugin for CameraPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraState>()
            .init_resource::<CameraSettings>()
            .add_startup_system(setup_cam);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// startup systems
// ----------------------------------------------------------------------------
fn setup_cam(mut commands: Commands, mut state: ResMut<CameraState>) {
    let camera_transform =
        Transform::from_xyz(0.0, 22.5, 0.0).looking_at(Vec3::new(10.0, 20.0, 7.5), Vec3::Y);

    // extract initial settings from current cam
    let (yaw, pitch) = get_yaw_pitch(&camera_transform.rotation);
    state.pitch = pitch.to_radians();
    state.yaw = yaw.to_radians();

    let perspective_projection = PerspectiveProjection {
        far: 16384.0,
        ..Default::default()
    };

    let view_projection = perspective_projection.get_projection_matrix();
    let frustum = Frustum::from_view_projection(
        &view_projection,
        &Vec3::ZERO,
        &Vec3::Z,
        perspective_projection.far(),
    );

    let perspective_cam = PerspectiveCameraBundle {
        camera: Camera {
            name: Some(bevy::render::camera::CameraPlugin::CAMERA_3D.to_string()),
            near: perspective_projection.near,
            far: perspective_projection.far,
            ..Default::default()
        },
        perspective_projection,
        visible_entities: VisibleEntities::default(),
        frustum,
        transform: camera_transform,
        global_transform: Default::default(),
    };

    commands
        .spawn_bundle(perspective_cam)
        .insert(FreeCam);
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
fn toggle_freecam(keys: Res<Input<KeyCode>>, mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    if keys.just_pressed(KeyCode::LControl) {
        toggle_grab_cursor(window);
    }
}
// ----------------------------------------------------------------------------
/// Handles keyboard input and movement
fn camera_movement(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    windows: Res<Windows>,
    settings: Res<CameraSettings>,
    mut query: Query<&mut Transform, With<FreeCam>>,
) {
    let window = windows.get_primary().unwrap();
    if window.cursor_locked() {
        for mut transform in query.iter_mut() {
            let mut velocity = Vec3::ZERO;
            let local_z = transform.local_z();
            let forward = -Vec3::new(local_z.x, 0., local_z.z);
            let right = Vec3::new(local_z.z, 0., -local_z.x);
            let mut modifier = 1.0;

            for key in keys.get_pressed() {
                match key {
                    KeyCode::W => velocity += forward,
                    KeyCode::S => velocity -= forward,
                    KeyCode::A => velocity -= right,
                    KeyCode::D => velocity += right,
                    KeyCode::Q => velocity -= Vec3::Y,
                    KeyCode::E => velocity += Vec3::Y,
                    KeyCode::LAlt => modifier = 1.0 / settings.speed_modifier,
                    KeyCode::LShift => modifier = settings.speed_modifier,
                    _ => (),
                }
            }
            velocity = velocity.normalize_or_zero();
            transform.translation +=
                velocity * time.delta_seconds() * settings.movement_speed * modifier
        }
    }
}
// ----------------------------------------------------------------------------
/// Handles looking around if cursor is locked
fn camera_mouse_rotation(
    settings: Res<CameraSettings>,
    windows: Res<Windows>,
    mut state: ResMut<CameraState>,
    motion: Res<Events<MouseMotion>>,
    mut query: Query<&mut Transform, With<FreeCam>>,
) {
    let window = windows.get_primary().unwrap();
    if window.cursor_locked() {
        for mut transform in query.iter_mut() {
            for ev in state.reader_motion.iter(&motion) {
                // Using smallest of height or width ensures equal vertical and horizontal sensitivity
                let window_scale = window.height().min(window.width());

                state.pitch -=
                    (settings.rotation_sensitivity * ev.delta.y * window_scale).to_radians();
                state.yaw -=
                    (settings.rotation_sensitivity * ev.delta.x * window_scale).to_radians();

                state.pitch = state.pitch.clamp(-1.54, 1.54);

                // Order is important to prevent unintended roll
                transform.rotation = Quat::from_axis_angle(Vec3::Y, state.yaw)
                    * Quat::from_axis_angle(Vec3::X, state.pitch);
            }
        }
    }
}
// ----------------------------------------------------------------------------
// utils
// ----------------------------------------------------------------------------
/// Grabs/ungrabs mouse cursor
fn toggle_grab_cursor(window: &mut Window) {
    window.set_cursor_lock_mode(!window.cursor_locked());
    window.set_cursor_visibility(!window.cursor_visible());
}
// ----------------------------------------------------------------------------
// from https://github.com/mcpar-land/bevy_fly_camera/pull/15/files
fn get_yaw_pitch(rotation: &Quat) -> (f32, f32) {
    let q = rotation;
    let sinp = 2.0 * (q.w * q.x - q.y * q.z);
    let pitch = sinp.asin();
    let siny_cosp = 2.0 * (q.w * q.y + q.z * q.x);
    let cosy_cosp = 1.0 - 2.0 * (q.x * q.x + q.y * q.y);
    let yaw = siny_cosp.atan2(cosy_cosp);
    (yaw.to_degrees(), pitch.to_degrees())
}
// ----------------------------------------------------------------------------
impl Default for CameraSettings {
    // ------------------------------------------------------------------------
    fn default() -> Self {
        Self {
            rotation_sensitivity: 0.00012,
            movement_speed: 12.,
            speed_modifier: 2.0,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
