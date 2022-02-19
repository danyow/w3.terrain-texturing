// ----------------------------------------------------------------------------
use bevy::prelude::*;

use crate::atmosphere::AtmosphereMat;
use crate::terrain_material::{MaterialSlot, TerrainMaterialSet};
use crate::terrain_tiles::TerrainMeshSettings;
use crate::SunSettings;

use super::{AtmosphereSetting, MaterialSetting, MeshSetting, SunSetting};
// ----------------------------------------------------------------------------
#[inline]
pub(super) fn update_material_settings(
    slot: MaterialSlot,
    action: &MaterialSetting,
    materialset: &mut Option<ResMut<TerrainMaterialSet>>,
) {
    use MaterialSetting::*;

    if let Some(mset) = materialset {
        match action {
            SetBlendSharpness(v) => mset.parameter[slot].blend_sharpness = *v,
            SetSlopeBaseDampening(v) => mset.parameter[slot].slope_base_dampening = *v,
            SetSlopeNormalDampening(v) => mset.parameter[slot].slope_normal_dampening = *v,
            SetSpecularityScale(v) => mset.parameter[slot].specularity_scale = *v,
            SetSpecularity(v) => mset.parameter[slot].specularity = *v,
            SetSpecularityBase(v) => mset.parameter[slot].specularity_base = *v,
            SetFalloff(v) => mset.parameter[slot].falloff = *v,
        }
    }
}
// ----------------------------------------------------------------------------
pub(super) fn update_sun_settings(action: &SunSetting, sun: &mut Option<ResMut<SunSettings>>) {
    use SunSetting::*;

    if let Some(sun) = sun {
        match action {
            SetPosition(v) => sun.pos = *v,
            SetDistance(v) => sun.distance = *v,
            ToggleCycle => sun.cycle_active = !sun.cycle_active,
            SetCycleSpeed(v) => sun.cycle_speed = *v,
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
