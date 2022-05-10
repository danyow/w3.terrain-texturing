//
// gui::update - simple(r) actions for updating state, mapping to other actions
//
// ----------------------------------------------------------------------------
use bevy::prelude::*;

use crate::atmosphere::AtmosphereMat;
use crate::cmds;
use crate::environment::{DayNightCycle, SunPositionSettings};
use crate::terrain_tiles::TerrainMeshSettings;

use super::{AtmosphereSetting, DayNightCycleSetting, MeshSetting, RenderSetting, SunSetting};
// ----------------------------------------------------------------------------
pub(super) fn update_daylight_cycle_settings(
    action: &DayNightCycleSetting,
    daylight_cycle: &mut ResMut<DayNightCycle>,
) {
    use DayNightCycleSetting::*;
    match action {
        SetTimeOfDay(v) => {
            daylight_cycle.update_time_of_day(*v);
            daylight_cycle.set_cycle_speed(0);
        }
        SetCycleSpeed(v) => {
            daylight_cycle.set_cycle_speed(*v);
            daylight_cycle.activate_cycle(*v > 0);
        }
    }
}
// ----------------------------------------------------------------------------
pub(super) fn update_sun_settings(
    action: &SunSetting,
    sun: &mut Option<ResMut<SunPositionSettings>>,
) {
    use SunSetting::*;

    if let Some(sun) = sun {
        match action {
            SetPlaneTilt(v) => sun.set_plane_tilt(*v),
            SetPlaneYaw(v) => sun.set_plane_yaw(*v),
            SetPlaneHeight(v) => sun.set_plane_height(*v),
            ToggleDebugMesh => sun.toggle_debug_mesh(),
        }
    }
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
pub(super) fn update_atmosphere_settings(
    action: &AtmosphereSetting,
    atmosphere: &mut Option<ResMut<AtmosphereMat>>,
) {
    use AtmosphereSetting::*;

    if let Some(atmosphere) = atmosphere {
        match action {
            SetRayOrigin(ray_origin) => atmosphere.set_ray_origin(*ray_origin),
            SetSunIntensity(sun_intensity) => atmosphere.set_sun_intensity(*sun_intensity),
            SetPlanetRadius(planet_radius) => atmosphere.set_planet_radius(*planet_radius),
            SetAtmosphereRadius(atmosphere_radius) => atmosphere.set_atmosphere_radius(*atmosphere_radius),
            SetRayleighScattering(coefficient) => atmosphere.set_rayleigh_scattering_coefficient(*coefficient),
            SetRayleighScaleHeight(scale) => atmosphere.set_rayleigh_scale_height(*scale),
            SetMieScattering(coefficient) => atmosphere.set_mie_scattering_coefficient(*coefficient),
            SetMieScaleHeight(scale) => atmosphere.set_mie_scale_height(*scale),
            SetMieScatteringDirection(direction) => atmosphere.set_mie_scattering_direction(*direction),
            ResetToDefault => **atmosphere = AtmosphereMat::default(),
        }
    }
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
pub(super) fn update_mesh_settings(
    action: &MeshSetting,
    mesh_settings: &mut Option<ResMut<TerrainMeshSettings>>,
) {
    if let Some(mesh) = mesh_settings {
        match action {
            MeshSetting::SetLodCount(count) => mesh.set_lodcount(*count),
            MeshSetting::SetLodMinError(error) => mesh.set_min_error(*error),
            MeshSetting::SetLodMaxError(error) => mesh.set_max_error(*error),
            MeshSetting::SetLodMaxDistance(distance) => mesh.set_max_distance(*distance),
            MeshSetting::SetLodError(slot, error) => mesh.set_lod_error(*slot, *error),
            MeshSetting::SetLodDistance(slot, distance) => mesh.set_lod_distance(*slot, *distance),
            MeshSetting::FreezeLods => mesh.ignore_anchor = !mesh.ignore_anchor,
            MeshSetting::ResetToDefault => **mesh = TerrainMeshSettings::default(),
        }
    }
}
// ----------------------------------------------------------------------------
pub(super) fn update_render_settings(
    action: &RenderSetting,
    task_manager: &mut cmds::AsyncCommandManager,
) {
    match action {
        RenderSetting::OverlayWireframe(_) => {
            task_manager.add_new(cmds::GenerateTerrainMeshes.into());
        }
    }
}
// ----------------------------------------------------------------------------
