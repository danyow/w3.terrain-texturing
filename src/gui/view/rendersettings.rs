// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
pub(super) fn show_settings(
    ui: &mut egui::Ui,
    settings: &mut TerrainRenderSettings,
    gui_event: &mut EventWriter<GuiAction>,
) {
    use GuiAction::UpdateRenderSettings;
    use RenderSetting::OverlayWireframe;

    egui::CollapsingHeader::new("Render settings")
        .default_open(false)
        .show(ui, |ui| {
            // this should be an aligned 2+ column layout which adds equally sized columns
            // according to available width. it also needs to allow complete width separators
            // between groups of sections. (so basically a table with mergable columns)
            // Note: a checkbox with min_width would allow to use ui.horizontal_wrapped
            // instead of Grids

            // background texture checkbox is used as max width
            let column_min_width = ui.checkbox_width("background texture");
            let columns = (ui.available_size().x / column_min_width).floor() as usize;

            // activating an exclusive view will ignore the other selected settings so disable ui
            ui.add_enabled_ui(!settings.exclusive_view_active(), |ui| {
                ui.checkbox(&mut settings.use_flat_shading, "use flat shading")
                    .on_hover_text("use non-interpolated vertex normals (no texture normal)");
                ui.separator();

                // --- overlay (can be combined with other settings)
                ui.small("Overlay:");
                egui::Grid::new("render.settings.overlay")
                    .min_col_width(column_min_width)
                    .show(ui, |ui| {
                        if ui.checkbox(&mut settings.overlay_wireframe, "wireframe").changed() {
                            gui_event.send(UpdateRenderSettings(OverlayWireframe(settings.overlay_wireframe)));
                        }
                        ui.checkbox(&mut settings.overlay_clipmap_level, "clipmap level");
                    });
                ui.separator();

                // --- selectively hide one of those
                ui.small("Don't apply:");
                egui::Grid::new("render.settings.ignored")
                    .min_col_width(column_min_width)
                    .show(ui, |ui| {
                        ui.checkbox(&mut settings.ignore_overlay_texture, "overlay texture");
                        ui.checkbox(&mut settings.ignore_bkgrnd_texture, "background texture");
                        ui.end_row_if(2 % columns == 0);
                        ui.checkbox(&mut settings.ignore_tint_map, "tint map");
                        ui.checkbox(&mut settings.disable_tonemapping, "tonemapping");
                    });
                ui.separator();
            });

            // --- exclusive views: only one of those can be applied
            ui.small("Render only:");
            egui::Grid::new("render.settings.exclusive")
                .min_col_width(column_min_width)
                .show(ui, |ui| {
                    use ExclusiveViewSelection::*;

                    if ui.checkbox(&mut settings.show_fragment_normals, "fragment normals").clicked() {
                        select_exclusive_view(settings, Normals, settings.show_fragment_normals);
                    }
                    if ui.checkbox(&mut settings.show_combined_normals, "merged normals").clicked() {
                        select_exclusive_view(settings, CombinedNormals, settings.show_combined_normals);
                    }
                    ui.end_row_if(2 % columns == 0);
                    if ui.checkbox(&mut settings.show_blend_threshold, "blend threshold").clicked() {
                        select_exclusive_view(settings, BlendThreshold, settings.show_blend_threshold);
                    }
                    ui.end_row_if(3 % columns == 0);
                    if ui.checkbox(&mut settings.show_bkgrnd_scaling, "background scaling").clicked() {
                        select_exclusive_view(settings, UvScaling, settings.show_bkgrnd_scaling);
                    }
                    ui.end_row_if(4 % columns == 0);
                    if ui.checkbox(&mut settings.show_tint_map, "tint map").clicked() {
                        select_exclusive_view(settings, TintMap, settings.show_tint_map);
                    }
                });
            ui.separator();
        });
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui;

use crate::{gui::RenderSetting, terrain_render::TerrainRenderSettings};

use super::{GuiAction, UiExtension};
// ----------------------------------------------------------------------------
enum ExclusiveViewSelection {
    Normals,
    CombinedNormals,
    BlendThreshold,
    UvScaling,
    TintMap,
}
// ----------------------------------------------------------------------------
fn select_exclusive_view(
    settings: &mut TerrainRenderSettings,
    selection: ExclusiveViewSelection,
    value: bool,
) {
    use ExclusiveViewSelection::*;

    // reset all and set selected to current value
    settings.reset_exclusive_view();

    match selection {
        Normals => settings.show_fragment_normals = value,
        CombinedNormals => settings.show_combined_normals = value,
        BlendThreshold => settings.show_blend_threshold = value,
        UvScaling => settings.show_bkgrnd_scaling = value,
        TintMap => settings.show_tint_map = value,
    }
}
// ----------------------------------------------------------------------------
