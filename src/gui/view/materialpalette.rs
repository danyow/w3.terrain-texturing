// ----------------------------------------------------------------------------
#[inline]
pub(super) fn show(
    ui: &mut egui::Ui,
    ui_img_registry: &Res<UiImages>,
    ui_state: &UiState,
    materialset: &Res<TerrainMaterialSet>,
    gui_event: &mut EventWriter<GuiAction>,
) {
    use GuiAction::*;

    egui::CollapsingHeader::new("Material palette")
        .default_open(true)
        .show(ui, |ui| {
            let min_width = TEXTURE_PREVIEW_SIZE_SMALL as f32;
            let max_width = min_width + 20.0;
            let columns = (ui.available_size().x / max_width).floor() as usize;

            if let Some(slot) = ui_state.selected_slot {
                let param = &materialset.parameter[slot];

                let mut p = TerrainMaterialParam {
                    blend_sharpness: param.blend_sharpness,
                    slope_base_dampening: param.slope_base_dampening,
                    slope_normal_dampening: param.slope_normal_dampening,
                    specularity_scale: param.specularity_scale,
                    specularity: param.specularity,
                    specularity_base: param.specularity_base,
                    _specularity_scale_copy: param._specularity_scale_copy,
                    falloff: param.falloff,
                };

                if let Some(action) = show_material_settings(ui, &mut p) {
                    gui_event.send(UpdateMaterial(slot, action));
                }
                ui.separator();
            }

            egui::Grid::new("palette.textures")
                .striped(true)
                .min_col_width(min_width)
                .max_col_width(max_width)
                .show(ui, |ui| {
                    for i in 0..materialset.parameter.len() {
                        let slot = MaterialSlot::from(i as u8);
                        let is_selected = ui_state
                            .selected_slot
                            .map(|s| s == slot)
                            .unwrap_or_default();

                        if let Some(event) =
                            show_material_selection(ui, ui_img_registry, slot, is_selected)
                        {
                            gui_event.send(event);
                        }
                        if (i + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui::{self, Slider};

use crate::gui::{GuiAction, MaterialSetting, UiImages, UiState, TEXTURE_PREVIEW_SIZE_SMALL};
use crate::terrain_material::{
    MaterialSlot, TerrainMaterialParam, TerrainMaterialSet, TextureType,
};
// ----------------------------------------------------------------------------
#[rustfmt::skip]
fn show_material_selection(
    ui: &mut egui::Ui,
    ui_image_registry: &Res<UiImages>,
    slot: MaterialSlot,
    is_selected: bool,
) -> Option<GuiAction> {
    let mut result = None;

    ui.vertical_centered(|ui| {
        if ui.add(egui::widgets::ImageButton::new(
            ui_image_registry
                .get_imageid(&format!("terraintexture.{}.{}", TextureType::Diffuse, slot)),
            [TEXTURE_PREVIEW_SIZE_SMALL as f32, TEXTURE_PREVIEW_SIZE_SMALL as f32],
        ).selected(is_selected)).clicked() {
            result = if is_selected {
                Some(GuiAction::UnselectMaterial)
            } else {
                Some(GuiAction::SelectMaterial(slot))
            };
        }

        // --- just for debugging
        #[cfg(debug_assertions)]
        {
            egui::CollapsingHeader::new("normal")
                .id_source(format!("normal.{}", slot))
                .default_open(false)
                .show(ui, |ui| {
                    ui.add(egui::widgets::Image::new(
                        ui_image_registry
                            .get_imageid(&format!("terraintexture.{}.{}", TextureType::Normal, slot)),
                        [TEXTURE_PREVIEW_SIZE_SMALL as f32, TEXTURE_PREVIEW_SIZE_SMALL as f32],
                    ));
                });
        }

        #[allow(deprecated)]
        let label = egui::Label::new(format!("material #{}", *slot + 1)).small();
        if is_selected {
            #[allow(deprecated)]
            ui.add(label.strong());
        } else {
            ui.add(label);
        }
    });
    result
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
fn show_material_settings(
    ui: &mut egui::Ui,
    param: &mut TerrainMaterialParam,
) -> Option<MaterialSetting> {
    use MaterialSetting::*;

    let p = param;

    egui::CollapsingHeader::new("material parameters")
        .default_open(true)
        .show(ui, |ui| {
            let mut result = None;

            if ui.add(Slider::new(&mut p.blend_sharpness, 0.0..=1.0).text("blend sharpness")
                .fixed_decimals(5)
            ).changed() {
                result = Some(SetBlendSharpness(p.blend_sharpness));
            }
            if ui.add(Slider::new(&mut p.slope_base_dampening, 0.0..=1.0).text("slope base dampening")).changed() {
                result = Some(SetSlopeBaseDampening(p.slope_base_dampening));
            }
            if ui.add(Slider::new(&mut p.slope_normal_dampening, 0.0..=1.0).text("slope normal dampening")).changed() {
                result = Some(SetSlopeNormalDampening(p.slope_normal_dampening));
            }
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
