//
// toolbox::update - simple(r) actions for updating state, mapping to other actions
//
// ----------------------------------------------------------------------------
use crate::terrain_render::BrushPointer;

use super::PointerSettings;
// ----------------------------------------------------------------------------
#[inline(always)]
pub(super) fn update_brush_pointer(settings: &PointerSettings, brush_pointer: &mut BrushPointer) {
    brush_pointer.radius = settings.radius();
    brush_pointer.ring_width = settings.ring_width();
    brush_pointer.color = settings.color();
}
// ----------------------------------------------------------------------------
