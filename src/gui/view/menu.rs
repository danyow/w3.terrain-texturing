// ----------------------------------------------------------------------------
#[inline]
pub fn show(
    egui_ctx: &mut EguiContext,
    ui_state: &UiState,
    gui_event: &mut EventWriter<GuiAction>,
) {
    egui::TopBottomPanel::top("top_panel").show(egui_ctx.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            ui.set_enabled(ui_state.enabled);

            ui.menu_button("Project", |ui| {
                if ui.button("Quit").clicked() {
                    ui.close_menu();
                    gui_event.send(GuiAction::QuitRequest);
                }
            });
            // #[cfg(debug_assertions))]
            {
                ui.add_space(50.0);
                ui.separator();
                crate::gui::debug::show_menu(ui, ui_state, gui_event);
            }
        });
    });
}
// ----------------------------------------------------------------------------
use bevy::prelude::EventWriter;
use bevy_egui::{egui, EguiContext};

use super::{GuiAction, UiState};
// ----------------------------------------------------------------------------
