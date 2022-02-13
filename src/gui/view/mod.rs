// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub(super) fn show_ui(
    egui_ctx: Res<EguiContext>,
    ui_state: Res<UiState>,
    sun_settings: Option<Res<SunSettings>>,
    atmosphere_settings: Option<Res<AtmosphereMat>>,
    mut gui_event: EventWriter<GuiAction>,
) {
    if ui_state.fullscreen {
        return;
    }
    menu::show(&egui_ctx, &mut gui_event);

    egui::SidePanel::right("side_panel")
        .width_range(300.0..=450.0)
        .show(egui_ctx.ctx(), |ui| {
            // ui.heading("Side Panel");

            egui::ScrollArea::vertical()
                // .max_height(f32::INFINITY)
                // .auto_shrink([true, true])
                .max_height(ui.available_height() - 45.0)
                .show(ui, |ui| {
                    if let Some(settings) = sun_settings {
                        atmosphere::show_sun_settings(ui, &settings, &mut gui_event);
                    }
                    if let Some(settings) = atmosphere_settings {
                        atmosphere::show_atmosphere_settings(ui, &settings, &mut gui_event);
                    }
                });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.label("powered by bevy & egui");
            });
        });
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

use crate::{atmosphere::AtmosphereMat, SunSettings};

use super::{GuiAction, UiState};
// ----------------------------------------------------------------------------
mod atmosphere;
mod menu;

mod debug;
// ----------------------------------------------------------------------------
