// ----------------------------------------------------------------------------
#[rustfmt::skip]
pub(super) fn show_brushsize_control(
    ui: &mut Ui,
    brushsize: &mut BrushSize,
) -> Option<ToolboxAction> {

    ui.label("Brush size:");

    let size = brushsize.to_u8();
    if ui.add(
        Slider::new(brushsize.as_mut(), BRUSH_SIZE_MIN..=BRUSH_SIZE_MAX)
            .show_value(false)
            .text(format!("{} [m]", size)),
        )
        .changed()
    {
        Some(ToolboxAction::UpdateBrushSettings)
    } else {
        None
    }
}
// ----------------------------------------------------------------------------
use bevy_egui::egui::{Slider, Ui};

use crate::gui::toolbox::common::{BrushSize, BRUSH_SIZE_MAX, BRUSH_SIZE_MIN};

use super::ToolboxAction;
// ----------------------------------------------------------------------------
