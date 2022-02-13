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
        app.add_state(EditorState::Initialization)
            .insert_resource(camera::CameraSettings {
                rotation_sensitivity: 0.00015, // default: 0.00012
                movement_speed: 122.0,         // default: 12.0
                speed_modifier: 3.0,
            })
            .add_plugin(CameraPlugin)
            .insert_resource(atmosphere::AtmosphereMat::default())
            .add_plugin(atmosphere::AtmospherePlugin { dynamic: true });

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

        app.add_system_set(CameraPlugin::active_free_camera(Editing));
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
