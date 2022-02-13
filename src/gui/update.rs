// ----------------------------------------------------------------------------
use bevy::prelude::*;

use crate::{
    atmosphere::AtmosphereMat,
    SunSettings,
};

use super::{AtmosphereSetting, SunSetting};
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
