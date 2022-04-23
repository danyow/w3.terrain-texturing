// ----------------------------------------------------------------------------
#[inline]
pub(super) fn show(
    ui: &mut egui::Ui,
    ui_images: &UiImages,
    toolbox: &ToolboxState,
    materialset: &Res<TerrainMaterialSet>,
    gui_event: &mut EventWriter<GuiAction>,
) {
    let min_height = 3.0 * TEXTURE_PREVIEW_SIZE_SMALL as f32;
    ui.set_min_height(min_height);

    ui.label("Material palette:");
    ui.separator();

    egui::ScrollArea::vertical()
        .max_height(min_height.max(ui.available_height() - ui.spacing().item_spacing.y))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let min_width = TEXTURE_PREVIEW_SIZE_SMALL as f32;
            let max_width = min_width + 20.0;
            let columns = (ui.available_size().x / max_width).floor() as usize;

            let brush = &toolbox.texture_brush;
            let overlay_texture = brush.overlay_texture;
            let bkgrnd_texture = brush.bkgrnd_texture;

            egui::Grid::new("toolbox.palette")
                .striped(true)
                .min_col_width(min_width)
                .max_col_width(max_width)
                .show(ui, |ui| {
                    for i in 0..materialset.parameter.len() {
                        let slot = MaterialSlot::from(i as u8);

                        if let Some(event) = show_material_selection(
                            ui,
                            ui_images,
                            slot,
                            overlay_texture == slot,
                            bkgrnd_texture == slot,
                        ) {
                            gui_event.send(GuiAction::Toolbox(event));
                        }
                        ui.end_row_if((i + 1) % columns == 0);
                    }
                });
        });
    // ui.separator();
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui;

use crate::gui::toolbox::texturebrush::BrushTexturesUsed;
use crate::terrain_material::{MaterialSlot, TerrainMaterialSet, TextureType};

use crate::gui::{GuiAction, UiExtension, UiImages, TEXTURE_PREVIEW_SIZE_SMALL};

use super::{ToolboxAction, ToolboxState};
// ----------------------------------------------------------------------------
#[rustfmt::skip]
fn show_material_selection(
    ui: &mut egui::Ui,
    ui_images: &UiImages,
    slot: MaterialSlot,
    matches_selected_overlay: bool,
    matches_selected_bkgrnd: bool,
) -> Option<ToolboxAction> {
    use ToolboxAction::*;

    let mut result = None;

    let is_selected = matches_selected_overlay || matches_selected_bkgrnd;

    if is_selected {
        let selection_color = match (matches_selected_overlay, matches_selected_bkgrnd) {
            (false, true) => BrushTexturesUsed::Background.selection_color(),
            (true, false) => BrushTexturesUsed::Overlay.selection_color(),
            (true, true) => BrushTexturesUsed::OverlayAndBackground.selection_color(),
            _ => unreachable!(),
        };
        ui.visuals_mut().selection.stroke = egui::Stroke::new(2.0, selection_color);
    }

    ui.vertical_centered(|ui| {
        let material_button = egui::widgets::ImageButton::new(
            ui_images.get_imageid(&format!("terraintexture.{}.{}", TextureType::Diffuse, slot)),
            [
                TEXTURE_PREVIEW_SIZE_SMALL as f32,
                TEXTURE_PREVIEW_SIZE_SMALL as f32,
            ],
        )
        .selected(is_selected);

        let response = ui.add(material_button);
        if response.clicked_by(egui::PointerButton::Primary) {
            result = Some(SelectOverlayTexture(slot));
        }
        if response.clicked_by(egui::PointerButton::Secondary) {
            result = Some(SelectBackgroundTexture(slot));
        }

        let label = egui::RichText::new(format!("material #{}", *slot + 1)).small();
        let label = if is_selected {
            label.strong().color(ui.visuals().selection.stroke.color)
        } else {
            label
        };
        ui.add(egui::Label::new(label));
    });
    result
}
// ----------------------------------------------------------------------------
