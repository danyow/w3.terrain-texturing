// ----------------------------------------------------------------------------
#[rustfmt::skip]
pub fn show_menu(
    ui: &mut egui::Ui,
    ui_state: &UiState,
    gui_event: &mut EventWriter<GuiAction>,
) {
    use crate::config::TerrainConfig;
    use GuiAction::{DebugCloseProject, DebugLoadTerrain};

    ui.set_enabled(!ui_state.project_is_loading);

    ui.menu_button("Debug", |ui| {
        let mut result = None;
        if ui.add_enabled(ui_state.project_open && !ui_state.debug.show_clipmaps, Button::new("show clipmaps")).clicked() {
            result = Some(GuiAction::DebugShowClipmap(true));
        }
        ui.separator();
        if ui.add_enabled(ui_state.project_open, Button::new("unload terrain")).clicked() { result = Some(DebugCloseProject); }
        ui.separator();

        ui.set_enabled(!ui_state.project_open);
        ui.label("Prolog");
        if ui.button("Load Prolog (1024)").clicked() { result = Some(DebugLoadTerrain(Box::new(TerrainConfig::prolog_village(1024)))); }
        if ui.button("Load Prolog (2048)").clicked() { result = Some(DebugLoadTerrain(Box::new(TerrainConfig::prolog_village(2048)))); }
        if ui.button("Load Prolog (4096)").clicked() { result = Some(DebugLoadTerrain(Box::new(TerrainConfig::prolog_village(4096)))); }
        ui.separator();
        ui.label("Bevy");
        if ui.button("Bevy Terrain (4096)").clicked() { result = Some(DebugLoadTerrain(Box::new(TerrainConfig::bevy_example()))); }
        ui.separator();
        ui.label("Kaer Morhen");
        if ui.button("Kaer Morhen (16384)").clicked() { result = Some(DebugLoadTerrain(Box::new(TerrainConfig::kaer_morhen()))); }

        if let Some(event) = result {
            ui.close_menu();
            gui_event.send(event);
        }
    });
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui::{self, Button};

use super::{GuiAction, UiState};
// ----------------------------------------------------------------------------
