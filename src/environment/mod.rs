// ----------------------------------------------------------------------------
use bevy::ecs::schedule::StateData;
use bevy::prelude::*;

pub use self::sun::{Sun, SunSettings};
pub use self::utils::{Angle, TimeOfDay};
// ----------------------------------------------------------------------------
pub struct EnvironmentPlugin;
// ----------------------------------------------------------------------------
impl EnvironmentPlugin {
    // ------------------------------------------------------------------------
    pub fn startup() -> SystemSet {
        SystemSet::new().with_system(sun::setup_sun)
    }
    // ------------------------------------------------------------------------
    pub fn activate_dynamic_updates<T: StateData>(state: T) -> SystemSet {
        SystemSet::on_update(state).with_system(sun::daylight_cycle)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SunSettings>();
    }
}
// ----------------------------------------------------------------------------
mod sun;
mod utils;
// ----------------------------------------------------------------------------
