// ----------------------------------------------------------------------------
use bevy::{math::Vec3, prelude::*};

use crate::{atmosphere::AtmosphereMat, SunSettings};
// ----------------------------------------------------------------------------
pub struct EditorUiPlugin;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct UiState {
    fullscreen: bool,
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub enum GuiAction {
    UpdateSunSetting(SunSetting),
    UpdateAtmosphereSetting(AtmosphereSetting),
    ToggleFullscreen,
    QuitRequest,
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub enum SunSetting {
    SetPosition(f32),
    SetDistance(f32),
    ToggleCycle,
    SetCycleSpeed(f32),
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub enum AtmosphereSetting {
    SetRayOrigin(Vec3),
    SetSunIntensity(f32),
    SetPlanetRadius(f32),
    SetAtmosphereRadius(f32),
    SetRayleighScattering(Vec3),
    SetRayleighScaleHeight(f32),
    SetMieScattering(f32),
    SetMieScaleHeight(f32),
    SetMieScatteringDirection(f32),
    ResetToDefault,
}
// ----------------------------------------------------------------------------
mod update;
mod view;
// ----------------------------------------------------------------------------
impl Plugin for EditorUiPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.add_plugin(bevy_egui::EguiPlugin)
            .init_resource::<UiState>()
            .add_event::<GuiAction>()
            .add_startup_system(initialize_ui.after("initialize_render_pipeline"))
            .add_system(view::show_ui.label("gui_actions"))
            .add_system(log_ui_actions.after("gui_actions"))
            .add_system(handle_ui_actions.after("gui_actions"));
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
fn initialize_ui() {
    info!("startup_system: initialize_ui");
}
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn handle_ui_actions(
    mut ui_state: ResMut<UiState>,
    mut ui_action: EventReader<GuiAction>,
    mut sun_settings: Option<ResMut<SunSettings>>,
    mut atmosphere_settings: Option<ResMut<AtmosphereMat>>,
) {
    for action in ui_action.iter() {
        match action {
            GuiAction::ToggleFullscreen => {
                ui_state.fullscreen = !ui_state.fullscreen;
            }
            GuiAction::QuitRequest => {
                warn!("TODO quit request");
            }
            GuiAction::UpdateSunSetting(setting) => {
                update::update_sun_settings(setting, &mut sun_settings)
            }
            GuiAction::UpdateAtmosphereSetting(setting) => {
                update::update_atmosphere_settings(setting, &mut atmosphere_settings)
            }
        }
    }
}
// ----------------------------------------------------------------------------
// debug
// ----------------------------------------------------------------------------
#[allow(dead_code)]
fn log_ui_actions(mut ui_action: EventReader<GuiAction>) {
    for ev in ui_action.iter() {
        debug!("UI Action {:?}", ev);
    }
}
// ----------------------------------------------------------------------------
