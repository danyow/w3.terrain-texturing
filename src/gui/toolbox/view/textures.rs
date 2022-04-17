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
            use BrushTexturesUsed::*;

            let (overlay_used, background_used) = match brush.textures_used {
                Overlay => (true, false),
                Background => (false, true),
                OverlayAndBackground => (true, true),
            };
            // --- Material icons
            ui.vertical_centered(|ui| {
                let prev_texture_selection = brush.textures_used;

                if add_texture_selection(ui, ui_images, brush.overlay_texture, Overlay, overlay_used)
                    .on_hover_text("Use only overlay material")
                    .clicked()
                {
                    brush.textures_used = Overlay;
                }
                if add_texture_selection(ui, ui_images, brush.bkgrnd_texture, Background, background_used)
                    .on_hover_text("Use only background material")
                    .clicked()
                {
                    brush.textures_used = Background;
                }

                if add_combined_textures_selection(ui, ui_images, brush, overlay_used && background_used)
                    .on_hover_text("Use overlay and background materials")
                    .clicked()
                {
                    brush.textures_used = OverlayAndBackground;
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
                scale_and_blend_settings(ui, brush);
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
fn add_texture_selection(
    ui: &mut Ui,
    ui_images: &UiImages,
    texture_slot: MaterialSlot,
    texture_type: BrushTexturesUsed,
    used: bool,
) -> egui::Response {
    ui.visuals_mut().selection.stroke = egui::Stroke::new(2.0, texture_type.selection_color());
    ui.add(texture_icon(ui_images, texture_slot, used))
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
fn add_combined_textures_selection(
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
#[inline]
fn scale_and_blend_settings(ui: &mut Ui, brush: &mut BrushSettings) {
    ui.small("Overwrite scaling and slope blending");
    // copy values (borrow checker)
    let (bkgrnd_scale, slope_blend) = (brush.scaling.0, brush.slope_blend.0);

    ui.horizontal(|ui| {
        ui.checkbox(&mut brush.overwrite_scale, "");
        ui.add_enabled(
            brush.overwrite_scale,
            Slider::new(&mut brush.scaling.0, 0..=7)
                .show_value(false)
                .text(format!("{} scale", bkgrnd_scale)),
        );
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut brush.overwrite_slope_blend, "");
        ui.add_enabled(
            brush.overwrite_slope_blend,
            Slider::new(&mut brush.slope_blend.0, 0..=7)
                .show_value(false)
                .text(format!("{} blend", slope_blend)),
        );
    });
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, ImageButton, Response, Sense, Slider, Ui};

use crate::gui::TEXTURE_PREVIEW_SIZE_SMALL;

use crate::gui::{GuiAction, UiImages};
use crate::terrain_material::{MaterialSlot, TextureType};

use crate::gui::toolbox::common::{BRUSH_SIZE_MAX, BRUSH_SIZE_MIN};
use crate::gui::toolbox::texturebrush::{BrushSettings, BrushTexturesUsed};

use super::ToolboxAction;
// ----------------------------------------------------------------------------
