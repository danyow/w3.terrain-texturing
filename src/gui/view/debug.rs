// ----------------------------------------------------------------------------
#[rustfmt::skip]
pub(super) fn show_debug_menu(
    ui: &mut egui::Ui,
    ui_state: &UiState,
    gui_event: &mut EventWriter<GuiAction>,
) {
    use crate::config::TerrainConfig;
    use GuiAction::{DebugCloseProject, DebugLoadTerrain};

    ui.set_enabled(!ui_state.project_is_loading);

    ui.menu_button("Debug", |ui| {
        if ui.add_enabled(ui_state.project_open, Button::new("unload terrain")).clicked() { gui_event.send(DebugCloseProject) }
        ui.separator();

        ui.set_enabled(!ui_state.project_open);
        ui.label("Prolog");
        if ui.button("Load Prolog (256)").clicked() { gui_event.send(DebugLoadTerrain(Box::new(TerrainConfig::prolog_village(256)))) }
        if ui.button("Load Prolog (512)").clicked() { gui_event.send(DebugLoadTerrain(Box::new(TerrainConfig::prolog_village(512)))) }
        if ui.button("Load Prolog (1024)").clicked() { gui_event.send(DebugLoadTerrain(Box::new(TerrainConfig::prolog_village(1024)))) }
        if ui.button("Load Prolog (2048)").clicked() { gui_event.send(DebugLoadTerrain(Box::new(TerrainConfig::prolog_village(2048)))) }
        if ui.button("Load Prolog (4096)").clicked() { gui_event.send(DebugLoadTerrain(Box::new(TerrainConfig::prolog_village(4096)))) }
        ui.separator();
        ui.label("Kaer Morhen");
        if ui.button("Kaer Morhen (16384)").clicked() { gui_event.send(DebugLoadTerrain(Box::new(TerrainConfig::kaer_morhen()))) }
    });
}
// ----------------------------------------------------------------------------
use bevy::prelude::EventWriter;
use bevy_egui::egui::{self, Button};

use crate::gui::{GuiAction, UiState};
// ----------------------------------------------------------------------------
