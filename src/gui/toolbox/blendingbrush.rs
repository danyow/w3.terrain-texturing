// ----------------------------------------------------------------------------
// all scaling brush operations
// ----------------------------------------------------------------------------
use bevy::prelude::Color;

use super::{OverwriteProbability, SlopeBlendThreshold, ToolBrushPointer, Variance};
// ----------------------------------------------------------------------------
pub(super) struct BrushSettings {
    pub slope_blend: SlopeBlendThreshold,
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
        Color::BLUE
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
            slope_blend: SlopeBlendThreshold::default(),
            adjust_values: true,

            draw_probability: 25,
            variance: Variance(2),
            randomize: false,
            use_variance: false,
        }
    }
}
// ----------------------------------------------------------------------------
