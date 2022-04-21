// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
pub(super) fn show(
    ui: &mut egui::Ui,
    brush: &mut BrushSettings,
    gui_event: &mut EventWriter<GuiAction>,
) {
    const CAPTION_COLUMN_WIDTH: f32 = 60.0;

    use GuiAction::Toolbox;

    // --- Brush settings
    // 2 column grid like in texture brush so it's look is somewhat similar
    egui::Grid::new("blending.brush.settings")
        .min_col_width(CAPTION_COLUMN_WIDTH)
        .num_columns(2)
        .show(ui, |ui| {
            // --- slope blend threshold value
            // wrapped in vertical to align label in column to top
            ui.vertical(|ui|{
                ui.label("Blending:");
            });

            ui.vertical(|ui| {
                scale_settings(ui, brush);
                randomize_settings(ui, brush);
            });
        });

    ui.separator();
    // ------------------------------------------------------------------------
    // --- Brush size
    egui::Grid::new("blending.brush.settings.size")
        .min_col_width(CAPTION_COLUMN_WIDTH)
        .num_columns(2)
        .show(ui, |ui| {
            if let Some(action) = common::show_brushsize_control(ui, &mut brush.size) {
                gui_event.send(Toolbox(action));
            }
        });

    ui.separator();
}
// ----------------------------------------------------------------------------
#[inline]
fn scale_settings(ui: &mut Ui, brush: &mut BrushSettings) {
    // relative adjustment of values or directly overwriting
    ui.horizontal(|ui| {
        ui.radio_value(&mut brush.adjust_values, true, "adjust");
        ui.radio_value(&mut brush.adjust_values, false, "overwrite");
    });

    // copy values (borrow checker)
    let slope_blend = brush.slope_blend.0;

    ui.add_enabled(
        !brush.adjust_values,
        Slider::new(&mut brush.slope_blend.0, 0..=7)
            .show_value(false)
            .text(format!("{} blend", slope_blend)),
    );
}
// ----------------------------------------------------------------------------
#[inline]
fn randomize_settings(ui: &mut Ui, brush: &mut BrushSettings) {
    // copy values (borrow checker)
    let variance = brush.variance;
    let probability = brush.draw_probability;

    ui.label("Randomize:");
    ui.horizontal(|ui| {
        ui.checkbox(&mut brush.randomize, "");
        ui.add_enabled(
            brush.randomize,
            Slider::new(&mut brush.draw_probability, 1..=100)
                .show_value(false)
                .text(format!("{: <.3}% draw", probability)),
        )
    })
    .response
    .on_hover_text("chance to assign slope blend threshold at texel");

    ui.horizontal(|ui| {
        ui.checkbox(&mut brush.use_variance, "");
        ui.add_enabled(
            brush.use_variance,
            Slider::new(&mut brush.variance.0, 1..=7)
                .show_value(false)
                .text(format!("{: >3} variance", variance.0)),
        );
    })
    .response
    .on_hover_text(
        "maximum range for additional random value added/subtracted to/from base value.",
    );
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui::{self, Slider, Ui};

use crate::gui::toolbox::blendingbrush::BrushSettings;
use crate::gui::GuiAction;

use super::common;
// ----------------------------------------------------------------------------
