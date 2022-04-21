// ----------------------------------------------------------------------------
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::terrain_material::MaterialSlot;
use crate::terrain_painting::{
    BrushPlacement, OverwriteProbability, PaintingEvent, SlopeBlendThreshold, TextureScale,
};
use crate::terrain_render::{
    BrushPointer, BrushPointerEventData, BrushPointerEventReceiver, TerrainMaterialSet,
};

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
            )
            .add_system(process_brush_clicks.before("handle_ui_actions"));
    }
    // ------------------------------------------------------------------------
}
// ---------------------------------------------------------------------------
#[derive(Default)]
pub struct ToolboxState {
    pub enabled: bool,
    selection: Option<ToolSelection>,
    texture_brush: texturebrush::BrushSettings,
    scaling_brush: scalingbrush::BrushSettings,
}
// ---------------------------------------------------------------------------
#[derive(Debug)]
/// Events triggered by user in the GUI (user actions)
pub enum ToolboxAction {
    UpdateBrushSettings,
    SelectOverlayTexture(MaterialSlot),
    SelectBackgroundTexture(MaterialSlot),
    UpdateMaterial(MaterialSlot, MaterialSetting),
}
// ---------------------------------------------------------------------------
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum MaterialSetting {
    SetBlendSharpness(f32),
    SetSlopeBaseDampening(f32),
    SetSlopeNormalDampening(f32),
    SetSpecularityScale(f32),
    SetSpecularity(f32),
    SetSpecularityBase(f32),
    SetFalloff(f32),
}
// ---------------------------------------------------------------------------
#[derive(Eq, PartialEq, Clone, Copy)]
enum ToolSelection {
    Texturing,
    Scaling,
    MaterialParameters,
}
// ----------------------------------------------------------------------------
mod common;
mod scalingbrush;
mod texturebrush;

mod update;
pub(super) mod view;
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
fn process_brush_clicks(
    receiver: Res<BrushPointerEventReceiver>,
    ui_state: Res<UiState>,
    mut editor_events: EventWriter<PaintingEvent>,
) {
    use ToolSelection::*;

    while let Ok(BrushPointerEventData::Centered(button, pos, radius)) = receiver.try_recv() {
        let settings = &ui_state.toolbox;

        if let Some(selection) = settings.selection {
            let cmds = match selection {
                Texturing => {
                    update::create_texture_brush_paint_cmds(button, &settings.texture_brush)
                }
                Scaling => update::create_scaling_brush_paint_cmds(button, &settings.scaling_brush),
                MaterialParameters => continue,
            };
            if !cmds.is_empty() {
                editor_events.send(PaintingEvent::new(BrushPlacement::new(pos, radius), cmds));
            }
        }
    }
}
// ----------------------------------------------------------------------------
fn handle_ui_actions(
    mut ui_state: ResMut<UiState>,
    mut ui_action: EventReader<GuiAction>,
    mut brush: ResMut<BrushPointer>,
    mut materialset: ResMut<TerrainMaterialSet>,
) {
    use ToolboxAction::*;

    for action in ui_action.iter() {
        if let GuiAction::Toolbox(action) = action {
            match action {
                UpdateBrushSettings => {
                    update::update_brush_pointer(&ui_state.toolbox.pointer_settings(), &mut *brush);
                }
                SelectOverlayTexture(material_slot) => {
                    ui_state.toolbox.texture_brush.overlay_texture = *material_slot;
                    update::update_brush_on_texture_selection(&mut ui_state.toolbox, &mut *brush);
                }
                SelectBackgroundTexture(material_slot) => {
                    ui_state.toolbox.texture_brush.bkgrnd_texture = *material_slot;
                    update::update_brush_on_texture_selection(&mut ui_state.toolbox, &mut *brush);
                }
                UpdateMaterial(slot, setting) => {
                    update::update_material_settings(*slot, setting, &mut *materialset);
                }
            }
        }
    }
}
// ----------------------------------------------------------------------------
fn init_brush_pointer(mut brush_pointer: ResMut<BrushPointer>) {
    update::update_brush_pointer(&PointerSettings::default(), &mut brush_pointer);
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

        brush_pointer.active = ui_state.toolbox.has_projected_pointer();
        if brush_pointer.active {
            if let Some(mouse_pos) = win.cursor_position() {
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
    fn has_projected_pointer(&self) -> bool {
        use ToolSelection::*;
        match self.selection {
            Some(Texturing) | Some(Scaling) => true,
            Some(MaterialParameters) | None => false,
        }
    }
    // ------------------------------------------------------------------------
    fn rescale_pointer(&mut self, scale: f32) {
        use ToolSelection::*;
        match self.selection {
            Some(Texturing) => self.texture_brush.scale_pointer(scale),
            Some(Scaling) => self.scaling_brush.scale_pointer(scale),
            Some(MaterialParameters) | None => {}
        }
    }
    // ------------------------------------------------------------------------
    fn pointer_settings(&self) -> PointerSettings {
        use ToolSelection::*;
        match self.selection {
            Some(Texturing) => self.texture_brush.pointer_settings(),
            Some(Scaling) => self.scaling_brush.pointer_settings(),
            Some(MaterialParameters) | None => {
                // pointer should be deactivated, see has_projected_pointer
                unreachable!("pointer should have been deactivated!")
            }
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
