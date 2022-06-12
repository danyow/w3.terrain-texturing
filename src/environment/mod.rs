// ----------------------------------------------------------------------------
use bevy::ecs::schedule::StateData;
use bevy::prelude::*;

use crate::config::TerrainConfig;
use crate::terrain_render::EnvironmentData;

use self::definition::EnvironmentConfig;
use self::settings::EnvironmentSettings;
use self::utils::{Angle, ColorCurveEntry, ScalarCurveEntry, TimeOfDay};
// ----------------------------------------------------------------------------
mod definition;
mod interpolation;
mod settings;
mod sun;
mod utils;
// ----------------------------------------------------------------------------
pub use self::sun::{Sun, SunPositionSettings};
// ----------------------------------------------------------------------------
pub struct EnvironmentPlugin;
// ----------------------------------------------------------------------------
pub struct DayNightCycle {
    time: TimeOfDay,
    cycle_active: bool,
    cycle_speed: u16,
}
// ----------------------------------------------------------------------------
impl EnvironmentPlugin {
    // ------------------------------------------------------------------------
    pub fn startup() -> SystemSet {
        SystemSet::new().with_system(sun::setup_sun)
    }
    // ------------------------------------------------------------------------
    pub fn setup_environment_settings<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_enter(state).with_system(setup_environment_settings)
    }
    // ------------------------------------------------------------------------
    pub fn reset_data<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_enter(state).with_system(reset_environment_settings)
    }
    // ------------------------------------------------------------------------
    pub fn activate_dynamic_updates<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_update(state)
            .with_system(daynight_cycle.label("daynight_cycle"))
            .with_system(
                sun::update_sun_position
                    .label("sun_position_update")
                    .after("daynight_cycle"),
            )
            // Note: update skybox and env.sun direction is in a dedicated system
            // as it is using GlobalTransform. Since sun_position_update is changing
            // Transform only if any settings are changed the GlobalTransfomr
            // does *NOT* reflect that change *yet* (changes to Globaltransform
            // are propagated in bevy in post-update change). This would result
            // in an out of sync sun position / skybox / light direction until
            // settings are changed again.
            // In a dedicated this will be always updated automatically but will
            // have one frame lag (which is ok).
            .with_system(sun::update_skybox)
            .with_system(update_environment_values.after("daynight_cycle"))
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DayNightCycle>()
            .init_resource::<SunPositionSettings>()
            .insert_resource(EnvironmentSettings::from(EnvironmentConfig::default()));
    }
}
// ----------------------------------------------------------------------------
// systems
// ----------------------------------------------------------------------------
fn setup_environment_settings(
    terrain_config: Res<TerrainConfig>,
    mut env_settings: ResMut<EnvironmentSettings>,
) {
    let definition = terrain_config
        .environment_definition()
        .map(|path| definition::EnvironmentConfig::load(path).unwrap_or_default())
        .unwrap_or_default();

    *env_settings = EnvironmentSettings::from(definition);
}
// ----------------------------------------------------------------------------
fn reset_environment_settings(mut env_settings: ResMut<EnvironmentSettings>) {
    *env_settings = EnvironmentSettings::from(EnvironmentConfig::default());
}
// ----------------------------------------------------------------------------
fn update_environment_values(
    day_night_cycle: Res<DayNightCycle>,
    env_settings: Res<EnvironmentSettings>,
    mut env_data: ResMut<EnvironmentData>,
) {
    if day_night_cycle.is_changed() {
        // sample new interpolated values and update current environment data
        env_data.sun.color = env_settings.sun.color.sample(day_night_cycle.time_of_day());
        // fog
        env_data.fog = env_settings.fog.sample(day_night_cycle.time_of_day());
    }
}
// ----------------------------------------------------------------------------
#[allow(clippy::type_complexity)]
fn daynight_cycle(time: Res<Time>, mut daynight_cycle: ResMut<DayNightCycle>) {
    // max speed approx 4sec per daylight
    const DAYNIGHT_SPEED_SCALE: f32 = 1.0 / 100.0 / 4.0;

    // Note: as_ref required to prevent setting "changed" flag
    if daynight_cycle.as_ref().cycle_active() {
        let speed = DAYNIGHT_SPEED_SCALE * daynight_cycle.cycle_speed() as f32;

        let pos = daynight_cycle.time_of_day().normalized() + time.delta_seconds() * speed;
        daynight_cycle.update_time_of_day(pos);
    }
}
// ----------------------------------------------------------------------------
// day night cycle
// ----------------------------------------------------------------------------
impl DayNightCycle {
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn time_of_day(&self) -> &TimeOfDay {
        &self.time
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn cycle_active(&self) -> bool {
        self.cycle_active
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn cycle_speed(&self) -> u16 {
        self.cycle_speed
    }
    // ------------------------------------------------------------------------
    pub fn update_time_of_day(&mut self, time: f32) {
        self.time.update(time);
    }
    // ------------------------------------------------------------------------
    pub fn activate_cycle(&mut self, activate: bool) {
        self.cycle_active = activate;
    }
    // ------------------------------------------------------------------------
    pub fn set_cycle_speed(&mut self, speed: u16) {
        self.cycle_speed = speed.max(0).min(100);
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// default
// ----------------------------------------------------------------------------
impl Default for DayNightCycle {
    fn default() -> Self {
        Self {
            time: TimeOfDay::new(14, 30, 0),
            cycle_active: false,
            cycle_speed: 0,
        }
    }
}
// ----------------------------------------------------------------------------
