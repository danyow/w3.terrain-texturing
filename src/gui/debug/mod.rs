// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::EguiContext;

use crate::config;
use crate::terrain_clipmap::{TextureControlClipmap, TintClipmap};
use crate::texturearray::TextureArray;

use crate::{DebugEvent, EditorEvent, EditorState};

use super::{GuiAction, UiImages, UiState};
// ----------------------------------------------------------------------------
pub struct EditorUiDebugPlugin;

pub use self::view::show_menu;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(super) struct DebugUiState {
    show_clipmaps: bool,
}
// ----------------------------------------------------------------------------
mod view;
// ----------------------------------------------------------------------------
impl Plugin for EditorUiDebugPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.add_system(
            view::show_ui
                .label("gui_debug_actions")
                .after("gui_actions"),
        )
        .add_system(handle_editor_events)
        .add_system(handle_ui_debug_actions.after("gui_debug_actions"));
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
fn handle_editor_events(
    mut egui_image_registry: ResMut<UiImages>,
    mut images: ResMut<Assets<Image>>,
    texture_arrays: Res<Assets<TextureArray>>,
    mut events: EventReader<EditorEvent>,
) {
    for event in events.iter() {
        if let EditorEvent::Debug(event) = event {
            match event {
                DebugEvent::ClipmapUpdate(clipmap_label, slot, handle) => {
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
    heightmap_clipmap: Res<crate::terrain_clipmap::HeightmapClipmap>,
    mut texture_arrays: ResMut<Assets<TextureArray>>,
) {
    for action in ui_action.iter() {
        match action {
            GuiAction::DebugLoadTerrain(new_config) => {
                *worldconf = (**new_config).clone();
                app_state.overwrite_set(EditorState::TerrainLoading).ok();
            }
            GuiAction::DebugShowClipmap(show) if *show => {
                ui_state.debug.show_clipmaps = true;
                for (label, handle) in [
                    (texture_clipmap.label(), texture_clipmap.array()),
                    (tint_clipmap.label(), tint_clipmap.array()),
                    (heightmap_clipmap.label(), heightmap_clipmap.array()),
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
                    &heightmap_clipmap,
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
                    &heightmap_clipmap,
                    &mut texture_arrays,
                );
                app_state.overwrite_set(EditorState::NoTerrainData).ok();
            }
            _ => {}
        }
    }
}
// ----------------------------------------------------------------------------
// helper
// ----------------------------------------------------------------------------
// #[allow(clippy::too_many_arguments)]
fn close_clipmap_window(
    ui_state: &mut UiState,
    egui_ctx: &mut EguiContext,
    egui_image_registry: &mut UiImages,
    texture_clipmap: &TextureControlClipmap,
    tint_clipmap: &TintClipmap,
    heightmap_clipmap: &crate::terrain_clipmap::HeightmapClipmap,
    texture_arrays: &mut Assets<TextureArray>,
) {
    ui_state.debug.show_clipmaps = false;
    for (label, handle) in [
        (texture_clipmap.label(), texture_clipmap.array()),
        (tint_clipmap.label(), tint_clipmap.array()),
        (heightmap_clipmap.label(), heightmap_clipmap.array()),
    ] {
        if let Some(array) = texture_arrays.get(handle) {
            for i in 0..array.texture_count() {
                egui_image_registry.remove(egui_ctx, &format!("clipmap.{}.{}", label, i));
            }
        }
    }
}
// ----------------------------------------------------------------------------
