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
    let selected_tool = toolbox.selection;
    ui.horizontal_wrapped(|ui| {
        ui.deselectable_value(&mut toolbox.selection, Texturing, egui::RichText::new("Texturing").small())
            .on_hover_text("Texture Brush: overwriting of overlay and/or background texture.");

        ui.deselectable_value(&mut toolbox.selection, Blending, egui::RichText::new("Blending").small())
            .on_hover_text("Blending Brush: adjusting or overwriting of slope blend threshold \
                (blending between overlay and background texture).");

        ui.deselectable_value(&mut toolbox.selection, Scaling, egui::RichText::new("Scaling").small())
            .on_hover_text("Scaling Brush: adjusting or overwriting of background texture scaling.");

        ui.deselectable_value(&mut toolbox.selection, MaterialParameters, ui.small_text("Material Parameters"));
    });
    if selected_tool != toolbox.selection {
        gui_event.send(GuiAction::Toolbox(ToolboxAction::UpdateBrushSettings));
    }
    ui.separator();

    let brush_size = &mut toolbox.brush_size;
    match toolbox.selection {
        Some(Texturing) => {
            textures::show(ui, ui_images, brush_size, &mut toolbox.texture_brush, gui_event);
        }
        Some(Blending) => {
            blending::show(ui, brush_size, &mut toolbox.blending_brush, gui_event);
        }
        Some(Scaling) => {
            scaling::show(ui, brush_size, &mut toolbox.scaling_brush, gui_event);
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
                textures::show(ui, ui_images, brush_size, &mut toolbox.texture_brush, gui_event);
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

use super::common::BrushSize;
use super::{ToolSelection::*, ToolboxAction, ToolboxState};
// ----------------------------------------------------------------------------
mod common;

mod blending;
mod materialpalette;
mod materialsettings;
mod scaling;
mod textures;
// ----------------------------------------------------------------------------
