// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub(super) fn show_windows(
    egui_ctx: &mut EguiContext,
    ui_images: &Res<UiImages>,
    ui_state: &Res<UiState>,
    clipmap_tracker: &Res<crate::terrain_clipmap::ClipmapTracker>,
    texture_clipmap: &Res<TextureControlClipmap>,
    tint_clipmap: &Res<TintClipmap>,
    gui_event: &mut EventWriter<GuiAction>,
) {
    let mut opened = ui_state.debug_show_clipmaps;
    egui::Window::new("DEBUG: clipmap")
        .open(&mut opened)
        .default_size((400.0, 600.0))
        .vscroll(true)
        .hscroll(true)
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.label(format!(
                "Conf: {}x{} #{} level (data: {}x{})",
                CLIPMAP_SIZE,
                CLIPMAP_SIZE,
                clipmap_tracker.level_count(),
                clipmap_tracker.datasource_size(),
                clipmap_tracker.datasource_size(),
            ));

            for label in [texture_clipmap.label(), tint_clipmap.label()] {
                egui::CollapsingHeader::new(label)
                    .default_open(true)
                    .show(ui, |ui| {
                        for (i, l) in clipmap_tracker.layers() {
                            egui::CollapsingHeader::new(format!("level {}", i))
                                .default_open(false)
                                .show(ui, |ui| {
                                    ui.add(egui::widgets::Image::new(
                                        ui_images.get_imageid(&format!("clipmap.{}.{}", label, i)),
                                        [256.0, 256.0],
                                    ));
                                });
                            ui.label(format!(
                                "rectangle: ({} / {}) - ({} / {})  {}x{}",
                                l.rectangle().pos.x,
                                l.rectangle().pos.y,
                                l.rectangle().pos.x + l.rectangle().size.x,
                                l.rectangle().pos.y + l.rectangle().size.y,
                                l.rectangle().size.x,
                                l.rectangle().size.y
                            ));
                        }
                    });
            }
        });

    if opened != ui_state.debug_show_clipmaps {
        gui_event.send(GuiAction::DebugShowClipmap(opened));
    }
}
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
        if ui.add_enabled(ui_state.project_open && !ui_state.debug_show_clipmaps, Button::new("show clipmaps")).clicked() {
            gui_event.send(GuiAction::DebugShowClipmap(true))
        }
        ui.separator();
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
use bevy::prelude::*;
use bevy_egui::egui::{self, Button};
use bevy_egui::EguiContext;

use crate::config::CLIPMAP_SIZE;
use crate::gui::{GuiAction, UiImages, UiState};
use crate::terrain_clipmap::{TextureControlClipmap, TintClipmap};
// ----------------------------------------------------------------------------
