// ----------------------------------------------------------------------------
// all scaling brush operations
// ----------------------------------------------------------------------------
use bevy::prelude::Color;

use super::{OverwriteProbability, TextureScale, ToolBrushPointer, Variance};
// ----------------------------------------------------------------------------
pub(super) struct BrushSettings {
    pub scaling: TextureScale,
    pub adjust_values: bool,

    pub draw_probability: u8,
    pub variance: Variance,

    pub randomize: bool,
    pub use_variance: bool,
}
// ----------------------------------------------------------------------------
impl ToolBrushPointer for BrushSettings {
    // ------------------------------------------------------------------------
    fn pointer_color(&self) -> Color {
        Color::YELLOW
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
    pub fn value_variance(&self) -> Option<Variance> {
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
            scaling: TextureScale::default(),
            adjust_values: true,

            draw_probability: 50,
            variance: Variance(1),
            randomize: false,
            use_variance: false,
        }
    }
}
// ----------------------------------------------------------------------------
