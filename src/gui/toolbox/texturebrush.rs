// ----------------------------------------------------------------------------
// all texture brush operations
// ----------------------------------------------------------------------------
use bevy::prelude::Color;

use crate::terrain_material::MaterialSlot;

use super::{BrushSize, OverwriteProbability, PointerSettings, ToolBrushPointer};
// ----------------------------------------------------------------------------
pub(super) struct BrushSettings {
    pub size: BrushSize,
    pub overlay_texture: MaterialSlot,
    pub bkgrnd_texture: MaterialSlot,
    pub texture_probabilities: (u8, u8),

    pub textures_used: BrushTexturesUsed,
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
    fn scale_pointer(&mut self, scale: f32) {
        self.size.scale(scale);
    }
    // ------------------------------------------------------------------------
    fn pointer_settings(&self) -> PointerSettings {
        PointerSettings {
            size: self.size,
            ring_width: self.size.ring_width(),
            color: match self.textures_used {
                BrushTexturesUsed::Overlay => Color::SEA_GREEN,
                BrushTexturesUsed::Background => Color::LIME_GREEN,
                BrushTexturesUsed::OverlayAndBackground => Color::YELLOW_GREEN,
            },
        }
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
// Default
// ----------------------------------------------------------------------------
impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            overlay_texture: MaterialSlot::from(2),
            bkgrnd_texture: MaterialSlot::from(1),
            texture_probabilities: (50, 50),
            size: BrushSize::default(),
            textures_used: BrushTexturesUsed::default(),
            randomize: false,
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
