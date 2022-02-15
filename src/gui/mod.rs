// ----------------------------------------------------------------------------
use bevy::{math::Vec3, prelude::*};
use bevy_egui::EguiContext;

use crate::atmosphere::AtmosphereMat;
use crate::config;
use crate::terrain_material::{MaterialSlot, TerrainMaterialSet, TextureType, TextureUpdatedEvent};
use crate::texturearray::TextureArray;
use crate::SunSettings;
use crate::{EditorEvent, EditorState};
// ----------------------------------------------------------------------------
pub struct EditorUiPlugin;
// ----------------------------------------------------------------------------
pub use self::images::UiImages;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct UiState {
    fullscreen: bool,

    // FIXME this should be some kind of brush state
    selected_slot: Option<MaterialSlot>,
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
/// Events triggered by user in the GUI (user actions)
pub enum GuiAction {
    SelectMaterial(MaterialSlot),
    UnselectMaterial,
    UpdateMaterial(MaterialSlot, MaterialSetting),
    UpdateSunSetting(SunSetting),
    UpdateAtmosphereSetting(AtmosphereSetting),
    ToggleFullscreen,
    QuitRequest,
    DebugLoadTerrain(Box<config::TerrainConfig>),
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum MaterialSetting {
    SetBlendSharpness(f32),
    SetSlopeBaseDampening(f32),
    SetSlopeNormalDampening(f32),
    SetSpecularityScale(f32),
    SetSpecularity(f32),
    SetSpecularityBase(f32),
    SetFalloff(f32),
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
mod images;
mod update;
mod view;
// ----------------------------------------------------------------------------
impl Plugin for EditorUiPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.add_plugin(bevy_egui::EguiPlugin)
            .init_resource::<UiState>()
            .init_resource::<UiImages>()
            .add_event::<GuiAction>()
            .add_system(view::show_ui.label("gui_actions"))
            .add_system(handle_editor_events)
            .add_system(log_ui_actions.after("gui_actions"))
            .add_system(handle_ui_actions.after("gui_actions"))
            .add_system(handle_ui_debug_actions.after("gui_actions"));
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
const TEXTURE_PREVIEW_SIZE_SMALL: u32 = 64;
// ----------------------------------------------------------------------------
pub(super) fn initialize_ui(
    mut egui_ctx: ResMut<EguiContext>,
    mut egui_image_registry: ResMut<UiImages>,
    mut images: ResMut<Assets<Image>>,
    texture_arrays: ResMut<Assets<TextureArray>>,
    materialset: ResMut<TerrainMaterialSet>,
) {
    info!("startup_system: initialize_ui");

    // setup egui link to terrain texture preview images
    for (array_handle, texture_type) in [
        (&materialset.diffuse, TextureType::Diffuse),
        (&materialset.normal, TextureType::Normal),
    ] {
        if let Some(array) = texture_arrays.get(array_handle) {
            for i in 0..array.texture_count() {
                let (format, size, img_data) = array.imagedata(i as u8, TEXTURE_PREVIEW_SIZE_SMALL);

                egui_image_registry.add_image(
                    &mut egui_ctx,
                    &mut *images,
                    format!("terraintexture.{}.{}", texture_type, i),
                    format,
                    (size, size),
                    img_data,
                );
            }
        }

        // if let Some(array) = texture_arrays.get(&materialset.normal) {
        //     for i in 0..array.texture_count() {
        //         let (format, size, img_data) = array.imagedata(i as u8, TEXTURE_PREVIEW_SIZE_SMALL);

        //         egui_image_registry.add_image(
        //             &mut egui_ctx,
        //             &mut *images,
        //             format!("terraintexture.{}.{}", TextureType::Normal, i),
        //             format,
        //             (size, size),
        //             img_data,
        //         );
        //     }
        // }
    }
}
// ----------------------------------------------------------------------------
fn handle_editor_events(
    mut egui_image_registry: ResMut<UiImages>,
    mut images: ResMut<Assets<Image>>,
    texture_arrays: Res<Assets<TextureArray>>,
    materialset: Res<TerrainMaterialSet>,
    mut events: EventReader<EditorEvent>,
) {
    use EditorEvent::*;

    for event in events.iter() {
        match event {
            TerrainTextureUpdated(TextureUpdatedEvent(slot, texture_ty)) => {
                let handle = match texture_ty {
                    TextureType::Diffuse => &materialset.diffuse,
                    // TODO ignore normal?
                    TextureType::Normal => &materialset.normal,
                };

                if let Some(array) = texture_arrays.get(handle) {
                    let (_, _, img_data) = array.imagedata(**slot, TEXTURE_PREVIEW_SIZE_SMALL);
                    egui_image_registry.update_image(
                        &mut *images,
                        &format!("terraintexture.{}.{}", texture_ty, slot),
                        img_data,
                    );
                }
            }
        }
    }
}
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn handle_ui_actions(
    mut ui_state: ResMut<UiState>,
    mut ui_action: EventReader<GuiAction>,
    mut materialset: Option<ResMut<TerrainMaterialSet>>,
    mut sun_settings: Option<ResMut<SunSettings>>,
    mut atmosphere_settings: Option<ResMut<AtmosphereMat>>,
) {
    for action in ui_action.iter() {
        match action {
            GuiAction::SelectMaterial(slot) => {
                ui_state.selected_slot = Some(*slot);
            }
            GuiAction::UnselectMaterial => {
                ui_state.selected_slot = None;
            }
            GuiAction::UpdateMaterial(slot, setting) => {
                update::update_material_settings(*slot, setting, &mut materialset);
            }
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
            // TODO should be removed late
            GuiAction::DebugLoadTerrain(_) => {}
        }
    }
}
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn handle_ui_debug_actions(
    mut ui_action: EventReader<GuiAction>,
    mut app_state: ResMut<State<EditorState>>,
    mut worldconf: ResMut<config::TerrainConfig>,
) {
    for action in ui_action.iter() {
        match action {
            GuiAction::DebugLoadTerrain(new_config) => {
                *worldconf = (**new_config).clone();
                app_state.overwrite_set(EditorState::TerrainLoading).ok();
            }
            _ => {}
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
