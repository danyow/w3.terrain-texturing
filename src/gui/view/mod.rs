// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub(super) fn show_ui(
    mut egui_ctx: ResMut<EguiContext>,
    ui_images: Res<UiImages>,
    ui_state: Res<UiState>,
    materialset: Option<Res<TerrainMaterialSet>>,
    mesh_settings: Option<Res<TerrainMeshSettings>>,
    sun_settings: Option<Res<SunSettings>>,
    atmosphere_settings: Option<Res<AtmosphereMat>>,
    mut gui_event: EventWriter<GuiAction>,
    // dbg
    clipmap_tracker: Res<crate::terrain_clipmap::ClipmapTracker>,
    texture_clipmap: Res<TextureControlClipmap>,
    tint_clipmap: Res<TintClipmap>,
) {
    if ui_state.fullscreen {
        return;
    }
    debug::show_windows(
        &mut egui_ctx,
        &ui_images,
        &ui_state,
        &clipmap_tracker,
        &texture_clipmap,
        &tint_clipmap,
        &mut gui_event,
    );

    menu::show(&mut egui_ctx, &ui_state, &mut gui_event);

    egui::SidePanel::right("side_panel")
        .width_range(300.0..=450.0)
        .show(egui_ctx.ctx_mut(), |ui| {
            egui::ScrollArea::vertical()
                .max_height(ui.available_height() - 45.0)
                .show(ui, |ui| {
                    if let Some(settings) = mesh_settings {
                        mesh::show_settings(ui, &settings, &mut gui_event);
                    }
                    if let Some(settings) = sun_settings {
                        atmosphere::show_sun_settings(ui, &settings, &mut gui_event);
                    }
                    if let Some(settings) = atmosphere_settings {
                        atmosphere::show_atmosphere_settings(ui, &settings, &mut gui_event);
                    }
                    if let Some(materialset) = materialset {
                        materialpalette::show(
                            ui,
                            &ui_images,
                            &*ui_state,
                            &materialset,
                            &mut gui_event,
                        );
                    }
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
use crate::terrain_clipmap::{TextureControlClipmap, TintClipmap};
use crate::terrain_material::TerrainMaterialSet;
use crate::terrain_tiles::TerrainMeshSettings;
use crate::SunSettings;

use super::{GuiAction, UiImages, UiState};
// ----------------------------------------------------------------------------
mod atmosphere;
mod materialpalette;
mod menu;
mod mesh;

mod debug;
// ----------------------------------------------------------------------------
