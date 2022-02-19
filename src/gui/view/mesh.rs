// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
pub(super) fn show_settings(
    ui: &mut egui::Ui,
    settings: &Res<TerrainMeshSettings>,
    gui_event: &mut EventWriter<GuiAction>,
) {
    use GuiAction::*;
    use MeshSetting::*;

    egui::CollapsingHeader::new("Terrain Mesh Lod settings")
        .default_open(false)
        .show(ui, |ui| {
            let mut result = None;

            let mut s = MeshSettings {
                freeze: !settings.ignore_anchor,
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
            if ui.checkbox(&mut s.freeze, "freeze lods").changed() {
                result = Some(FreezeLods);
            }

            if ui.button("reset").clicked() {
                result = Some(ResetToDefault);
            }
            if let Some(action) = result {
                gui_event.send(UpdateMeshSetting(action));
            }
        });
}
// ----------------------------------------------------------------------------
use bevy::prelude::{EventWriter, Res};
use bevy_egui::egui::{self, Response, Slider};

use crate::terrain_tiles::{LodSlot, TerrainMeshSettings};

use crate::gui::MeshSetting;
use crate::GuiAction;
// ----------------------------------------------------------------------------
struct MeshSettings {
    freeze: bool,
    lod_count: u8,
    min_error: f32,
    max_error: f32,
    max_distance: f32,
}
// ----------------------------------------------------------------------------
