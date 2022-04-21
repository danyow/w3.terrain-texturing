// ----------------------------------------------------------------------------
// all scaling brush operations
// ----------------------------------------------------------------------------
use bevy::prelude::Color;

use super::{BrushSize, OverwriteProbability, PointerSettings, TextureScale, ToolBrushPointer};
// ----------------------------------------------------------------------------
pub(super) struct BrushSettings {
    pub size: BrushSize,
    pub scaling: TextureScale,
    pub adjust_values: bool,

    pub draw_probability: u8,
    pub variance: TextureScale,

    pub randomize: bool,
    pub use_variance: bool,
}
// ----------------------------------------------------------------------------
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
            color: Color::YELLOW,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl BrushSettings {
    // ------------------------------------------------------------------------
    pub fn overwrite_probability(&self) -> Option<OverwriteProbability> {
        if self.randomize && self.draw_probability < 100 {
            Some(self.draw_probability.into())
        } else {
            None
        }
    }
    // ------------------------------------------------------------------------
    pub fn value_variance(&self) -> Option<TextureScale> {
        if self.use_variance {
            Some(self.variance)
        } else {
            None
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
            size: BrushSize::default(),
            scaling: TextureScale::default(),
            adjust_values: true,

            draw_probability: 50,
            variance: TextureScale(1),
            randomize: false,
            use_variance: false,
        }
    }
}
// ----------------------------------------------------------------------------
