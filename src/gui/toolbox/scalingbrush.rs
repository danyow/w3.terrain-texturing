// ----------------------------------------------------------------------------
// all scaling brush operations
// ----------------------------------------------------------------------------
use bevy::prelude::Color;

use crate::terrain_render::TerrainRenderSettings;

use super::{OverwriteProbability, TextureScale, ToolSettings, Variance};
// ----------------------------------------------------------------------------
pub(super) struct BrushSettings {
    pub scaling: TextureScale,
    pub adjust_values: bool,

    pub draw_probability: u8,
    pub variance: Variance,

    pub randomize: bool,
    pub use_variance: bool,

    pub show_bkgrnd_scaling: bool,
}
// ----------------------------------------------------------------------------
impl ToolSettings for BrushSettings {
    // ------------------------------------------------------------------------
    fn pointer_color(&self) -> Color {
        Color::YELLOW
    }
    // ------------------------------------------------------------------------
    fn sync_rendersettings(&mut self, settings: &mut TerrainRenderSettings) {
        settings.reset_exclusive_view();

        // scaling is used on background texture so it should be switched on
        settings.ignore_bkgrnd_texture = false;

        // if debug was previously set, set it again
        if self.show_bkgrnd_scaling {
            settings.show_bkgrnd_scaling = true;
        }
        self.show_bkgrnd_scaling = settings.show_bkgrnd_scaling;
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

            show_bkgrnd_scaling: false,
        }
    }
}
// ----------------------------------------------------------------------------
