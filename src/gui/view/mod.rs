// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[allow(clippy::too_many_arguments)]
pub(super) fn show_ui(
    mut egui_ctx: ResMut<EguiContext>,
    mut ui_state: ResMut<UiState>,
    ui_images: Res<UiImages>,
    materialset: Res<TerrainMaterialSet>,
    mesh_settings: Option<Res<TerrainMeshSettings>>,
    sun_settings: Option<Res<SunSettings>>,
    atmosphere_settings: Option<Res<AtmosphereMat>>,
    mesh_stats: Res<TerrainStats>,
    mut render_settings: ResMut<TerrainRenderSettings>,
    mut gui_event: EventWriter<GuiAction>,
) {
    if ui_state.fullscreen {
        return;
    }
    menu::show(&mut egui_ctx, &ui_state, &mut gui_event);

    egui::SidePanel::right("side_panel")
        .resizable(ui_state.enabled)
        .width_range(300.0..=500.0)
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.set_enabled(ui_state.enabled);

            egui::ScrollArea::vertical()
                .max_height(ui.available_height() - 45.0)
                .show(ui, |ui| {
                    rendersettings::show_settings(ui, &mut *render_settings, &mut gui_event);

                    if let Some(settings) = mesh_settings {
                        mesh::show_settings(ui, &settings, &mesh_stats, &mut gui_event);
                    }
                    if let Some(settings) = sun_settings {
                        atmosphere::show_sun_settings(ui, &settings, &mut gui_event);
                    }
                    if let Some(settings) = atmosphere_settings {
                        atmosphere::show_atmosphere_settings(ui, &settings, &mut gui_event);
                    }

                    super::toolbox::view::show_ui(
                        ui, &mut ui_state.toolbox, &ui_images, materialset, &mut gui_event);

                });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.label("powered by bevy & egui");
                if let Some(task_tracking) = ui_state.progress.task() {
                    ui.add(
                        egui::ProgressBar::new(task_tracking.progress())
                            .text(task_tracking.last_msg()),
                    );
                }
            });
        });
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

use crate::atmosphere::AtmosphereMat;
use crate::environment::SunSettings;
use crate::terrain_material::TerrainMaterialSet;
use crate::terrain_render::TerrainRenderSettings;
use crate::terrain_tiles::{TerrainMeshSettings, TerrainStats};

use super::{GuiAction, UiExtension, UiImages, UiState};
// ----------------------------------------------------------------------------
mod atmosphere;
mod menu;
mod mesh;
mod rendersettings;
// ----------------------------------------------------------------------------
