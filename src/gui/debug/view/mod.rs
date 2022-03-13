// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub(super) fn show_ui(
    mut egui_ctx: ResMut<EguiContext>,
    ui_images: Res<UiImages>,
    ui_state: Res<UiState>,
    clipmap_tracker: Res<crate::terrain_clipmap::ClipmapTracker>,
    texture_clipmap: Res<TextureControlClipmap>,
    tint_clipmap: Res<TintClipmap>,

    mut gui_event: EventWriter<GuiAction>,
) {
    if ui_state.fullscreen {
        return;
    }
    clipmap::show_window(
        &mut egui_ctx,
        &ui_images,
        &ui_state,
        &clipmap_tracker,
        &texture_clipmap,
        &tint_clipmap,
        &mut gui_event,
    );
}
// ----------------------------------------------------------------------------
pub use self::menu::show_menu;
// ----------------------------------------------------------------------------
mod clipmap;
mod menu;
// ----------------------------------------------------------------------------
use bevy::prelude::*;
use bevy_egui::EguiContext;

use crate::terrain_clipmap::{TextureControlClipmap, TintClipmap};

use super::{GuiAction, UiImages, UiState};
// ----------------------------------------------------------------------------
