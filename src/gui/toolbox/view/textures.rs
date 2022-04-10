// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
pub(super) fn show(
    ui: &mut egui::Ui,
    ui_images: &UiImages,
    brush: &mut BrushSettings,
    gui_event: &mut EventWriter<GuiAction>,
) {
    use GuiAction::*;

    let mut result = None;

    egui::Grid::new("texture.brush.settings")
        .num_columns(2)
        .show(ui, |ui| {

            let (overlay_used, background_used) = match brush.textures_used {
                BrushTexturesUsed::Overlay => (true, false),
                BrushTexturesUsed::Background => (false, true),
                BrushTexturesUsed::OverlayAndBackground => (true, true),
            };
            // --- Material icons
            ui.vertical_centered(|ui| {
                let prev_texture_selection = brush.textures_used;

                if ui.add(texture_icon(ui_images, brush.overlay_texture, overlay_used))
                    .on_hover_text("Use only overlay material")
                    .clicked()
                {
                    brush.textures_used = BrushTexturesUsed::Overlay;
                }
                if ui.add(texture_icon(ui_images, brush.bkgrnd_texture, background_used))
                    .on_hover_text("Use only background material")
                    .clicked()
                {
                    brush.textures_used = BrushTexturesUsed::Background;
                }
                // if combined_textures(ui, ui_images, brush, overlay_used && background_used).interact(Sense::click())
                if combined_textures_icon(ui, ui_images, brush, overlay_used && background_used)
                    .on_hover_text("Use overlay and background materials")
                    .clicked()
                {
                    brush.textures_used = BrushTexturesUsed::OverlayAndBackground;
                }
                // update changes
                if prev_texture_selection != brush.textures_used {
                    result = Some(ToolboxAction::UpdateBrushSettings);
                }
            });

            // --- Brush settings
            ui.vertical(|ui| {
                ui.small("Selected Textures");
                ui.add_enabled_ui(overlay_used, |ui| {
                    ui.small(format!("Overlay: material #{}", *brush.overlay_texture + 1));
                });
                ui.add_enabled_ui(background_used, |ui| {
                    ui.small(format!("Background: material #{}", *brush.bkgrnd_texture + 1));
                });
                // ----------------------------------------------------------------------------
                ui.separator();
                randomize_settings(ui, brush, overlay_used, background_used);
                // ----------------------------------------------------------------------------
                ui.separator();
            });
            ui.end_row();

            // --- Brush size
            ui.label("Brush size:");

            let size = brush.size.to_u8();
            if ui.add(Slider::new(brush.size.as_mut(), BRUSH_SIZE_MIN..=BRUSH_SIZE_MAX)
                .show_value(false)
                .text(format!("{} [m]", size)))
                .changed()
            {
                result = Some(ToolboxAction::UpdateBrushSettings);
            }

            ui.end_row();
        });

    ui.separator();

    if let Some(action) = result {
        gui_event.send(Toolbox(action));
    }
}
// ----------------------------------------------------------------------------
#[inline]
fn randomize_settings(
    ui: &mut Ui,
    brush: &mut BrushSettings,
    overlay_used: bool,
    background_used: bool,
) {
    // copy values (borrow checker)
    let (overlay_chance, bkgrnd_chance) = brush.texture_probabilities;

    ui.checkbox(&mut brush.randomize, "Randomize");
    ui.add_enabled(
        brush.randomize && overlay_used,
        Slider::new(&mut brush.texture_probabilities.0, 1..=100)
            .show_value(false)
            .text(format!("{: <.3}% overlay", overlay_chance)),
    )
    .on_hover_text("chance to assign texture at texel");

    ui.add_enabled(
        brush.randomize && background_used,
        Slider::new(&mut brush.texture_probabilities.1, 1..=100)
            .show_value(false)
            .text(format!("{: >.3}% background", bkgrnd_chance)),
    )
    .on_hover_text("chance to assign texture at texel");
}
// ----------------------------------------------------------------------------
#[inline]
fn texture_icon(ui_images: &UiImages, texture_slot: MaterialSlot, used: bool) -> ImageButton {
    let darken = if used {
        Color32::WHITE
    } else {
        Color32::DARK_GRAY
    };

    ImageButton::new(
        ui_images.get_imageid(&format!(
            "terraintexture.{}.{}",
            TextureType::Diffuse,
            texture_slot,
        )),
        [
            TEXTURE_PREVIEW_SIZE_SMALL as f32,
            TEXTURE_PREVIEW_SIZE_SMALL as f32,
        ],
    )
    .selected(used)
    .tint(darken)
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
fn combined_textures_icon(
    ui: &mut Ui,
    ui_images: &UiImages,
    brush: &mut BrushSettings,
    both_textures_used: bool,
) -> Response {
    let img_size = (TEXTURE_PREVIEW_SIZE_SMALL / 4) as f32;

    let darken = if both_textures_used {
        Color32::WHITE
    } else {
        Color32::DARK_GRAY
    };

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.add(egui::Image::new(
                ui_images.get_imageid(&format!(
                    "terraintexture.{}.{}",
                    TextureType::Diffuse,
                    brush.overlay_texture,
                )),
                [img_size, img_size],
            )
            .tint(darken));

            ui.label("+");

            ui.add(egui::Image::new(
                ui_images.get_imageid(&format!(
                    "terraintexture.{}.{}",
                    TextureType::Diffuse,
                    brush.bkgrnd_texture
                )),
                [img_size, img_size],
            )
            .tint(darken));

        })
    }).response.interact(Sense::click())
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, ImageButton, Response, Sense, Slider, Ui};

use crate::gui::TEXTURE_PREVIEW_SIZE_SMALL;

use crate::gui::toolbox::common::{BRUSH_SIZE_MAX, BRUSH_SIZE_MIN};
use crate::gui::toolbox::ToolboxAction;
use crate::gui::{GuiAction, UiImages};
use crate::terrain_material::{MaterialSlot, TextureType};

use crate::gui::toolbox::texturebrush::{BrushSettings, BrushTexturesUsed};
// ----------------------------------------------------------------------------
