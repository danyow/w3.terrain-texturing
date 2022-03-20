// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
pub(super) fn show_sun_settings(
    ui: &mut egui::Ui,
    settings: &Res<SunSettings>,
    gui_event: &mut EventWriter<GuiAction>,
) {
    use GuiAction::*;
    use SunSetting::*;

    egui::CollapsingHeader::new("Sun settings")
        .default_open(false)
        .show(ui, |ui| {
            let mut s = SunGuiSettings {
                time: settings.time_of_day().normalized(),
                cycle_speed: settings.daylight_cycle_speed(),
                yaw: settings.plane_yaw().value(),
                tilt: settings.plane_tilt().value(),
                height: settings.plane_height(),
                show_debug_mesh: settings.show_debug_mesh(),
            };

            if ui.add(Slider::new(&mut s.time, 0.0..=1.0)
                .show_value(false).text(format!("{} Time [HH:mm]", settings.time_of_day().as_str())))
                .changed() {
                    gui_event.send(UpdateSunSetting(SetTimeOfDay(s.time)));
            }
            if ui.add(Slider::new(&mut s.cycle_speed, 0..=100).text("daylight cycle speed")).changed() {
                gui_event.send(UpdateSunSetting(SetCycleSpeed(s.cycle_speed)));
            }
            ui.separator();
            if ui.add(Slider::new(&mut s.tilt, 0..=90).text("sun plane tilt [°]")).changed() {
                gui_event.send(UpdateSunSetting(SetPlaneTilt(s.tilt)));
            }
            if ui.add(Slider::new(&mut s.yaw, 0..=360).text("sun plane yaw [°]")).changed() {
                gui_event.send(UpdateSunSetting(SetPlaneYaw(s.yaw)));
            }
            if ui.add(Slider::new(&mut s.height, 0..=100).text("sun plane height")).changed() {
                gui_event.send(UpdateSunSetting(SetPlaneHeight(s.height)));
            }
            if ui.checkbox(&mut s.show_debug_mesh, "show visualization").changed() {
                gui_event.send(UpdateSunSetting(ToggleDebugMesh));
            }
            ui.separator();
        });
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
#[inline]
pub(super) fn show_atmosphere_settings(
    ui: &mut egui::Ui,
    settings: &Res<AtmosphereMat>,
    gui_event: &mut EventWriter<GuiAction>
) {
    use GuiAction::*;
    use AtmosphereSetting::*;

    egui::CollapsingHeader::new("Atmosphere settings")
        .default_open(false)
        .show(ui, |ui| {
            let mut s = AtmosphereSettings {
                ray_origin: settings.ray_origin(),
                sun_intensity: settings.sun_intensity(),
                planet_radius: settings.planet_radius(),
                atmosphere_radius: settings.atmosphere_radius(),
                rayleigh_scattering: settings.rayleigh_scattering_coefficient(),
                rayleigh_scale_height: settings.rayleigh_scale_height(),
                mie_scattering: settings.mie_scattering_coefficient(),
                mie_scale_height: settings.mie_scale_height(),
                mie_scattering_direction: settings.mie_scattering_direction()
            };

            let mut result = None;

            ui.label("ray origin");
            let big_min = 6360e3;
            let big_max = 6412e3;
            if ui.add(Slider::new(&mut s.ray_origin.x, -100000.0..=100000.0).text("x")).changed() {
                result = Some(SetRayOrigin(s.ray_origin));
            }
            if ui.add(Slider::new(&mut s.ray_origin.y, big_min..=big_max).text("y")).changed() {
                result = Some(SetRayOrigin(s.ray_origin));
            }
            if ui.add(Slider::new(&mut s.ray_origin.z, -10000.0..=10000.0).text("z")).changed() {
                result = Some(SetRayOrigin(s.ray_origin));
            }

            ui.separator();
            if ui.add(Slider::new(&mut s.sun_intensity, 1.0..=100.0).text("sun intensity")).changed() {
                result = Some(SetSunIntensity(s.sun_intensity));
            }
            if ui.add(Slider::new(&mut s.planet_radius, big_min..=big_max).text("planet radius")).changed() {
                result = Some(SetPlanetRadius(s.planet_radius));
            }
            if ui.add(Slider::new(&mut s.atmosphere_radius, big_min..=big_max).text("atmosphere radius")).changed() {
                result = Some(SetAtmosphereRadius(s.atmosphere_radius));
            }

            ui.separator();
            ui.label("Rayleigh scattering");
            if ui.add(Slider::new(&mut s.rayleigh_scattering.x, 5.5e-7..=5.5e-5)).changed() {
                result = Some(SetRayleighScattering(s.rayleigh_scattering));
            }
            if ui.add(Slider::new(&mut s.rayleigh_scattering.y, 13.0e-7..=13.0e-5)).changed() {
                result = Some(SetRayleighScattering(s.rayleigh_scattering));
            }
            if ui.add(Slider::new(&mut s.rayleigh_scattering.z, 22.4e-7..=22.4e-5)).changed() {
                result = Some(SetRayleighScattering(s.rayleigh_scattering));
            }
            if ui.add(Slider::new(&mut s.rayleigh_scale_height, 8e2..=8e4).text("scale height [m]")).changed() {
                result = Some(SetRayleighScaleHeight(s.rayleigh_scale_height));
            }

            ui.separator();
            ui.label("Mie scattering");
            if ui.add(Slider::new(&mut s.mie_scattering, 21e-7..=21e-5).text("coefficient")).changed() {
                result = Some(SetMieScattering(s.mie_scattering));
            }
            if ui.add(Slider::new(&mut s.mie_scale_height, 1.2e2..=1.2e4).text("scale height [m]")).changed() {
                result = Some(SetMieScaleHeight(s.mie_scale_height));
            }
            if ui.add(Slider::new(&mut s.mie_scattering_direction, 0.0..=1.0).text("direction")).changed() {
                result = Some(SetMieScatteringDirection(s.mie_scattering_direction));
            }

            ui.separator();
            if ui.button("reset").clicked() {
                result = Some(ResetToDefault);
            }

            if let Some(action) = result {
                gui_event.send(UpdateAtmosphereSetting(action));
            }
        });
}
// ----------------------------------------------------------------------------
use bevy::math::Vec3;
use bevy::prelude::{EventWriter, Res};
use bevy_egui::egui::{self, Slider};

use crate::atmosphere::AtmosphereMat;
use crate::environment::SunSettings;
use crate::gui::{AtmosphereSetting, SunSetting};
use crate::GuiAction;
// ----------------------------------------------------------------------------
struct AtmosphereSettings {
    ray_origin: Vec3,
    sun_intensity: f32,
    planet_radius: f32,
    atmosphere_radius: f32,
    rayleigh_scattering: Vec3,
    rayleigh_scale_height: f32,
    mie_scattering: f32,
    mie_scale_height: f32,
    mie_scattering_direction: f32,
}
// ----------------------------------------------------------------------------
struct SunGuiSettings {
    time: f32,
    cycle_speed: u16,
    yaw: u16,
    tilt: u16,
    height: u16,
    show_debug_mesh: bool,
}
// ----------------------------------------------------------------------------
