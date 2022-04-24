// ----------------------------------------------------------------------------
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::terrain_material::MaterialSlot;
use crate::terrain_painting::{
    BrushPlacement, OverwriteProbability, PaintingEvent, PickedType, PickerEvent, PickerResult,
    PickerResultEvent, SlopeBlendThreshold, TextureScale, Variance,
};
use crate::terrain_render::{
    BrushPointer, BrushPointerEventData, BrushPointerEventReceiver, TerrainMaterialSet,
    TerrainRenderSettings,
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
            .add_system(process_brush_clicks.before("handle_ui_actions"))
            .add_system(process_picker_results);
    }
    // ------------------------------------------------------------------------
}
// ---------------------------------------------------------------------------
#[derive(Default)]
pub struct ToolboxState {
    pub enabled: bool,
    selection: Option<ToolSelection>,
    texture_brush: texturebrush::BrushSettings,
    blending_brush: blendingbrush::BrushSettings,
    scaling_brush: scalingbrush::BrushSettings,

    brush_size: BrushSize,
}
// ---------------------------------------------------------------------------
#[derive(Debug)]
/// Events triggered by user in the GUI (user actions)
pub enum ToolboxAction {
    ChangedToolSelection,
    UpdateBrushSettings,
    SelectOverlayTexture(MaterialSlot),
    SelectBackgroundTexture(MaterialSlot),
    UpdateMaterial(MaterialSlot, MaterialSetting),
    ShowOnlyOverlayTexture(bool),
    ShowOnlyBackgroundTexture(bool),
    ShowBackgroundScaling(bool),
    ShowSlopeBlendThreshold(bool),
    TexturePickerSelected(bool),
    SlopeBlendThresholdPickerSelected(bool),
    BkgrndScalingPickerSelected(bool),
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
    Blending,
    Scaling,
    MaterialParameters,
}
// ----------------------------------------------------------------------------
mod common;

mod blendingbrush;
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
    mut painting_events: EventWriter<PaintingEvent>,
    mut picker_events: EventWriter<PickerEvent>,
) {
    use PickedType::*;
    use ToolSelection::*;

    while let Ok(BrushPointerEventData::Centered(button, pos, radius)) = receiver.try_recv() {
        let settings = &ui_state.toolbox;

        if let Some(selection) = settings.selection {
            let placement = BrushPlacement::new(pos, radius);
            match selection {
                // -- picker
                Texturing if settings.texture_brush.picker_activated => {
                    let cmds = update::create_texture_picker_cmds(button, &settings.texture_brush);
                    picker_events.send(PickerEvent::new(placement, cmds));
                }
                Blending if settings.blending_brush.picker_activated => {
                    picker_events.send(PickerEvent::new(placement, vec![SlopeBlendThreshold]));
                }
                Scaling if settings.scaling_brush.picker_activated => {
                    picker_events.send(PickerEvent::new(placement, vec![BackgroundScaling]));
                }
                // -- painting
                Texturing => {
                    let cmds = update::create_texture_paint_cmds(button, &settings.texture_brush);
                    painting_events.send(PaintingEvent::new(placement, cmds));
                }
                Blending => {
                    let cmds = update::create_blending_paint_cmds(button, &settings.blending_brush);
                    painting_events.send(PaintingEvent::new(placement, cmds));
                }
                Scaling => {
                    let cmds = update::create_scaling_paint_cmds(button, &settings.scaling_brush);
                    painting_events.send(PaintingEvent::new(placement, cmds));
                }
                MaterialParameters => continue,
            }
        }
    }
}
// ----------------------------------------------------------------------------
fn process_picker_results(
    mut ui_state: ResMut<UiState>,
    mut brush: ResMut<BrushPointer>,
    mut picker_results: EventReader<PickerResultEvent>,
    mut rendersettings: ResMut<TerrainRenderSettings>,
) {
    use PickerResult::*;

    let picker_used = !picker_results.is_empty();

    for pick in picker_results.iter() {
        match **pick {
            OverlayTexture(material_slot) => {
                ui_state.toolbox.texture_brush.overlay_texture = material_slot;
                update::update_brush_on_material_selection(
                    &mut ui_state.toolbox,
                    &mut *brush,
                    &mut *rendersettings,
                    true,
                );
            }
            BackgroundTexture(material_slot) => {
                ui_state.toolbox.texture_brush.bkgrnd_texture = material_slot;
                update::update_brush_on_material_selection(
                    &mut ui_state.toolbox,
                    &mut *brush,
                    &mut *rendersettings,
                    false,
                );
            }
            BlendThreshold(value) => {
                update::update_brush_on_blendthreshold_pick(
                    &mut ui_state.toolbox,
                    &mut *brush,
                    value,
                );
            }
            BackgroundScaling(value) => {
                update::update_brush_on_scaling_pick(&mut ui_state.toolbox, &mut *brush, value);
            }
        }
    }

    if picker_used {
        ui_state.toolbox.reset_picker();
    }
}
// ----------------------------------------------------------------------------
fn handle_ui_actions(
    mut ui_state: ResMut<UiState>,
    mut ui_action: EventReader<GuiAction>,
    mut brush: ResMut<BrushPointer>,
    mut materialset: ResMut<TerrainMaterialSet>,
    mut rendersettings: ResMut<TerrainRenderSettings>,
) {
    use ToolboxAction::*;

    for action in ui_action.iter() {
        if let GuiAction::Toolbox(action) = action {
            match action {
                ChangedToolSelection => {
                    update::on_changed_tool_selection(
                        &mut ui_state.toolbox,
                        &mut *brush,
                        &mut *rendersettings,
                    );
                }
                UpdateBrushSettings if !ui_state.toolbox.has_projected_pointer() => {
                    // ignore update if there is no pointer for currently selected tool
                }
                UpdateBrushSettings => {
                    update::update_brush_pointer(&ui_state.toolbox.pointer_settings(), &mut *brush);
                }
                SelectOverlayTexture(material_slot) => {
                    ui_state.toolbox.texture_brush.overlay_texture = *material_slot;
                    update::update_brush_on_material_selection(
                        &mut ui_state.toolbox,
                        &mut *brush,
                        &mut *rendersettings,
                        true,
                    );
                }
                SelectBackgroundTexture(material_slot) => {
                    ui_state.toolbox.texture_brush.bkgrnd_texture = *material_slot;
                    update::update_brush_on_material_selection(
                        &mut ui_state.toolbox,
                        &mut *brush,
                        &mut *rendersettings,
                        false,
                    );
                }
                UpdateMaterial(slot, setting) => {
                    update::update_material_settings(*slot, setting, &mut *materialset);
                }
                ShowOnlyOverlayTexture(only_overlay) => {
                    update::render_only_overlay_material(
                        &mut ui_state.toolbox.texture_brush,
                        &mut *rendersettings,
                        *only_overlay,
                    );
                }
                ShowOnlyBackgroundTexture(only_bkgrnd) => {
                    update::render_only_bkgrnd_material(
                        &mut ui_state.toolbox.texture_brush,
                        &mut *rendersettings,
                        *only_bkgrnd,
                    );
                }
                ShowBackgroundScaling(show) => {
                    update::render_only_bkgrnd_scaling(
                        &mut ui_state.toolbox.scaling_brush,
                        &mut *rendersettings,
                        *show,
                    );
                }
                ShowSlopeBlendThreshold(show) => {
                    update::render_only_slopeblend_threshold(
                        &mut ui_state.toolbox.blending_brush,
                        &mut *rendersettings,
                        *show,
                    );
                }
                TexturePickerSelected(selected) => {
                    ui_state.toolbox.texture_brush.picker_activated = *selected;
                    update::picker_selection(&mut ui_state.toolbox, &mut *brush, *selected);
                }
                SlopeBlendThresholdPickerSelected(selected) => {
                    ui_state.toolbox.blending_brush.picker_activated = *selected;
                    update::picker_selection(&mut ui_state.toolbox, &mut *brush, *selected);
                }
                BkgrndScalingPickerSelected(selected) => {
                    ui_state.toolbox.scaling_brush.picker_activated = *selected;
                    update::picker_selection(&mut ui_state.toolbox, &mut *brush, *selected);
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
    let toolbox = &mut ui_state.toolbox;
    // check if cursor is not over gui or used by gui (slider draging into 3d area)
    if toolbox.enabled && toolbox.has_projected_pointer() && !ui_state.wants_input() {
        let win = windows.get_primary().expect("no primary window");

        brush_pointer.active = true;
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
        }
    } else {
        brush_pointer.active = false;
    }
}
// ----------------------------------------------------------------------------
// toolbox state
// ----------------------------------------------------------------------------
trait ToolSettings {
    fn pointer_color(&self) -> Color;
    fn sync_rendersettings(&mut self, settings: &mut TerrainRenderSettings);
}
// ----------------------------------------------------------------------------
impl ToolboxState {
    // ------------------------------------------------------------------------
    fn has_projected_pointer(&self) -> bool {
        use ToolSelection::*;
        match self.selection {
            Some(MaterialParameters) | None => false,
            Some(Texturing) | Some(Scaling) | Some(Blending) => true,
        }
    }
    // ------------------------------------------------------------------------
    fn rescale_pointer(&mut self, scale: f32) {
        self.brush_size.scale(scale);
    }
    // ------------------------------------------------------------------------
    fn reset_picker(&mut self) {
        use ToolSelection::*;
        match self.selection {
            Some(Texturing) => self.texture_brush.picker_activated = false,
            Some(Blending) => self.blending_brush.picker_activated = false,
            Some(Scaling) => self.scaling_brush.picker_activated = false,
            Some(MaterialParameters) | None => {}
        }
    }
    // ------------------------------------------------------------------------
    fn pointer_settings(&self) -> PointerSettings {
        use ToolSelection::*;
        let color = match self.selection {
            Some(Texturing) => self.texture_brush.pointer_color(),
            Some(Blending) => self.blending_brush.pointer_color(),
            Some(Scaling) => self.scaling_brush.pointer_color(),
            Some(MaterialParameters) | None => {
                // pointer should be deactivated, see has_projected_pointer
                unreachable!("pointer should have been deactivated!")
            }
        };
        PointerSettings {
            size: self.brush_size,
            ring_width: self.brush_size.ring_width(),
            color,
        }
    }
    // ------------------------------------------------------------------------
    fn sync_rendersettings(&mut self, rendersettings: &mut TerrainRenderSettings) {
        use ToolSelection::*;
        match self.selection {
            Some(Texturing) => self.texture_brush.sync_rendersettings(rendersettings),
            Some(Blending) => self.blending_brush.sync_rendersettings(rendersettings),
            Some(Scaling) => self.scaling_brush.sync_rendersettings(rendersettings),
            Some(MaterialParameters) | None => {}
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
