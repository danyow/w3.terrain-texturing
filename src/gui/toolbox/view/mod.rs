// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[allow(clippy::too_many_arguments)]
pub fn show_ui(
    ui: &mut egui::Ui,
    toolbox: &mut ToolboxState,
    ui_images: &UiImages,
    materialset: Option<Res<TerrainMaterialSet>>,
    gui_event: &mut EventWriter<GuiAction>,
) {
    ui.separator();
    ui.label("Toolbox");
    // tool selection
    ui.horizontal_wrapped(|ui| {
        ui.deselectable_value(&mut toolbox.selection, Texturing, egui::RichText::new("Texturing").small())
            .on_hover_text("Texture Brush: overwriting of overlay and/or background texture.");
    });
    ui.separator();

    match toolbox.selection {
        None | Some(Texturing) => {
            textures::show(ui, ui_images, &mut toolbox.texture_brush, gui_event);
        }
    }

    if let Some(materialset) = materialset {
        materialpalette::show(ui, ui_images, toolbox, &materialset, gui_event);
    }
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui;

use crate::terrain_material::TerrainMaterialSet;

use crate::gui::{GuiAction, UiExtension, UiImages};

use super::{ToolSelection::*, ToolboxAction, ToolboxState};
// ----------------------------------------------------------------------------
mod materialpalette;
mod textures;
// ----------------------------------------------------------------------------
