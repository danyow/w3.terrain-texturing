//
// toolbox::update - simple(r) actions for updating state, mapping to other actions
//
// ----------------------------------------------------------------------------
use bevy::prelude::MouseButton;

use crate::terrain_painting::PaintCommand;
use crate::terrain_render::BrushPointer;

use super::texturebrush;
use super::{PointerSettings, ToolSelection, ToolboxState};
// ----------------------------------------------------------------------------
#[inline(always)]
pub(super) fn update_brush_pointer(settings: &PointerSettings, brush_pointer: &mut BrushPointer) {
    brush_pointer.radius = settings.radius();
    brush_pointer.ring_width = settings.ring_width();
    brush_pointer.color = settings.color();
}
// ----------------------------------------------------------------------------
pub(super) fn update_brush_on_texture_selection(
    toolbox: &mut ToolboxState,
    brush: &mut BrushPointer,
) {
    toolbox.selection = Some(ToolSelection::Texturing);
    update_brush_pointer(&toolbox.pointer_settings(), brush);
}
// ----------------------------------------------------------------------------
#[inline(always)]
pub(super) fn create_texture_brush_paint_cmds(
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
