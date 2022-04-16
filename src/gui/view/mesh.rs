// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
pub(super) fn show_settings(
    ui: &mut egui::Ui,
    settings: &Res<TerrainMeshSettings>,
    stats: &Res<TerrainStats>,
    gui_event: &mut EventWriter<GuiAction>,
) {
    use GuiAction::*;
    use MeshSetting::*;

    egui::CollapsingHeader::new("Terrain Mesh Lod settings")
        .default_open(false)
        .show(ui, |ui| {
            let mut result = None;

            let mut s = MeshSettings {
                freeze: settings.ignore_anchor,
                lod_count: settings.lod_count,
                min_error: settings.min_error,
                max_error: settings.max_error,
                max_distance: settings.max_distance,
            };

            // make sure the event is only triggered on release
            let changed = |response: Response, ui: &egui::Ui| -> bool {
                (response.changed() && response.drag_released())
                    || (response.changed() && ui.input().key_pressed(egui::Key::Enter))
            };

            if changed(ui.add(Slider::new(&mut s.lod_count, 1..=10).text("Lod count")), ui) {
                result = Some(SetLodCount(s.lod_count));
            }
            ui.separator();
            if changed(
                ui.add(Slider::new(&mut s.min_error, 0.001..=1.0).text("min error [m]"))
                    .on_hover_text("Min error threshold for Lods"),
                ui)
            {
                result = Some(SetLodMinError(s.min_error));
            }
            if changed(
                ui.add(Slider::new(&mut s.max_error, s.min_error..=10.0).text("max error [m]"))
                    .on_hover_text("Max error threshold for last Lod"),
                ui)
            {
                result = Some(SetLodMaxError(s.max_error));
            }
            if changed(
                ui.add(Slider::new(&mut s.max_distance, 100.0..=10000.0).text("last lod start [m]"))
                    .on_hover_text(
                        format!("Start distance for Lod {}", s.lod_count)
                    ),
                ui)
            {
                result = Some(SetLodMaxDistance(s.max_distance));
            }

            ui.separator();
            egui::CollapsingHeader::new("Lod - per level settings")
                .default_open(false)
                .show(ui, |ui| {
                    ui.label("Error thresholds [m]");
                    for (i, lod) in settings.lod_settings().enumerate() {
                        let mut threshold = lod.threshold;
                        if changed(ui.add(Slider::new(&mut threshold, s.min_error..=s.max_error).text(format!("lod {}", i))), ui) {
                            result = Some(SetLodError(LodSlot::from(i as u8), threshold));
                        }
                    }
                    ui.separator();
                    ui.label("Start distances [m]");
                    for (i, lod) in settings.lod_settings().enumerate() {
                        let mut distance = lod.distance;
                        if changed(ui.add(Slider::new(&mut distance, 0.0..=s.max_distance).text(format!("lod {}", i))), ui) {
                            result = Some(SetLodDistance(LodSlot::from(i as u8), distance));
                        }
                    }
                });
            ui.separator();
            for (i, lod) in settings.lod_settings().enumerate() {
                ui.label(format!("lod #{:<2}: starts: {:>6.1} m  error: {:>4.3} m", i, lod.distance, lod.threshold));
            }
            ui.separator();
            ui.horizontal(|ui| {

                if ui.checkbox(&mut s.freeze, "freeze lods").changed() {
                    result = Some(FreezeLods);
                }
                ui.add_space(ui.spacing().item_spacing.x * 2.0);

                if ui.button(" reset ").clicked() {
                    result = Some(ResetToDefault);
                }
            });

            if let Some(action) = result {
                gui_event.send(UpdateMeshSetting(action));
            }
        });

    egui::CollapsingHeader::new("Terrain Stats")
        .default_open(true)
        .show(ui, |ui| {
            ui.small(format!("Tiles: #{} ({} x {})", stats.tiles, TILE_SIZE, TILE_SIZE));

            egui::Grid::new("stats.all")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.small(format!("{} vertices", stats.vertices));
                    ui.small(format!("{} triangles", stats.triangles));
                    ui.small(format!("{:.3} MB", stats.data_bytes as f32 / 1024.0 / 1024.0));
                });

            ui.separator();
            ui.small("last update:")
                .on_hover_text("Data is accumulated over multiple frames until all pending tile \
                    updates are finished.\
                    \nNote: If camera is moving too fast and generation cannot catch up this will \
                    grow indefinitely.");

            egui::Grid::new("stats.last.update")
                .num_columns(4)
                .show(ui, |ui| {
                    ui.small(format!("{} tiles", stats.last_update_tiles));
                    ui.small(format!("{} vertices", stats.last_update_vertices));
                    ui.small(format!("{} triangles", stats.last_update_triangles));
                    ui.small(format!("{:.3} MB", stats.last_update_data_bytes as f32 / 1024.0 / 1024.0));
                });
            ui.separator();
        });
}
// ----------------------------------------------------------------------------
use bevy::prelude::{EventWriter, Res};
use bevy_egui::egui::{self, Response, Slider};

use crate::config::TILE_SIZE;
use crate::terrain_tiles::{LodSlot, TerrainMeshSettings, TerrainStats};

use crate::gui::MeshSetting;

use super::GuiAction;
// ----------------------------------------------------------------------------
struct MeshSettings {
    freeze: bool,
    lod_count: u8,
    min_error: f32,
    max_error: f32,
    max_distance: f32,
}
// ----------------------------------------------------------------------------
