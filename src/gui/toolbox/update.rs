//
// toolbox::update - simple(r) actions for updating state, mapping to other actions
//
// ----------------------------------------------------------------------------
use bevy::prelude::MouseButton;

use crate::terrain_material::MaterialSlot;
use crate::terrain_painting::{PaintCommand, PickedType, SlopeBlendThreshold, TextureScale};
use crate::terrain_render::{BrushPointer, TerrainMaterialSet, TerrainRenderSettings};

use super::common::BrushSize;
use super::{blendingbrush, scalingbrush, texturebrush};
use super::{MaterialSetting, PointerSettings, ToolSelection, ToolboxState};
// ----------------------------------------------------------------------------
#[inline(always)]
pub(super) fn update_material_settings(
    slot: MaterialSlot,
    action: &MaterialSetting,
    materialset: &mut TerrainMaterialSet,
) {
    use MaterialSetting::*;

    let param = &mut materialset.parameter[slot];

    match action {
        SetBlendSharpness(v) => param.blend_sharpness = *v,
        SetSlopeBaseDampening(v) => param.slope_base_dampening = *v,
        SetSlopeNormalDampening(v) => param.slope_normal_dampening = *v,
        SetSpecularityScale(v) => param.specularity_scale = *v,
        SetSpecularity(v) => param.specularity = *v,
        SetSpecularityBase(v) => param.specularity_base = *v,
        SetFalloff(v) => param.falloff = *v,
    }
}
// ----------------------------------------------------------------------------
pub(super) fn picker_selection(
    toolbox: &mut ToolboxState,
    brush_pointer: &mut BrushPointer,
    select: bool,
) {
    let mut pointer_settings = toolbox.pointer_settings();

    if select {
        let current_col = pointer_settings.color;

        pointer_settings.size = BrushSize::minimal();
        pointer_settings.ring_width = 0.25;
        pointer_settings.color.set_r(1.0);
        pointer_settings.color.set_g(current_col.g() * 0.25);
        pointer_settings.color.set_b(current_col.b() * 0.25);
    }
    update_brush_pointer(&pointer_settings, brush_pointer);
}
// ----------------------------------------------------------------------------
pub(super) fn render_only_overlay_material(
    brush: &mut texturebrush::BrushSettings,
    rendersettings: &mut TerrainRenderSettings,
    render_overlay: bool,
) {
    brush.show_only_overlay = render_overlay;
    if brush.show_only_overlay {
        brush.show_only_background = false;
    }

    rendersettings.reset_exclusive_view();
    rendersettings.ignore_bkgrnd_texture = render_overlay;
    if rendersettings.ignore_bkgrnd_texture {
        rendersettings.ignore_overlay_texture = false;
    }
}
// ----------------------------------------------------------------------------
pub(super) fn render_only_bkgrnd_material(
    brush: &mut texturebrush::BrushSettings,
    rendersettings: &mut TerrainRenderSettings,
    render_bkgrnd: bool,
) {
    brush.show_only_background = render_bkgrnd;
    if brush.show_only_background {
        brush.show_only_overlay = false;
    }

    rendersettings.reset_exclusive_view();
    rendersettings.ignore_overlay_texture = render_bkgrnd;
    if rendersettings.ignore_overlay_texture {
        rendersettings.ignore_bkgrnd_texture = false;
    }
}
// ----------------------------------------------------------------------------
pub(super) fn render_only_bkgrnd_scaling(
    brush: &mut scalingbrush::BrushSettings,
    rendersettings: &mut TerrainRenderSettings,
    show: bool,
) {
    brush.show_bkgrnd_scaling = show;

    rendersettings.reset_exclusive_view();
    rendersettings.show_bkgrnd_scaling = show;
}
// ----------------------------------------------------------------------------
pub(super) fn render_only_slopeblend_threshold(
    brush: &mut blendingbrush::BrushSettings,
    rendersettings: &mut TerrainRenderSettings,
    show: bool,
) {
    brush.show_blend_threshold = show;

    rendersettings.reset_exclusive_view();
    rendersettings.show_blend_threshold = show;
}
// ----------------------------------------------------------------------------
pub(super) fn on_changed_tool_selection(
    toolbox: &mut ToolboxState,
    brush_pointer: &mut BrushPointer,
    rendersettings: &mut TerrainRenderSettings,
) {
    // reset if it was previously set
    toolbox.reset_picker();

    if toolbox.has_projected_pointer() {
        update_brush_pointer(&toolbox.pointer_settings(), brush_pointer);
    }
    toolbox.sync_rendersettings(rendersettings);
}
// ----------------------------------------------------------------------------
#[inline(always)]
pub(super) fn update_brush_pointer(settings: &PointerSettings, brush_pointer: &mut BrushPointer) {
    brush_pointer.radius = settings.radius();
    brush_pointer.ring_width = settings.ring_width();
    brush_pointer.color = settings.color();
}
// ----------------------------------------------------------------------------
pub(super) fn update_brush_on_material_selection(
    toolbox: &mut ToolboxState,
    brush_pointer: &mut BrushPointer,
    rendersettings: &mut TerrainRenderSettings,
    overlay_selected: bool,
) {
    use ToolSelection::{MaterialParameters, Texturing};

    match toolbox.selection {
        Some(MaterialParameters) | None => {
            // texture is used in current tool -> no need to switch tool or
            // change active texture in brush
        }
        _ => {
            use texturebrush::BrushTexturesUsed::*;

            // switch to texturing brush...
            toolbox.selection = Some(Texturing);

            // ...and activate background or overlay texture selection of the brush
            // to allow direct painting with selected texture
            match toolbox.texture_brush.textures_used {
                Overlay if !overlay_selected => {
                    toolbox.texture_brush.textures_used = Background;
                }
                Background if overlay_selected => {
                    toolbox.texture_brush.textures_used = Overlay;
                }
                _ => {
                    // texture was already active
                }
            }

            update_brush_pointer(&toolbox.pointer_settings(), brush_pointer);
            // make sure the shortcut show_* button states are synced with current
            // rendersettings
            toolbox.sync_rendersettings(rendersettings);
        }
    }
}
// ----------------------------------------------------------------------------
pub(super) fn update_brush_on_blendthreshold_pick(
    toolbox: &mut ToolboxState,
    brush_pointer: &mut BrushPointer,
    value: SlopeBlendThreshold,
) {
    toolbox.blending_brush.slope_blend = value;
    toolbox.blending_brush.adjust_values = false;
    update_brush_pointer(&toolbox.pointer_settings(), &mut *brush_pointer);
}
// ----------------------------------------------------------------------------
pub(super) fn update_brush_on_scaling_pick(
    toolbox: &mut ToolboxState,
    brush_pointer: &mut BrushPointer,
    value: TextureScale,
) {
    toolbox.scaling_brush.scaling = value;
    toolbox.scaling_brush.adjust_values = false;
    update_brush_pointer(&toolbox.pointer_settings(), &mut *brush_pointer);
}
// ----------------------------------------------------------------------------
pub(super) fn create_texture_picker_cmds(
    button: MouseButton,
    settings: &texturebrush::BrushSettings,
) -> Vec<PickedType> {
    use texturebrush::BrushTexturesUsed::*;
    use PickedType::*;

    let mut cmds = Vec::default();

    // other buttons are ignored
    if let MouseButton::Left = button {
        match settings.textures_used {
            Overlay => {
                cmds.push(OverlayTexture);
            }
            Background => {
                cmds.push(BackgroundTexture);
            }
            OverlayAndBackground => {
                cmds.push(OverlayTexture);
                cmds.push(BackgroundTexture);
            }
        }
    }
    cmds
}
// ----------------------------------------------------------------------------
#[inline(always)]
pub(super) fn create_texture_paint_cmds(
    button: MouseButton,
    settings: &texturebrush::BrushSettings,
) -> Vec<PaintCommand> {
    use PaintCommand::*;

    let mut cmds = Vec::default();

    // other buttons are ignored for texture brush
    if let MouseButton::Left = button {
        // --- texture overwrite(s)
        use texturebrush::BrushTexturesUsed::*;

        let (p_overlay, p_bkgrnd) = settings.texture_probabilities();

        match settings.textures_used {
            Overlay if p_overlay.is_some() => cmds.push(RandomizedSetOverlayMaterial(
                p_overlay.unwrap(),
                settings.overlay_texture,
            )),
            Overlay => cmds.push(SetOverlayMaterial(settings.overlay_texture)),

            Background if p_bkgrnd.is_some() => cmds.push(RandomizedSetBackgroundMaterial(
                p_bkgrnd.unwrap(),
                settings.bkgrnd_texture,
            )),
            Background => cmds.push(SetBackgroundMaterial(settings.bkgrnd_texture)),

            OverlayAndBackground => {
                cmds.push(if let Some(prob) = p_overlay {
                    RandomizedSetOverlayMaterial(prob, settings.overlay_texture)
                } else {
                    SetOverlayMaterial(settings.overlay_texture)
                });

                cmds.push(if let Some(prob) = p_bkgrnd {
                    RandomizedSetBackgroundMaterial(prob, settings.bkgrnd_texture)
                } else {
                    SetBackgroundMaterial(settings.bkgrnd_texture)
                });
            }
        }
        // optional scaling overwrite
        if settings.overwrite_scale {
            cmds.push(SetBackgroundScaling(settings.scaling));
        }

        // optional blending overwrite
        if settings.overwrite_slope_blend {
            cmds.push(SetSlopeBlendThreshold(settings.slope_blend));
        }
    }
    cmds
}
// ----------------------------------------------------------------------------
#[inline(always)]
pub(super) fn create_blending_paint_cmds(
    button: MouseButton,
    settings: &blendingbrush::BrushSettings,
) -> Vec<PaintCommand> {
    use PaintCommand::*;

    let mut cmds = Vec::default();
    let overwrite_probability = settings.overwrite_probability();
    let variance = settings.value_variance();

    // other buttons are ignored for texture brush
    match button {
        MouseButton::Left if !settings.adjust_values => {
            // direct ovewrite of slope blend
            let paint_operation = match (overwrite_probability, variance) {
                (None, None) => SetSlopeBlendThreshold(settings.slope_blend),
                (None, Some(variance)) => {
                    SetSlopeBlendThresholdWithVariance(settings.slope_blend, variance)
                }
                (Some(prob), None) => RandomizedSetSlopeBlendThreshold(prob, settings.slope_blend),
                (Some(prob), Some(variance)) => RandomizedSetSlopeBlendThresholdWithVariance(
                    prob,
                    settings.slope_blend,
                    variance,
                ),
            };
            cmds.push(paint_operation);
        }

        MouseButton::Left if settings.adjust_values => {
            // relative increase of slope blend
            let paint_operation = match (overwrite_probability, variance) {
                (None, None) => IncreaseSlopeBlendThreshold,
                (None, Some(variance)) => IncreaseSlopeBlendThresholdWithVariance(variance),
                (Some(prob), None) => RandomizedIncreaseSlopeBlendThreshold(prob),
                (Some(prob), Some(variance)) => {
                    RandomizedIncreaseSlopeBlendThresholdWithVariance(prob, variance)
                }
            };
            cmds.push(paint_operation);
        }

        MouseButton::Right if settings.adjust_values => {
            // relative reduction of slope blend
            let paint_operation = match (overwrite_probability, variance) {
                (None, None) => ReduceSlopeBlendThreshold,
                (None, Some(variance)) => ReduceSlopeBlendThresholdWithVariance(variance),
                (Some(prob), None) => RandomizedReduceSlopeBlendThreshold(prob),
                (Some(prob), Some(variance)) => {
                    RandomizedReduceSlopeBlendThresholdWithVariance(prob, variance)
                }
            };
            cmds.push(paint_operation);
        }
        _ => {}
    }

    cmds
}
// ----------------------------------------------------------------------------
#[inline(always)]
pub(super) fn create_scaling_paint_cmds(
    button: MouseButton,
    settings: &scalingbrush::BrushSettings,
) -> Vec<PaintCommand> {
    use PaintCommand::*;

    let mut cmds = Vec::default();
    let overwrite_probability = settings.overwrite_probability();
    let variance = settings.value_variance();

    // other buttons are ignored for texture brush
    match button {
        MouseButton::Left if !settings.adjust_values => {
            // direct ovewrite of scaling
            let paint_operation = match (overwrite_probability, variance) {
                (None, None) => SetBackgroundScaling(settings.scaling),
                (None, Some(variance)) => {
                    SetBackgroundScalingWithVariance(settings.scaling, variance)
                }
                (Some(prob), None) => RandomizedSetBackgroundScaling(prob, settings.scaling),
                (Some(prob), Some(variance)) => {
                    RandomizedSetBackgroundScalingWithVariance(prob, settings.scaling, variance)
                }
            };
            cmds.push(paint_operation);
        }

        MouseButton::Left if settings.adjust_values => {
            // relative increase of scaling
            let paint_operation = match (overwrite_probability, variance) {
                (None, None) => IncreaseBackgroundScaling,
                (None, Some(variance)) => IncreaseBackgroundScalingWithVariance(variance),
                (Some(prob), None) => RandomizedIncreaseBackgroundScaling(prob),
                (Some(prob), Some(variance)) => {
                    RandomizedIncreaseBackgroundScalingWithVariance(prob, variance)
                }
            };
            cmds.push(paint_operation);
        }

        MouseButton::Right if settings.adjust_values => {
            // relative reduction of scaling
            let paint_operation = match (overwrite_probability, variance) {
                (None, None) => ReduceBackgroundScaling,
                (None, Some(variance)) => ReduceBackgroundScalingWithVariance(variance),
                (Some(prob), None) => RandomizedReduceBackgroundScaling(prob),
                (Some(prob), Some(variance)) => {
                    RandomizedReduceBackgroundScalingWithVariance(prob, variance)
                }
            };
            cmds.push(paint_operation);
        }
        _ => {}
    }

    cmds
}
// ----------------------------------------------------------------------------
