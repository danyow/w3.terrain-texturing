// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[allow(clippy::too_many_arguments)]
pub fn show_ui(
    ui: &mut egui::Ui,
    toolbox: &mut ToolboxState,
    ui_images: &UiImages,
    materialset: Res<TerrainMaterialSet>,
    gui_event: &mut EventWriter<GuiAction>,
) {
    ui.separator();
    ui.label("Toolbox");

    // tool selection
    ui.horizontal_wrapped(|ui| {
        ui.deselectable_value(&mut toolbox.selection, Texturing, egui::RichText::new("Texturing").small())
            .on_hover_text("Texture Brush: overwriting of overlay and/or background texture.");

        ui.deselectable_value(&mut toolbox.selection, Scaling, egui::RichText::new("Scaling").small())
            .on_hover_text("Scaling Brush: adjusting or overwriting of background texture scaling.");

        ui.deselectable_value(&mut toolbox.selection, MaterialParameters, ui.small_text("Material Parameters"));
    });
    ui.separator();

    match toolbox.selection {
        Some(Texturing) => {
            textures::show(ui, ui_images, &mut toolbox.texture_brush, gui_event);
        }
        Some(Scaling) => {
            scaling::show(ui, &mut toolbox.scaling_brush, gui_event);
        }
        Some(MaterialParameters) => {
            materialsettings::show(
                ui,
                ui_images,
                &toolbox.texture_brush,
                &materialset,
                gui_event,
            );
        }
        None => {
            // show default texture brush settings but deactivate it
            ui.add_enabled_ui(false, |ui| {
                textures::show(ui, ui_images, &mut toolbox.texture_brush, gui_event);
            });
        }
    }

    materialpalette::show(ui, ui_images, toolbox, &materialset, gui_event);
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui;

use crate::terrain_material::TerrainMaterialSet;

use crate::gui::{GuiAction, UiExtension, UiImages};

use super::{ToolSelection::*, ToolboxAction, ToolboxState};
// ----------------------------------------------------------------------------
mod common;

mod materialpalette;
mod materialsettings;
mod textures;
mod scaling;
// ----------------------------------------------------------------------------
