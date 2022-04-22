// ----------------------------------------------------------------------------
// all texture brush operations
// ----------------------------------------------------------------------------
use bevy::prelude::Color;

use bevy_egui::egui;

use crate::terrain_material::MaterialSlot;

use super::{OverwriteProbability, SlopeBlendThreshold, TextureScale, ToolBrushPointer};
// ----------------------------------------------------------------------------
pub(super) struct BrushSettings {
    pub overlay_texture: MaterialSlot,
    pub bkgrnd_texture: MaterialSlot,
    pub scaling: TextureScale,
    pub slope_blend: SlopeBlendThreshold,
    pub texture_probabilities: (u8, u8),

    pub textures_used: BrushTexturesUsed,
    pub overwrite_scale: bool,
    pub overwrite_slope_blend: bool,
    pub randomize: bool,
}
// ----------------------------------------------------------------------------
#[derive(Eq, PartialEq, Clone, Copy)]
pub(super) enum BrushTexturesUsed {
    Overlay,
    Background,
    OverlayAndBackground,
}
// ----------------------------------------------------------------------------
impl ToolBrushPointer for BrushSettings {
    // ------------------------------------------------------------------------
    fn pointer_color(&self) -> Color {
        self.textures_used.pointer_color()
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl BrushSettings {
    // ------------------------------------------------------------------------
    pub fn texture_probabilities(
        &self,
    ) -> (Option<OverwriteProbability>, Option<OverwriteProbability>) {
        if self.randomize {
            let mut overlay = None;
            let mut bkgrnd = None;

            if self.texture_probabilities.0 < 100 {
                overlay = Some(self.texture_probabilities.0.into());
            }
            if self.texture_probabilities.1 < 100 {
                bkgrnd = Some(self.texture_probabilities.1.into());
            }
            (overlay, bkgrnd)
        } else {
            (None, None)
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl BrushTexturesUsed {
    // ------------------------------------------------------------------------
    pub const fn pointer_color(&self) -> Color {
        match self {
            BrushTexturesUsed::Overlay => Color::SEA_GREEN,
            BrushTexturesUsed::Background => Color::LIME_GREEN,
            BrushTexturesUsed::OverlayAndBackground => Color::YELLOW_GREEN,
        }
    }
    // ------------------------------------------------------------------------
    pub const fn selection_color(&self) -> egui::Color32 {
        match self {
            BrushTexturesUsed::Overlay => egui::Color32::LIGHT_GREEN,
            BrushTexturesUsed::Background => egui::Color32::LIGHT_BLUE,
            BrushTexturesUsed::OverlayAndBackground => egui::Color32::from_rgb(153, 204, 51),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// Default
// ----------------------------------------------------------------------------
impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            overlay_texture: MaterialSlot::from(2),
            bkgrnd_texture: MaterialSlot::from(1),
            scaling: TextureScale::default(),
            slope_blend: SlopeBlendThreshold::default(),
            texture_probabilities: (50, 50),
            textures_used: BrushTexturesUsed::default(),
            randomize: false,
            overwrite_scale: false,
            overwrite_slope_blend: false,
        }
    }
}
// ----------------------------------------------------------------------------
impl Default for BrushTexturesUsed {
    fn default() -> Self {
        Self::OverlayAndBackground
    }
}
// ----------------------------------------------------------------------------
