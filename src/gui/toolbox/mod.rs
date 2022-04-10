// ----------------------------------------------------------------------------
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::terrain_render::BrushPointer;

use common::{BrushSize, PointerSettings};

use super::{GuiAction, UiState};
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct TexturingToolboxPlugin;
// ----------------------------------------------------------------------------
impl Plugin for TexturingToolboxPlugin {
    // ------------------------------------------------------------------------
    fn build(&self, app: &mut App) {
        app.add_startup_system(init_brush_pointer)
            .add_system(update_brush_pointer_info)
            .add_system(
                handle_ui_actions
                    .label("handle_ui_actions")
                    .after("gui_actions"),
            );
    }
    // ------------------------------------------------------------------------
}
// ---------------------------------------------------------------------------
#[derive(Default)]
pub struct ToolboxState {
    pub enabled: bool,
    selection: ToolSelection,
    texture_brush: texturebrush::BrushSettings,
}
// ---------------------------------------------------------------------------
#[derive(Debug)]
/// Events triggered by user in the GUI (user actions)
pub enum ToolboxAction {
    UpdateBrushSettings,
}
// ---------------------------------------------------------------------------
#[derive(Eq, PartialEq, Clone, Copy)]
enum ToolSelection {
    Texturing,
}
// ----------------------------------------------------------------------------
mod common;
mod texturebrush;

mod update;
pub(super) mod view;
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn handle_ui_actions(
    mut ui_state: ResMut<UiState>,
    mut ui_action: EventReader<GuiAction>,
    mut brush: ResMut<BrushPointer>,
) {
    use ToolboxAction::*;

    for action in ui_action.iter() {
        if let GuiAction::Toolbox(action) = action {
            match action {
                UpdateBrushSettings => {
                    update::update_brush_pointer(&ui_state.toolbox.pointer_settings(), &mut *brush);
                }
            }
        }
    }
}
// ----------------------------------------------------------------------------
fn init_brush_pointer(ui_state: ResMut<UiState>, mut brush_pointer: ResMut<BrushPointer>) {
    update::update_brush_pointer(&ui_state.toolbox.pointer_settings(), &mut brush_pointer);
}
// ----------------------------------------------------------------------------
fn update_brush_pointer_info(
    mut ui_state: ResMut<UiState>,
    mut brush_pointer: ResMut<BrushPointer>,
    mouse_input: Res<Input<MouseButton>>,
    mut mouse_wheel: EventReader<MouseWheel>,
    windows: Res<Windows>,
) {
    // check if cursor is not over gui or used by gui (slider draging into 3d area)
    if ui_state.toolbox.enabled && !ui_state.wants_input() {
        let win = windows.get_primary().expect("no primary window");

        if let Some(mouse_pos) = win.cursor_position() {
            brush_pointer.active = true;

            brush_pointer.pos = mouse_pos * win.scale_factor() as f32;
            brush_pointer.click_primary = mouse_input.just_pressed(MouseButton::Left);
            brush_pointer.click_secondary = mouse_input.just_pressed(MouseButton::Right);

            for e in mouse_wheel.iter() {
                ui_state.toolbox.rescale_pointer(e.y);

                update::update_brush_pointer(
                    &ui_state.toolbox.pointer_settings(),
                    &mut brush_pointer,
                );
            }
        } else {
            brush_pointer.active = false;
        }
    } else {
        brush_pointer.active = false;
    }
}
// ----------------------------------------------------------------------------
// toolbox state
// ----------------------------------------------------------------------------
trait ToolBrushPointer {
    fn scale_pointer(&mut self, scale: f32);
    fn pointer_settings(&self) -> PointerSettings;
}
// ----------------------------------------------------------------------------
impl ToolboxState {
    // ------------------------------------------------------------------------
    fn rescale_pointer(&mut self, scale: f32) {
        match self.selection {
            ToolSelection::Texturing => self.texture_brush.scale_pointer(scale),
        }
    }
    // ------------------------------------------------------------------------
    fn pointer_settings(&self) -> PointerSettings {
        match self.selection {
            ToolSelection::Texturing => self.texture_brush.pointer_settings(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// Default impl
// ----------------------------------------------------------------------------
impl Default for ToolSelection {
    fn default() -> Self {
        Self::Texturing
    }
}
// ----------------------------------------------------------------------------
