// ----------------------------------------------------------------------------
#[rustfmt::skip]
pub(super) fn show_debug_menu(
    ui: &mut egui::Ui,
    _gui_event: &mut EventWriter<GuiAction>,
) {
    ui.menu_button("Debug", |_ui| {});
}
// ----------------------------------------------------------------------------
use bevy::prelude::EventWriter;
use bevy_egui::egui;

use crate::gui::GuiAction;
// ----------------------------------------------------------------------------
