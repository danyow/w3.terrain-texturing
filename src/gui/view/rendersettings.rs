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
                        ui.end_row_if(3 % columns == 0);
                        ui.checkbox(&mut settings.disable_shadows, "terrain shadows");
                        ui.end_row_if(4 % columns == 0);
                        ui.checkbox(&mut settings.disable_fog, "environment fog");
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
                    if ui.checkbox(&mut settings.show_lightheight_map, "lightheight map").clicked() {
                        select_exclusive_view(settings, LightheightMap, settings.show_lightheight_map);
                    }
                });
            ui.separator();
        });
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
pub(super) fn show_terrain_shadows_settings(
    ui: &mut egui::Ui,
    settings: &mut TerrainRenderSettings,
    shadow_settings: &mut TerrainShadowsRenderSettings,
) {
    egui::CollapsingHeader::new("Terrain Shadows settings")
        .default_open(false)
        .show(ui, |ui| {

            ui.add_enabled_ui(!settings.exclusive_view_active(), |ui| {
                ui.checkbox(&mut settings.disable_shadows, "disable terrain shadows");
                ui.add_enabled_ui(!settings.disable_shadows, |ui| {
                    ui.add(Slider::new(&mut shadow_settings.intensity, 0.5..=1.0).text("shadow intensity"));

                    ui.add(Slider::new(&mut shadow_settings.recompute_frequency, 1..=10).text("refresh [frames]"))
                        .on_hover_text("recomputes shadows only every n'th frame if daynight cycle is active.");
                    ui.separator();

                    ui.small("Shadow Smoothiness:");
                    ui.add_enabled_ui(!settings.fast_shadows, |ui| {
                        ui.add(Slider::new(&mut shadow_settings.falloff_smoothness, 1.0..=75.0).text("max"));
                        ui.add(Slider::new(&mut shadow_settings.falloff_scale, 100.0..=4000.0).text("scale distance"));
                        ui.add(Slider::new(&mut shadow_settings.falloff_bias, 1.0..=100.0).text("scale bias"));

                        ui.add(Slider::new(&mut shadow_settings.interpolation_range, 100.0..=2000.0)
                            .text("interpol. range [m]"))
                            .on_hover_text("max distance from camera for active interpolation of shadows.");
                    });
                });
                ui.separator();
                let column_min_width = ui.checkbox_width("background texture");

                egui::Grid::new("render.settings.ignored")
                    .min_col_width(column_min_width)
                    .show(ui, |ui| {
                        if ui.checkbox(&mut settings.fast_shadows, "fast shadows").clicked() && settings.fast_shadows {
                            shadow_settings.recompute_frequency = 10;
                        }

                        if ui.button("reset").clicked() {
                            *shadow_settings = TerrainShadowsRenderSettings::default();
                            settings.fast_shadows = false;
                        }
                    });
            });
            ui.separator();
        });
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui::{self, Slider};

use crate::gui::RenderSetting;
use crate::terrain_render::{TerrainRenderSettings, TerrainShadowsRenderSettings};

use super::{GuiAction, UiExtension};
// ----------------------------------------------------------------------------
enum ExclusiveViewSelection {
    Normals,
    CombinedNormals,
    BlendThreshold,
    UvScaling,
    TintMap,
    LightheightMap,
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
        LightheightMap => settings.show_lightheight_map = value,
    }
}
// ----------------------------------------------------------------------------
