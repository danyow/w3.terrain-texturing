// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub(super) fn show_window(
    egui_ctx: &mut EguiContext,
    ui_images: &Res<UiImages>,
    ui_state: &Res<UiState>,
    clipmap_tracker: &Res<crate::terrain_clipmap::ClipmapTracker>,
    texture_clipmap: &Res<TextureControlClipmap>,
    tint_clipmap: &Res<TintClipmap>,
    heightmap_clipmap: &Res<crate::terrain_clipmap::HeightmapClipmap>,
    gui_event: &mut EventWriter<GuiAction>,
) {
    let mut opened = ui_state.debug.show_clipmaps;
    egui::Window::new("DEBUG: clipmap")
        .open(&mut opened)
        .default_size((400.0, 600.0))
        .vscroll(true)
        .hscroll(true)
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.label(format!(
                "Conf: {}x{} #{} level (data: {}x{})",
                CLIPMAP_SIZE,
                CLIPMAP_SIZE,
                clipmap_tracker.level_count(),
                clipmap_tracker.datasource_size(),
                clipmap_tracker.datasource_size(),
            ));

            for label in [
                texture_clipmap.label(),
                tint_clipmap.label(),
                heightmap_clipmap.label(),
            ] {
                egui::CollapsingHeader::new(label)
                    .default_open(true)
                    .show(ui, |ui| {
                        for (i, l) in clipmap_tracker.layers() {
                            egui::CollapsingHeader::new(format!("level {}", i))
                                .default_open(false)
                                .show(ui, |ui| {
                                    ui.add(egui::widgets::Image::new(
                                        ui_images.get_imageid(&format!("clipmap.{}.{}", label, i)),
                                        [256.0, 256.0],
                                    ));
                                });
                            ui.label(format!(
                                "rectangle: ({} / {}) - ({} / {})  {}x{}",
                                l.rectangle().pos.x,
                                l.rectangle().pos.y,
                                l.rectangle().pos.x + l.rectangle().size.x,
                                l.rectangle().pos.y + l.rectangle().size.y,
                                l.rectangle().size.x,
                                l.rectangle().size.y
                            ));
                        }
                    });
            }
        });

    if opened != ui_state.debug.show_clipmaps {
        gui_event.send(GuiAction::DebugShowClipmap(opened));
    }
}
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_egui::EguiContext;

use crate::config::CLIPMAP_SIZE;
use crate::terrain_clipmap::{TextureControlClipmap, TintClipmap};

use super::{GuiAction, UiImages, UiState};
// ----------------------------------------------------------------------------
