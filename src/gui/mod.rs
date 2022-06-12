// ----------------------------------------------------------------------------
use bevy::{math::Vec3, prelude::*};
use bevy_egui::EguiContext;

use crate::atmosphere::AtmosphereMat;
use crate::cmds;
use crate::config;
use crate::environment::DayNightCycle;
use crate::environment::SunPositionSettings;
use crate::terrain_material::{TerrainMaterialSet, TextureType, TextureUpdatedEvent};
use crate::terrain_tiles::{LodSlot, TerrainMeshSettings};
use crate::texturearray::TextureArray;
use crate::{EditorEvent, EditorState};
// ----------------------------------------------------------------------------
pub struct EditorUiPlugin;
// ----------------------------------------------------------------------------
pub use self::images::UiImages;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct UiState {
    fullscreen: bool,
    enabled: bool,
    project_open: bool,
    project_is_loading: bool,

    progress: ProgressTracking,

    toolbox: toolbox::ToolboxState,

    is_using_pointer: bool,
    wants_pointer: bool,
    wants_keyboard: bool,

    // debug
    debug: debug::DebugUiState,
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
/// Events triggered by user in the GUI (user actions)
pub enum GuiAction {
    Toolbox(toolbox::ToolboxAction),
    UpdateDayNightCycleSetting(DayNightCycleSetting),
    UpdateSunSetting(SunSetting),
    UpdateAtmosphereSetting(AtmosphereSetting),
    UpdateMeshSetting(MeshSetting),
    UpdateRenderSettings(RenderSetting),
    QuitRequest,
    DebugCloseProject,
    DebugLoadTerrain(Box<config::TerrainConfig>),
    DebugShowClipmap(bool),
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub enum DayNightCycleSetting {
    SetTimeOfDay(f32),
    SetCycleSpeed(u16),
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub enum SunSetting {
    SetPlaneTilt(u16),
    SetPlaneYaw(u16),
    SetPlaneHeight(u16),
    ToggleDebugMesh,
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
#[derive(Debug)]
pub enum MeshSetting {
    SetLodCount(u8),
    SetLodMinError(f32),
    SetLodMaxError(f32),
    SetLodMaxDistance(f32),
    SetLodError(LodSlot, f32),
    SetLodDistance(LodSlot, f32),
    FreezeLods,
    ResetToDefault,
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
// Note: as of now other settings are directly flipped
pub enum RenderSetting {
    OverlayWireframe(bool),
}
// ----------------------------------------------------------------------------
use self::egui_extensions::UiExtension;

use self::progresstracking::ProgressTracking;
// ----------------------------------------------------------------------------
mod debug;

mod toolbox;

mod images;
mod progresstracking;
mod update;
mod view;

mod egui_extensions;
// ----------------------------------------------------------------------------
impl Plugin for EditorUiPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.add_plugin(bevy_egui::EguiPlugin)
            .add_plugin(debug::EditorUiDebugPlugin)
            .add_plugin(toolbox::TexturingToolboxPlugin)
            .init_resource::<UiState>()
            .init_resource::<UiImages>()
            .add_event::<GuiAction>()
            .add_system(view::show_ui.label("gui_actions"))
            .add_system(update_input_processing_request.after("gui_actions"))
            .add_system(handle_editor_events)
            .add_system(log_ui_actions.after("gui_actions"))
            .add_system(
                handle_ui_actions
                    .label("handle_ui_actions")
                    .after("gui_actions"),
            );
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
    mut ui_state: ResMut<UiState>,
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
            ProgressTrackingStart(name, subtasks) => {
                ui_state.progress.start_task_tracking(name, subtasks);
            }
            ProgressTrackingUpdate(update) => {
                ui_state.progress.update(update);
            }
            StateChange(new_state) => ui_state.update(*new_state),
            ToggleGuiVisibility => {
                ui_state.fullscreen = !ui_state.fullscreen;
            }

            Debug(_) => {}
        }
    }
}
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn handle_ui_actions(
    mut ui_action: EventReader<GuiAction>,
    mut daylight_cycle: ResMut<DayNightCycle>,
    mut sun_settings: Option<ResMut<SunPositionSettings>>,
    mut atmosphere_settings: Option<ResMut<AtmosphereMat>>,
    mut mesh_settings: Option<ResMut<TerrainMeshSettings>>,
    mut task_manager: ResMut<cmds::AsyncCommandManager>,
) {
    for action in ui_action.iter() {
        match action {
            GuiAction::Toolbox(_action) => {
                // handled by toolbox module explicitely
                // this would be the place for a reactive one shot system
            }
            GuiAction::QuitRequest => {
                warn!("TODO quit request");
            }
            GuiAction::UpdateDayNightCycleSetting(setting) => {
                update::update_daynight_cycle_settings(setting, &mut daylight_cycle)
            }
            GuiAction::UpdateSunSetting(setting) => {
                update::update_sun_settings(setting, &mut sun_settings)
            }
            GuiAction::UpdateAtmosphereSetting(setting) => {
                update::update_atmosphere_settings(setting, &mut atmosphere_settings)
            }
            GuiAction::UpdateMeshSetting(setting) => {
                update::update_mesh_settings(setting, &mut mesh_settings)
            }
            GuiAction::UpdateRenderSettings(setting) => {
                update::update_render_settings(setting, &mut *task_manager)
            }

            // TODO should be removed later
            GuiAction::DebugLoadTerrain(_)
            | GuiAction::DebugCloseProject
            | GuiAction::DebugShowClipmap(_) => {}
        }
    }
}
// ----------------------------------------------------------------------------
impl UiState {
    // ------------------------------------------------------------------------
    fn update(&mut self, editor_state: EditorState) {
        match editor_state {
            EditorState::Initialization => {}
            EditorState::NoTerrainData => {
                self.enabled = true;
                self.project_is_loading = false;
                self.project_open = false;
                self.toolbox.enabled = false;
            }
            EditorState::TerrainLoading => {
                self.enabled = true;
                self.toolbox.enabled = false;
                self.project_open = false;
                self.project_is_loading = true;
            }
            EditorState::Editing => {
                self.enabled = true;
                self.toolbox.enabled = true;
                self.project_open = true;
                self.project_is_loading = false;
            }
            EditorState::FreeCam => {
                self.enabled = false;
                self.toolbox.enabled = false;
            }
            EditorState::AutoFlyCamera => {
                self.enabled = true;
                self.toolbox.enabled = false;
            }
        }
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn wants_input(&self) -> bool {
        self.wants_pointer || self.wants_keyboard || self.is_using_pointer
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
fn update_input_processing_request(
    mut egui_ctx: ResMut<EguiContext>,
    mut ui_state: ResMut<UiState>,
) {
    let is_using_pointer = egui_ctx.ctx_mut().is_using_pointer();
    let wants_pointer = egui_ctx.ctx_mut().wants_pointer_input();
    let wants_keyboard = egui_ctx.ctx_mut().wants_keyboard_input();

    if ui_state.wants_pointer != wants_pointer
        || ui_state.wants_keyboard != wants_keyboard
        || ui_state.is_using_pointer != is_using_pointer
    {
        ui_state.wants_pointer = wants_pointer;
        ui_state.wants_keyboard = wants_keyboard;
        ui_state.is_using_pointer = is_using_pointer;
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
