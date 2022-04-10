// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub fn show_ui(
    ui: &mut egui::Ui,
    toolbox: &mut ToolboxState,
    ui_images: &UiImages,
    gui_event: &mut EventWriter<GuiAction>,
) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.selectable_value(&mut toolbox.selection, Texturing, "Texturing")
            .on_hover_text("Textures: overwriting of overlay and/or background texture.");
    });
    ui.separator();

    match toolbox.selection {
        Texturing => {
            textures::show(ui, ui_images, &mut toolbox.texture_brush, gui_event);
        }
    }
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui;

use crate::gui::{GuiAction, UiImages};

use super::{ToolSelection::*, ToolboxState};
// ----------------------------------------------------------------------------
mod textures;
// ----------------------------------------------------------------------------
