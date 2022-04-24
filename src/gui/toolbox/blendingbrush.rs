// ----------------------------------------------------------------------------
// all scaling brush operations
// ----------------------------------------------------------------------------
use bevy::prelude::Color;

use crate::terrain_render::TerrainRenderSettings;

use super::{OverwriteProbability, SlopeBlendThreshold, ToolSettings, Variance};
// ----------------------------------------------------------------------------
pub(super) struct BrushSettings {
    pub slope_blend: SlopeBlendThreshold,
    pub adjust_values: bool,

    pub draw_probability: u8,
    pub variance: Variance,

    pub randomize: bool,
    pub use_variance: bool,

    pub show_blend_threshold: bool,
    pub picker_activated: bool,
}
// ----------------------------------------------------------------------------
impl ToolSettings for BrushSettings {
    // ------------------------------------------------------------------------
    fn pointer_color(&self) -> Color {
        Color::BLUE
    }
    // ------------------------------------------------------------------------
    fn sync_rendersettings(&mut self, settings: &mut TerrainRenderSettings) {
        settings.reset_exclusive_view();

        // blending is only visible if both texture are activated
        settings.ignore_overlay_texture = false;
        settings.ignore_bkgrnd_texture = false;

        // if it was previously set, set it again
        if self.show_blend_threshold {
            settings.show_blend_threshold = true;
        }
        self.show_blend_threshold = settings.show_blend_threshold;
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

            show_blend_threshold: false,
            picker_activated: false,
        }
    }
}
// ----------------------------------------------------------------------------
