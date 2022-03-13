// ----------------------------------------------------------------------------
use bevy::{math::Vec3, prelude::*};
use bevy_egui::EguiContext;

use crate::atmosphere::AtmosphereMat;
use crate::config;
use crate::terrain_clipmap::{TextureControlClipmap, TintClipmap};
use crate::terrain_material::{MaterialSlot, TerrainMaterialSet, TextureType, TextureUpdatedEvent};
use crate::terrain_tiles::{LodSlot, TerrainMeshSettings};
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
    project_open: bool,
    project_is_loading: bool,

    progress: ProgressTracking,
    // FIXME this should be some kind of brush state
    selected_slot: Option<MaterialSlot>,

    // debug
    debug_show_clipmaps: bool,
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
    UpdateMeshSetting(MeshSetting),
    ToggleFullscreen,
    QuitRequest,
    DebugCloseProject,
    DebugLoadTerrain(Box<config::TerrainConfig>),
    DebugShowClipmap(bool),
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
use self::progresstracking::ProgressTracking;
// ----------------------------------------------------------------------------
mod images;
mod progresstracking;
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

            Debug(crate::DebugEvent::ClipmapUpdate(clipmap_label, slot, handle)) => {
                if let Some(array) = texture_arrays.get(handle) {
                    let (_, _, img_data) = array.imagedata(*slot, config::CLIPMAP_SIZE);
                    egui_image_registry.update_image(
                        &mut *images,
                        &format!("clipmap.{}.{}", clipmap_label, slot),
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
    mut mesh_settings: Option<ResMut<TerrainMeshSettings>>,
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
            GuiAction::UpdateMeshSetting(setting) => {
                update::update_mesh_settings(setting, &mut mesh_settings)
            }
            // TODO should be removed late
            GuiAction::DebugLoadTerrain(_)
            | GuiAction::DebugCloseProject
            | GuiAction::DebugShowClipmap(_) => {}
        }
    }
}
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn handle_ui_debug_actions(
    mut ui_state: ResMut<UiState>,
    mut ui_action: EventReader<GuiAction>,
    mut app_state: ResMut<State<EditorState>>,
    mut worldconf: ResMut<config::TerrainConfig>,

    mut egui_ctx: ResMut<EguiContext>,
    mut egui_image_registry: ResMut<UiImages>,
    mut textures: ResMut<Assets<Image>>,
    texture_clipmap: Res<TextureControlClipmap>,
    tint_clipmap: Res<TintClipmap>,
    mut texture_arrays: ResMut<Assets<TextureArray>>,
) {
    fn close_clipmap_window(
        ui_state: &mut UiState,
        egui_ctx: &mut EguiContext,
        egui_image_registry: &mut UiImages,
        texture_clipmap: &TextureControlClipmap,
        tint_clipmap: &TintClipmap,
        texture_arrays: &mut Assets<TextureArray>,
    ) {
        ui_state.debug_show_clipmaps = false;
        for (label, handle) in [
            (texture_clipmap.label(), texture_clipmap.array()),
            (tint_clipmap.label(), tint_clipmap.array()),
        ] {
            if let Some(array) = texture_arrays.get(handle) {
                for i in 0..array.texture_count() {
                    egui_image_registry.remove(egui_ctx, &format!("clipmap.{}.{}", label, i));
                }
            }
        }
    }

    for action in ui_action.iter() {
        match action {
            GuiAction::DebugLoadTerrain(new_config) => {
                *worldconf = (**new_config).clone();
                app_state.overwrite_set(EditorState::TerrainLoading).ok();
            }
            GuiAction::DebugShowClipmap(show) if *show => {
                ui_state.debug_show_clipmaps = true;
                for (label, handle) in [
                    (texture_clipmap.label(), texture_clipmap.array()),
                    (tint_clipmap.label(), tint_clipmap.array()),
                ] {
                    if let Some(array) = texture_arrays.get(handle) {
                        for i in 0..array.texture_count() {
                            let (format, size, img_data) =
                                array.imagedata(i as u8, config::CLIPMAP_SIZE);

                            egui_image_registry.add_image(
                                &mut egui_ctx,
                                &mut *textures,
                                format!("clipmap.{}.{}", label, i),
                                format,
                                (size, size),
                                img_data,
                            );
                        }
                    }
                }
            }
            GuiAction::DebugShowClipmap(_) => {
                close_clipmap_window(
                    &mut ui_state,
                    &mut egui_ctx,
                    &mut egui_image_registry,
                    &texture_clipmap,
                    &tint_clipmap,
                    &mut texture_arrays,
                );
            }
            GuiAction::DebugCloseProject => {
                close_clipmap_window(
                    &mut ui_state,
                    &mut egui_ctx,
                    &mut egui_image_registry,
                    &texture_clipmap,
                    &tint_clipmap,
                    &mut texture_arrays,
                );
                app_state.overwrite_set(EditorState::NoTerrainData).ok();
            }
            _ => {}
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
                self.project_open = false;
                self.project_is_loading = false;
            }
            EditorState::TerrainLoading => {
                self.project_open = false;
                self.project_is_loading = true;
            }
            EditorState::Editing => {
                self.project_open = true;
                self.project_is_loading = false;
            }
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// debug
// ----------------------------------------------------------------------------
#[allow(dead_code)]
fn log_ui_actions(mut ui_action: EventReader<GuiAction>) {
    for ev in ui_action.iter() {
        warn!("UI Action {:?}", ev);
    }
}
// ----------------------------------------------------------------------------
