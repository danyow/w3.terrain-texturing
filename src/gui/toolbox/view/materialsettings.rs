// ----------------------------------------------------------------------------
#[inline]
pub(super) fn show(
    ui: &mut egui::Ui,
    ui_images: &UiImages,
    brush: &BrushSettings,
    materialset: &Res<TerrainMaterialSet>,
    gui_event: &mut EventWriter<GuiAction>,
) {
    use BrushTexturesUsed::*;
    use GuiAction::Toolbox;
    use ToolboxAction::UpdateMaterial;

    let mut material_slot = brush.overlay_texture;
    let mut param = materialset.parameter[material_slot];

    egui::Grid::new("texture.material.overlay.settings")
        .num_columns(2)
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                add_texture_icon(ui, ui_images, material_slot, Overlay);
            });

            // settings only relevant if used as overlay material
            ui.vertical(|ui| {
                if let Some(changed) = show_overlay_material_settings(ui, &mut param) {
                    gui_event.send(Toolbox(UpdateMaterial(material_slot, changed)));
                }
            });
        });

    if let Some(changed) =
        show_general_material_settings(ui, "Overlay Material General Settings", &mut param)
    {
        gui_event.send(Toolbox(UpdateMaterial(material_slot, changed)));
    }
    ui.separator();

    // ------------------------------------------------------------------------
    material_slot = brush.bkgrnd_texture;
    param = materialset.parameter[material_slot];

    egui::Grid::new("texture.material.bkgrnd.settings")
        .num_columns(2)
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                add_texture_icon(ui, ui_images, material_slot, Background);
            });

            // settings only relevant as background material
            ui.vertical(|ui| {
                if let Some(changed) = show_bkgrnd_material_settings(ui, &mut param) {
                    gui_event.send(Toolbox(UpdateMaterial(material_slot, changed)));
                }
            });
        });

    if let Some(changed) =
        show_general_material_settings(ui, "Background Material General Settings", &mut param)
    {
        gui_event.send(Toolbox(UpdateMaterial(material_slot, changed)));
    }
    ui.separator();
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui::{self, ImageButton, Slider};

use crate::gui::toolbox::texturebrush::{BrushSettings, BrushTexturesUsed};
use crate::gui::toolbox::{MaterialSetting, ToolboxAction};
use crate::gui::{GuiAction, UiExtension, UiImages, TEXTURE_PREVIEW_SIZE_SMALL};

use crate::terrain_material::{
    MaterialSlot, TerrainMaterialParam, TerrainMaterialSet, TextureType,
};
// ----------------------------------------------------------------------------
#[inline]
fn add_texture_icon(
    ui: &mut egui::Ui,
    ui_images: &UiImages,
    texture_slot: MaterialSlot,
    texture_type: BrushTexturesUsed,
) {
    ui.visuals_mut().selection.stroke = egui::Stroke::new(2.0, texture_type.selection_color());
    ui.add(
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
        .selected(true),
    );
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
fn show_overlay_material_settings(
    ui: &mut egui::Ui,
    p: &mut TerrainMaterialParam,
) -> Option<MaterialSetting> {
    let mut result = None;
    ui.small("Material Overlay Settings");

    // bottom up aligned -> flip order :(
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        if ui.add(Slider::new(&mut p.blend_sharpness, 0.0..=1.0)
            .text("sharpness").fixed_decimals(3)
        ).changed() {
            result = Some(MaterialSetting::SetBlendSharpness(p.blend_sharpness));
        }
        ui.small("Blend sharpness:");
    });
    result
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
fn show_bkgrnd_material_settings(
    ui: &mut egui::Ui,
    p: &mut TerrainMaterialParam,
) -> Option<MaterialSetting> {
    use MaterialSetting::*;

    let mut result = None;
    ui.small("Material Background Settings");

    // bottom up aligned -> flip order :(
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {

        if ui.add(Slider::new(&mut p.slope_normal_dampening, 0.0..=1.0).text("normal")).changed() {
            result = Some(SetSlopeNormalDampening(p.slope_normal_dampening));
        }
        if ui.add(Slider::new(&mut p.slope_base_dampening, 0.0..=1.0).text("base")).changed() {
            result = Some(SetSlopeBaseDampening(p.slope_base_dampening));
        }
        ui.small("Slope dampening:");
    });
    result
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
fn show_general_material_settings(
    ui: &mut egui::Ui,
    caption: &str,
    p: &mut TerrainMaterialParam,
) -> Option<MaterialSetting> {
    use MaterialSetting::*;

    egui::CollapsingHeader::new(ui.small_text(caption))
        .default_open(false)
        .show(ui, |ui| {
            let mut result = None;
            if ui.add(Slider::new(&mut p.specularity_scale, 0.0..=1.0).text("specularity scale")).changed() {
                result = Some(SetSpecularityScale(p.specularity_scale));
            }
            if ui.add(Slider::new(&mut p.specularity, 0.0..=1.0).text("specularity")).changed() {
                result = Some(SetSpecularity(p.specularity));
            }
            if ui.add(Slider::new(&mut p.specularity_base, 0.0..=1.0).text("specularity base")).changed() {
                result = Some(SetSpecularityBase(p.specularity_base));
            }
            if ui.add(Slider::new(&mut p.falloff, 0.0..=1.0).text("falloff")).changed() {
                result = Some(SetFalloff(p.falloff));
            }
            result
        })
        .body_returned.flatten()
}
// ----------------------------------------------------------------------------
