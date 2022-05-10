// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
pub(super) fn show_settings(
    ui: &mut egui::Ui,
    daynight_cycle: &Res<DayNightCycle>,
    gui_event: &mut EventWriter<GuiAction>,
) {
    use GuiAction::*;
    use DayNightCycleSetting::*;

    egui::CollapsingHeader::new("Day / Night cycle")
        .default_open(false)
        .show(ui, |ui| {

            let mut s = DayNightCycleSettings {
                time: daynight_cycle.time_of_day().normalized(),
                cycle_speed: daynight_cycle.cycle_speed(),
            };

            if ui.add(Slider::new(&mut s.time, 0.0..=1.0)
                .show_value(false).text(format!("{} Time [HH:mm]", daynight_cycle.time_of_day().as_str())))
                .changed() {
                    gui_event.send(UpdateDayNightCycleSetting(SetTimeOfDay(s.time)));
            }
            if ui.add(Slider::new(&mut s.cycle_speed, 0..=100).text("cycle speed")).changed() {
                gui_event.send(UpdateDayNightCycleSetting(SetCycleSpeed(s.cycle_speed)));
            }
    });
}
// ----------------------------------------------------------------------------
use bevy::prelude::{EventWriter, Res};
use bevy_egui::egui::{self, Slider};

use crate::environment::DayNightCycle;
use crate::gui::DayNightCycleSetting;

use super::GuiAction;
// ----------------------------------------------------------------------------
struct DayNightCycleSettings {
    time: f32,
    cycle_speed: u16,
}
// ----------------------------------------------------------------------------
