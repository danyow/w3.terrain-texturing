// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
pub(super) fn show(
    ui: &mut egui::Ui,
    brush_size: &mut BrushSize,
    brush: &mut BrushSettings,
    gui_event: &mut EventWriter<GuiAction>,
) {
    const CAPTION_COLUMN_WIDTH: f32 = 60.0;

    use GuiAction::Toolbox;

    // --- Brush settings
    // 2 column grid like in texture brush so it's look is somewhat similar
    egui::Grid::new("scaling.brush.settings")
        .min_col_width(CAPTION_COLUMN_WIDTH)
        .num_columns(2)
        .show(ui, |ui| {
            // --- scaling value
            // wrapped in vertical to align label in column to top
            ui.vertical(|ui|{
                ui.label("Scaling:");
            });

            ui.vertical(|ui| {
                if let Some(action) = scale_settings(ui, brush) {
                    gui_event.send(Toolbox(action));
                }
                randomize_settings(ui, brush);
            });
        });

    ui.separator();
    // ------------------------------------------------------------------------
    // --- Brush size
    egui::Grid::new("scaling.brush.settings.size")
        .min_col_width(CAPTION_COLUMN_WIDTH)
        .num_columns(2)
        .show(ui, |ui| {
            if let Some(action) = common::show_brushsize_control(ui, brush_size) {
                gui_event.send(Toolbox(action));
            }
        });

    ui.separator();
}
// ----------------------------------------------------------------------------
#[inline]
fn scale_settings(ui: &mut Ui, brush: &mut BrushSettings) -> Option<ToolboxAction> {
    use ToolboxAction::ShowBackgroundScaling;

    let mut result = None;
    // relative adjustment of values or directly overwriting
    ui.horizontal(|ui| {
        ui.radio_value(&mut brush.adjust_values, true, "adjust");
        ui.radio_value(&mut brush.adjust_values, false, "overwrite");

        if ui
            .add(ui.small_selectable_button(brush.show_bkgrnd_scaling, "S"))
            .on_hover_text("Show background scaling.")
            .clicked()
        {
            result = Some(ShowBackgroundScaling(!brush.show_bkgrnd_scaling));
        }
    });

    // copy values (borrow checker)
    let bkgrnd_scale = brush.scaling.0;

    ui.add_enabled(
        !brush.adjust_values,
        Slider::new(&mut brush.scaling.0, 0..=7)
            .show_value(false)
            .text(format!("{} scale", bkgrnd_scale)),
    );
    result
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
    .on_hover_text("chance to assign scaling at texel");

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

use crate::gui::toolbox::scalingbrush::BrushSettings;
use crate::gui::{GuiAction, UiExtension};

use super::common;
use super::{BrushSize, ToolboxAction};
// ----------------------------------------------------------------------------
