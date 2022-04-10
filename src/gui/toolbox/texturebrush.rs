// ----------------------------------------------------------------------------
// all texture brush operations
// ----------------------------------------------------------------------------
use bevy::prelude::Color;

use super::{BrushSize, PointerSettings, ToolBrushPointer};
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(super) struct BrushSettings {
    size: BrushSize,
}
// ----------------------------------------------------------------------------
impl ToolBrushPointer for BrushSettings {
    fn scale_pointer(&mut self, scale: f32) {
        self.size.scale(scale);
    }

    fn pointer_settings(&self) -> PointerSettings {
        PointerSettings {
            size: self.size,
            ring_width: self.size.ring_width(),
            color: Color::GREEN,
        }
    }
}
// ----------------------------------------------------------------------------
